#!/usr/bin/env python3
"""Serve one exported simulation on loopback, without external dependencies."""

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sys
from urllib.parse import urlsplit


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "file", nargs="?", type=Path,
        default=root / "runs/simulation-browser-v2/simulation.html",
        help="exported simulation.html (default: saved mixed scenario)",
    )
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    try:
        content = args.file.read_bytes()
    except OSError as error:
        parser.exit(1, f"Cannot read {args.file}: {error}\nGenerate the default run with cargo run --locked -- simulate runs/simulation-browser-v2 mixed 42\nOr pass the path to an existing simulation.html.\n")
    if b'__SIMULATION_DATA__' in content or b'id="simulation-data"' not in content:
        parser.exit(1, "Choose a generated simulation.html, not the viewer template.\n")

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            self.respond(send_body=True)

        def do_HEAD(self):
            self.respond(send_body=False)

        def respond(self, send_body):
            if urlsplit(self.path).path not in ("/", "/simulation.html"):
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(content)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            if send_body:
                self.wfile.write(content)

    try:
        server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    except OSError as error:
        parser.exit(1, f"Cannot start viewer: {error}. Try --port 8766.\n")
    print(f"Simulation: {args.file.resolve()}\nOpen http://127.0.0.1:{server.server_port}/ in your browser.\nKeep this terminal running; Ctrl+C stops the viewer.", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nViewer stopped.")
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
