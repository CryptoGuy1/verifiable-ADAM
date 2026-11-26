#!/usr/bin/env python3
"""Local Weaviate ADAM resolved-event history. No cloud endpoint or redirects."""
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid
import urllib.request
import urllib.error
import urllib.parse

URL = os.environ.get("VERI_ADAM_HISTORY_URL", "http://127.0.0.1:8091").rstrip("/")
_endpoint = urllib.parse.urlsplit(URL)
if _endpoint.scheme != "http" or _endpoint.hostname not in {"127.0.0.1", "localhost", "::1"} or _endpoint.username or _endpoint.password or _endpoint.path or _endpoint.query or _endpoint.fragment:
    raise ValueError("History service must be an explicit local HTTP endpoint")
CLASS = "VerifiableAdamEventTrace"

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        raise ValueError("History-service redirect refused")

def request(method, path, data=None, timeout=5):
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    encoded = None if data is None else json.dumps(data).encode()
    req = urllib.request.Request(URL + path, data=encoded, method=method, headers={"Content-Type":"application/json"})
    with opener.open(req, timeout=max(0.001, timeout)) as response:
        body = response.read(1048577)
    if len(body)>1048576:
        raise ValueError("History response exceeds 1 MiB")
    return json.loads(body) if body else {}

def initialize():
    try:
        existing = request("GET", "/v1/schema/" + CLASS)
        if existing.get("vectorizer") != "text2vec-transformers":
            raise ValueError("Existing history class has a different vectorizer")
        names = {p["name"] for p in existing.get("properties", [])}
        if "verificationJson" not in names:
            request("POST", "/v1/schema/" + CLASS + "/properties", {"name":"verificationJson","dataType":["text"],"tokenization":"field","moduleConfig":{"text2vec-transformers":{"skip":True,"vectorizePropertyName":False}}})
            return request("GET", "/v1/schema/" + CLASS)
        return existing
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
    props = []
    for name, kind in [("description","text"),("candidateJson","text"),("verificationJson","text"),("eventRecordJson","text"),("historyScope","text"),("eventId","text"),("dataKind","text"),("resolvedAt","number"),("eventStartedAt","number")]:
        props.append({"name":name,"dataType":[kind],**({"tokenization":"field"} if kind=="text" and name!="description" else {}),"moduleConfig":{"text2vec-transformers":{"skip":name!="description","vectorizePropertyName":False}}})
    return request("POST","/v1/schema",{"class":CLASS,"vectorizer":"text2vec-transformers","moduleConfig":{"text2vec-transformers":{"vectorizeClassName":False}},"properties":props})

def retrieve(context, remaining_ms):
    # Experiment streams stay separate; current/future events are filtered by
    # actual resolution time, not merely their capture timestamps.
    start = context["primary"]["captured_at_ms"]
    # The Rust event context uses an event start >= every captured observation.
    # Caller supplies the actual event start explicitly below when available.
    start = context.get("event_started_at_ms", start)
    kind = context["primary"]["data_kind"]
    query_text = f"CO2 concentration {context['fusion']['fused_ppm']:.6f} ppm"
    where = {"operator":"And","operands":[
        {"path":["resolvedAt"],"operator":"LessThan","valueNumber":start},
        {"path":["eventId"],"operator":"NotEqual","valueText":context["event_id"]},
        {"path":["dataKind"],"operator":"Equal","valueText":kind},
        {"path":["historyScope"],"operator":"Equal","valueText":context["history_scope"]},
    ]}
    # JSON literals also encode these GraphQL string/number inputs safely.
    filters = ('{operator:And,operands:[' +
        '{path:["resolvedAt"],operator:LessThan,valueNumber:'+str(start)+'},'+
        '{path:["eventId"],operator:NotEqual,valueText:'+json.dumps(context["event_id"])+'},'+
        '{path:["dataKind"],operator:Equal,valueText:'+json.dumps(kind)+'},'+
        '{path:["historyScope"],operator:Equal,valueText:'+json.dumps(context["history_scope"])+"}]}" )
    query = '{Get{'+CLASS+'(nearText:{concepts:['+json.dumps(query_text)+']},limit:5,where:'+filters+'){candidateJson eventRecordJson verificationJson historyScope eventId resolvedAt dataKind _additional{id distance}}}}'
    result = request("POST","/v1/graphql",{"query":query},min(5,remaining_ms/1000))
    if result.get("errors"):
        raise ValueError("History query failed: " + json.dumps(result["errors"])[:600])
    records = result["data"]["Get"][CLASS]
    if len(records)>5:
        raise ValueError("History service exceeded top-five limit")
    candidates=[]
    verified_records=[]
    seen=set()
    for record in records:
        if record["resolvedAt"]>=start or record["eventId"]==context["event_id"] or record["dataKind"]!=kind or record["historyScope"]!=context["history_scope"]:
            raise ValueError("Current/future/cross-modality history was returned")
        view=verify_event(json.loads(record["verificationJson"]),context["history_scope"],start,remaining_ms)
        if view["proposal_id"] in seen:
            raise ValueError("Duplicate authenticated historical decision")
        seen.add(view["proposal_id"])
        if view["event_id"]!=record["eventId"] or view["resolved_at_ms"]!=record["resolvedAt"] or view["data_kind"]!=kind or view["candidate"]!=json.loads(record["candidateJson"]) or view["event_record"]!=json.loads(record["eventRecordJson"]):
            raise ValueError("Retrieved summary differs from authenticated historical evidence")
        candidates.append(view["candidate"])
        # Full historical verification envelopes never enter model text/metadata:
        # that would recursively duplicate prior traces without a bound.
        verified_records.append({k:v for k,v in record.items() if k!="verificationJson"})
        verified_records[-1]["authenticated_proposal_id"]=view["proposal_id"]
    return candidates,{"backend":"weaviate-1.30.2/text2vec-transformers/all-MiniLM-L6-v2","query":query_text,"where":where,"records":verified_records,"independent_history_verification":True}

def verify_event(verification,scope,before,remaining_ms):
    verifier=os.environ.get("VERI_ADAM_VERIFIER")
    if verifier is None:
        verifier=str(Path(__file__).resolve().parents[1]/"target"/"debug"/"veri-adam")
    raw=json.dumps(verification,separators=(",", ":")).encode()
    if len(raw)>1048576:
        raise ValueError("Historical verification envelope exceeds 1 MiB")
    result=subprocess.run([verifier,"adam-history-check",scope,str(before)],input=raw,capture_output=True,timeout=max(0.001,min(5,remaining_ms/1000)))
    if result.returncode!=0:
        raise ValueError("Historical independent verification failed: "+result.stderr.decode(errors="replace")[:500])
    if len(result.stdout)>65536:
        raise ValueError("Verified history summary exceeds bounds")
    return json.loads(result.stdout)

def record(data):
    verification=data["verification"]
    r=verification["record"]
    p=r["proposal"]["payload"]
    resolved=verification["resolution"]["entry"]["event"]["at_ms"]
    view=verify_event(verification,p["registry_hash"],resolved+1,data["remaining_ms"])
    event=view["event_record"]
    fusion=event["fusion"]
    final=event["final_decision"]
    description=(f"CO2 concentration {fusion['fused_ppm']:.6f} ppm. "
                 f"Trigger {event['trigger']['co2_ppm']} ppm. "
                 f"Dispersion {fusion['dispersion_ppm']:.6f} ppm. "
                 f"Classification {final['classification']}, severity {final['severity']}. " + final["reasoning"])
    properties={"description":description,"historyScope":p["registry_hash"],"verificationJson":json.dumps(verification,separators=(",", ":")),"eventRecordJson":json.dumps(event),"candidateJson":json.dumps(view["candidate"]),"eventId":view["event_id"],"dataKind":view["data_kind"],"resolvedAt":resolved,"eventStartedAt":p["context"]["started_at_ms"]}
    identifier=str(uuid.uuid5(uuid.NAMESPACE_URL,"verifiable-adam/"+p["registry_hash"]+"/"+p["context"]["session_id"]+"/"+p["context"]["event_id"]))
    result=request("POST","/v1/objects",{"class":CLASS,"id":identifier,"properties":properties},max(0.001,min(5,data["remaining_ms"]/1000)))
    if result.get("id")!=identifier:
        raise ValueError("History store did not acknowledge the submitted event")
    return {"id":identifier,"class":CLASS,"resolved_at_ms":resolved,"recording_acknowledged":True,"independent_history_verification":True}

if __name__=="__main__":
    try:
        if sys.argv[1:]==["init"]:
            result=initialize()
        elif sys.argv[1:]==["record"]:
            raw=sys.stdin.buffer.read(1048577)
            if len(raw)>1048576:
                raise ValueError("History record input exceeds 1 MiB")
            result=record(json.loads(raw))
        else:
            raise ValueError("Usage: history_store.py init|record")
        print(json.dumps(result))
    except Exception as error:
        print(str(error),file=sys.stderr)
        sys.exit(1)
