#!/usr/bin/env python3
"""Local Weaviate ADAM resolved-event history. No cloud endpoint or redirects."""
import json
import sys
import uuid
import urllib.request
import urllib.error

URL = "http://127.0.0.1:8091"
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
        return existing
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
    props = []
    for name, kind in [("description","text"),("candidateJson","text"),("eventRecordJson","text"),("historyScope","text"),("eventId","text"),("dataKind","text"),("resolvedAt","number"),("eventStartedAt","number")]:
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
    query = '{Get{'+CLASS+'(nearText:{concepts:['+json.dumps(query_text)+']},limit:5,where:'+filters+'){candidateJson eventRecordJson historyScope eventId resolvedAt dataKind _additional{id distance}}}}'
    result = request("POST","/v1/graphql",{"query":query},min(5,remaining_ms/1000))
    if result.get("errors"):
        raise ValueError("History query failed: " + json.dumps(result["errors"])[:600])
    records = result["data"]["Get"][CLASS]
    if len(records)>5:
        raise ValueError("History service exceeded top-five limit")
    candidates=[]
    for record in records:
        candidate=json.loads(record["candidateJson"])
        if record["resolvedAt"]>=start or record["eventId"]==context["event_id"] or record["dataKind"]!=kind or record["historyScope"]!=context["history_scope"]:
            raise ValueError("Current/future/cross-modality history was returned")
        candidates.append(candidate)
    return candidates,{"backend":"weaviate-1.30.2/text2vec-transformers/all-MiniLM-L6-v2","query":query_text,"where":where,"records":records}

def record(data):
    r, d = data["record"], data["disclosure"]
    p, stage = r["proposal"]["payload"], d["stage"]
    if p["stage_status"]!="approved_awaiting_recording" or not p["governance_accepted"] or not stage["final_classification"]:
        raise ValueError("Only verified resolved policy-approved events enter semantic history")
    primary=next(v for v in d["event"]["readings"] if v["source_id"]==d["event"]["trigger_source"])
    kind=primary["data_kind"]
    final=stage["final_decision"]
    description=(f"CO2 concentration {stage['fusion']['fused_ppm']:.6f} ppm. "
                 f"Trigger {primary['co2_ppm']} ppm. "
                 f"Dispersion {stage['fusion']['dispersion_ppm']:.6f} ppm. "
                 f"Classification {stage['final_classification']}, severity {final['severity']}. " + final["reasoning"])
    properties={"description":description,"historyScope":p["registry_hash"],"eventRecordJson":json.dumps({"trigger":primary,"fusion":stage["fusion"],"selected_recommendation":stage["selected_recommendation"],"local_recommendation":stage["local_recommendation"],"crew_support":stage["crew_support"],"final_decision":final,"resolved_at_ms":data["resolved_at_ms"]}),"candidateJson":json.dumps(stage["selected_recommendation"]),"eventId":p["context"]["event_id"],"dataKind":kind,"resolvedAt":data["resolved_at_ms"],"eventStartedAt":p["context"]["started_at_ms"]}
    identifier=str(uuid.uuid5(uuid.NAMESPACE_URL,"verifiable-adam/"+p["registry_hash"]+"/"+p["context"]["session_id"]+"/"+p["context"]["event_id"]))
    result=request("POST","/v1/objects",{"class":CLASS,"id":identifier,"properties":properties},max(0.001,min(5,data["remaining_ms"]/1000)))
    return {"id":result["id"],"class":CLASS,"resolved_at_ms":data["resolved_at_ms"],"recording_acknowledged":True}

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
