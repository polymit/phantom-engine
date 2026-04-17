import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";
import { PHANTOM_THRESHOLDS } from "./thresholds.js";

const BASE_URL = __ENV.PHANTOM_BASE_URL || "http://localhost:8080";
const API_KEY = __ENV.PHANTOM_API_KEY || "phantom-ci-key";

const mixedErrors = new Rate("mixed_errors");
const mixedNavigate = new Trend("mixed_navigate_duration", true);
const mixedSceneGraph = new Trend("mixed_scene_graph_duration", true);
const mixedClick = new Trend("mixed_click_duration", true);

export const options = {
  vus: 100,
  duration: "120s",
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
  const html =
    "<html><body><h1>Mixed Flow</h1><button id='mixed-btn'>Run</button></body></html>";
  const url = `data:text/html,${encodeURIComponent(html)}`;

  const navStart = Date.now();
  const nav = rpcRequest(
    "browser_navigate",
    { url },
    1,
    { tool: "browser_navigate" }
  );
  const navElapsed = Date.now() - navStart;
  mixedNavigate.add(navElapsed);
  const navBody = parseBody(nav);

  const graphStart = Date.now();
  const graph1 = rpcRequest(
    "browser_get_scene_graph",
    { mode: "full" },
    2,
    { tool: "browser_get_scene_graph" }
  );
  const graphElapsed = Date.now() - graphStart;
  mixedSceneGraph.add(graphElapsed);
  const graph1Body = parseBody(graph1);
  const cct = graph1Body?.result?.cct || "";

  const clickStart = Date.now();
  const click = rpcRequest(
    "browser_click",
    { selector: "#mixed-btn" },
    3,
    { tool: "browser_click" }
  );
  const clickElapsed = Date.now() - clickStart;
  mixedClick.add(clickElapsed);
  const clickBody = parseBody(click);

  const graph2 = rpcRequest(
    "browser_get_scene_graph",
    { mode: "full" },
    4,
    { tool: "browser_get_scene_graph" }
  );
  const graph2Body = parseBody(graph2);

  const ok = check(nav, {
    "mixed: navigate status 200": (res) => res.status === 200,
    "mixed: navigate no error": () => !navBody.error,
    "mixed: navigate under 5s minimum": () => navElapsed < 5000,
  }) &&
    check(graph1, {
      "mixed: scene_graph status 200": (res) => res.status === 200,
      "mixed: scene_graph no error": () => !graph1Body.error,
      "mixed: scene_graph cct header": () =>
        typeof cct === "string" && cct.startsWith("##PAGE"),
      "mixed: scene_graph under 100ms minimum": () => graphElapsed < 100,
    }) &&
    check(click, {
      "mixed: click status 200": (res) => res.status === 200,
      "mixed: click no error": () => !clickBody.error,
      "mixed: click acknowledged": () => clickBody?.result?.clicked === true,
      "mixed: click under 500ms minimum": () => clickElapsed < 500,
    }) &&
    check(graph2, {
      "mixed: post-click scene_graph status 200": (res) => res.status === 200,
      "mixed: post-click scene_graph no error": () => !graph2Body.error,
    });

  mixedErrors.add(!ok);
  sleep(0.1);
}
