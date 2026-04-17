import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";
import { PHANTOM_THRESHOLDS } from "./thresholds.js";

const BASE_URL = __ENV.PHANTOM_BASE_URL || "http://localhost:8080";
const API_KEY = __ENV.PHANTOM_API_KEY || "phantom-ci-key";

const navigateDuration = new Trend("navigate_duration", true);
const navigateErrors = new Rate("navigate_errors");

export const options = {
  stages: [
    { duration: "30s", target: 50 },
    { duration: "60s", target: 100 },
    { duration: "30s", target: 0 },
  ],
  thresholds: PHANTOM_THRESHOLDS,
};

function rpcRequest(method, params, id, tags) {
  return http.post(
    `${BASE_URL}/rpc`,
    JSON.stringify({
      jsonrpc: "2.0",
      id,
      method,
      params,
    }),
    {
      headers: {
        "Content-Type": "application/json",
        "X-API-Key": API_KEY,
      },
      tags,
    }
  );
}

function parseBody(response) {
  try {
    return JSON.parse(response.body);
  } catch (_err) {
    return { error: { message: "invalid json response" } };
  }
}

export default function () {
  const html = "<html><body><h1>Load Test</h1><button id='btn'>Go</button></body></html>";
  const url = `data:text/html,${encodeURIComponent(html)}`;
  const startedAt = Date.now();

  const response = rpcRequest(
    "browser_navigate",
    { url },
    1,
    { tool: "browser_navigate" }
  );
  const elapsedMs = Date.now() - startedAt;
  navigateDuration.add(elapsedMs);

  const body = parseBody(response);
  const ok = check(response, {
    "navigate: status 200": (res) => res.status === 200,
    "navigate: no json-rpc error": () => !body.error,
    "navigate: p-call under 5s": () => elapsedMs < 5000,
  });
  navigateErrors.add(!ok);

  sleep(0.1);
}
