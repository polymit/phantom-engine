// Blueprint Section 10 — MINIMUM targets as k6 pass/fail thresholds.
// k6 exits non-zero when any threshold fails.
export const PHANTOM_THRESHOLDS = {
  // MCP navigate latency: minimum <5s, goal <2s
  "http_req_duration{tool:browser_navigate}": ["p(95)<5000", "p(50)<2000"],
  // MCP scene graph latency: minimum <100ms, goal <20ms
  "http_req_duration{tool:browser_get_scene_graph}": ["p(95)<100", "p(50)<20"],
  // MCP click latency: minimum <500ms, goal <100ms
  "http_req_duration{tool:browser_click}": ["p(95)<500", "p(50)<100"],
  // Error rate: must be <1% under normal load.
  http_req_failed: ["rate<0.01"],
};
