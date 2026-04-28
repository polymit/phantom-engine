"""
Phantom Engine Fuzzing Orchestrator (PEFO)
Version: 0.1.1
Description: Automates the execution of generated fuzzing plans against the Phantom MCP server.
"""

import json
import os
import sys
import time
import urllib.request
import tempfile
import random
import string
import threading
import http.server
import socketserver
from pathlib import Path

# --- Configuration Constants ---
RPC_ENDPOINT = "http://127.0.0.1:8080/rpc"
PAYLOAD_SERVER_PORT = 9000
HEALTH_CHECK_DELAY = 0.5
REQUEST_TIMEOUT_SEC = 60

class FuzzOrchestrator:
    def __init__(self, manifest_path):
        self.manifest_path = Path(manifest_path)
        self.plan_dir = self.manifest_path.parent
        self.workspace_dir = Path(tempfile.mkdtemp(prefix="phantom-fuzz-work-"))
        self.api_key = self._initialize_auth()
        self.active_session_id = None
        self.cases_survived = 0

    def _initialize_auth(self):
        """Initializes the PHANTOM_API_KEY from environment or .env file."""
        key = os.getenv("PHANTOM_API_KEY")
        if key:
            return key
        try:
            env_path = Path(".env")
            if env_path.exists():
                with open(env_path, "r") as f:
                    for line in f:
                        if line.startswith("PHANTOM_API_KEY="):
                            return line.split("=", 1)[1].strip()
        except Exception:
            pass
        return "default_key"

    def execute_rpc(self, method, params):
        """Dispatches a JSON-RPC 2.0 request to the target engine."""
        payload = {
            "jsonrpc": "2.0",
            "id": int(time.time() * 1000),
            "method": method,
            "params": params
        }
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            RPC_ENDPOINT,
            data=data,
            headers={"Content-Type": "application/json", "x-api-key": self.api_key}
        )
        try:
            with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT_SEC) as response:
                body = json.loads(response.read().decode("utf-8"))
                self._update_session_state(body)
                return body
        except Exception as e:
            return {"error": str(e)}

    def _update_session_state(self, response_body):
        """Tracks the active session/tab ID from RPC responses."""
        result = response_body.get("result")
        if isinstance(result, dict):
            if "session_id" in result:
                self.active_session_id = result["session_id"]
            elif "id" in result:
                self.active_session_id = result["id"]

    def _start_payload_server(self):
        """Initializes background HTTP server for payload delivery."""
        class LoglessHandler(http.server.SimpleHTTPRequestHandler):
            def log_message(self, format, *args): pass

        os.chdir(self.workspace_dir)
        server = socketserver.TCPServer(("", PAYLOAD_SERVER_PORT), LoglessHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()

    def _resolve_placeholders(self, params, blob_size=None):
        """Resolves dynamic placeholders within the RPC parameter set."""
        if not isinstance(params, dict):
            return params
        
        resolved = {}
        for key, value in params.items():
            if value == "{{active_tab_id}}":
                resolved[key] = self.active_session_id
            elif value == "$blob" and blob_size:
                resolved[key] = ''.join(random.choices(string.ascii_letters + string.digits, k=blob_size))
            else:
                resolved[key] = value
        return resolved

    def run(self):
        """Main execution loop for the fuzzing plan."""
        print(f"[INFO] Initializing Orchestration")
        print(f"[INFO] Source Manifest: {self.manifest_path}")
        
        try:
            with open(self.manifest_path, 'r') as f:
                manifest = json.load(f)
        except Exception as e:
            print(f"[FATAL] Failed to load manifest: {e}")
            sys.exit(1)

        self._start_payload_server()
        print(f"[INFO] Payload Server: http://127.0.0.1:{PAYLOAD_SERVER_PORT}")

        for idx, case in enumerate(manifest["cases"]):
            print(f"\n[CASE {idx}] Type: {case['kind']}")
            
            # 1. Stage Payload
            filename = f"payload_{idx}.html"
            with open(self.workspace_dir / filename, 'w', encoding='utf-8') as f:
                f.write(case["doc"])
            
            # 2. Trigger Navigation
            target_url = f"http://127.0.0.1:{PAYLOAD_SERVER_PORT}/{filename}"
            print(f"  [>] Dispatched browser_navigate: {target_url}")
            self.execute_rpc("browser_navigate", {"url": target_url})
            
            # 3. Execute Atomic Sequences (Storm)
            if case.get("storm") and case["storm"].get("calls"):
                print(f"  [>] Executing Atomic Sequence ({len(case['storm']['calls'])} calls)")
                for call in case["storm"]["calls"]:
                    time.sleep(call.get("delay_ms", 0) / 1000.0)
                    params = self._resolve_placeholders(call["params"], call.get("blob_bytes"))
                    self.execute_rpc(call["method"], params)

            # 4. Target Health Verification
            time.sleep(HEALTH_CHECK_DELAY)
            health = self.execute_rpc("ping", {})
            if "error" in health:
                print(f"\n[CRITICAL] TARGET UNRESPONSIVE AT CASE {idx}")
                print(f"[CRITICAL] Error Details: {health['error']}")
                print(f"[CRITICAL] Trigger Payload: {self.workspace_dir / filename}")
                sys.exit(1)
            
            print(f"  [+] Case {idx} validation successful.")
            self.cases_survived += 1

        print(f"\n[SUCCESS] Execution complete. Target survived {self.cases_survived} test cases.")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 fuzz-run.py <path_to_manifest.json>")
        sys.exit(1)
    
    orchestrator = FuzzOrchestrator(sys.argv[1])
    orchestrator.run()
