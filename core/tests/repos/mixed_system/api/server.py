import json
from http.server import BaseHTTPRequestHandler

# vantage:
#   invariant: Stateless
#   reason: Each request must be independently processable
class ArenaHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"items": ["a", "b", "c"]}).encode())
