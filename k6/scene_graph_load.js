import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";
import { PHANTOM_THRESHOLDS } from "./thresholds.js";

const BASE_URL = __ENV.PHANTOM_BASE_URL || "http://localhost:8080";
const API_KEY = __ENV.PHANTOM_API_KEY || "phantom-ci-key";

const sceneGraphDuration = new Trend("scene_graph_duration", true);
const sceneGraphErrors = new Rate("scene_graph_errors");

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
  const html = "<html><body><main><h1>Scene Graph</h1><p>k6 node</p></main></body></html>";
  const url = `data:text/html,${encodeURIComponent(html)}`;
  const nav = rpcRequest(
    "browser_navigate",
    { url },
    1,
    { tool: "browser_navigate" }
  );
  const navBody = parseBody(nav);
  if (nav.status !== 200 || navBody.error) {
    sceneGraphErrors.add(true);
    sleep(0.1);
    return;
  }

  const startedAt = Date.now();
  const sceneGraph = rpcRequest(
    "browser_get_scene_graph",
    { mode: "full" },
    2,
    { tool: "browser_get_scene_graph" }
  );
  const elapsedMs = Date.now() - startedAt;
  sceneGraphDuration.add(elapsedMs);

  const body = parseBody(sceneGraph);
  const cct = body?.result?.cct || "";
  const ok = check(sceneGraph, {
    "scene_graph: status 200": (res) => res.status === 200,
    "scene_graph: no json-rpc error": () => !body.error,
    "scene_graph: cct starts with ##PAGE": () =>
      typeof cct === "string" && cct.startsWith("##PAGE"),
    "scene_graph: under 100ms minimum": () => elapsedMs < 100,
  });
  sceneGraphErrors.add(!ok);

  sleep(0.1);
}
