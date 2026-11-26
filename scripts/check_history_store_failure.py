#!/usr/bin/env python3
"""Live local failure test: real retrieval/model, deliberate write refusal.
The proxy and its environment override exist only for this test subprocess.
"""
import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import urllib.request
import urllib.error

class Proxy(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        self.forward()

    def do_POST(self):
        self.forward()

    def forward(self):
        size=int(self.headers.get("Content-Length",0))
        if size>1048576:
            self.send_error(413)
            return
        body=self.rfile.read(size) if size else None
        if self.command=="POST" and self.path=="/v1/objects":
            self.send_error(503,"Deliberate required-store write refusal")
            return
        if not (self.path.startswith("/v1/schema/") or self.path=="/v1/graphql"):
            self.send_error(404)
            return
        req=urllib.request.Request("http://127.0.0.1:8091"+self.path,data=body,method=self.command,headers={"Content-Type":"application/json"})
        opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
        try:
            with opener.open(req,timeout=5) as response:
                data=response.read(1048577)
                status=response.status
        except urllib.error.HTTPError as error:
            data=error.read(1048577)
            status=error.code
        self.send_response(status)
        self.send_header("Content-Type","application/json")
        self.send_header("Content-Length",str(len(data)))
        self.end_headers()
        self.wfile.write(data)

def main():
    if len(sys.argv)!=2:
        raise ValueError("Usage: check_history_store_failure.py NEW_RUN_DIRECTORY")
    root=Path(__file__).resolve().parents[1]
    target=Path(sys.argv[1]).resolve()
    if target.exists():
        raise ValueError("Use a new output directory")
    server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Proxy)
    worker=threading.Thread(target=server.serve_forever,daemon=True)
    worker.start()
    env=dict(os.environ,VERI_ADAM_HISTORY_URL=f"http://127.0.0.1:{server.server_port}")
    try:
        result=subprocess.run([str(root/"target/debug/veri-adam"),"adam-agent-run",str(target),"ollama-history"],env=env,timeout=240)
        if result.returncode:
            raise ValueError("Fault run did not complete")
    finally:
        server.shutdown()
        server.server_close()
        worker.join()
    rounds=json.loads((target/"agent-run.json").read_text())
    for round in rounds[:4]:
        case=target/round["event"]["event_id"]
        if round["verification"]["outcome"]!="verified_current_awaiting_recording" or not round["private_verification"]["accepted"]:
            raise ValueError("Failure test did not reach an otherwise valid decision")
        if not (case/"history-recording-failed.json").exists() or (case/"advisory-authorization.json").exists():
            raise ValueError("Missing required-store acknowledgment did not withhold authorization")
    entries=[json.loads(line)["payload"] for line in (target/"audit/events.jsonl").read_text().splitlines()]
    failures=[e for e in entries if e["event"]["kind"]=="recording_failure"]
    if len(failures)!=4:
        raise ValueError("Required-store failures were not all recorded durably")
    report={"fault":"required history-store writes return HTTP 503","real_qwen":True,"otherwise_valid_decisions":4,"advisory_authorizations":0,"durable_recording_failures":len(failures),"passed":True}
    (target/"required-store-failure-test.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report))

if __name__=="__main__":
    main()
