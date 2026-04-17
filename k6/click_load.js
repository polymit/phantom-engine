import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";
import { PHANTOM_THRESHOLDS } from "./thresholds.js";

const BASE_URL = __ENV.PHANTOM_BASE_URL || "http://localhost:8080";
const API_KEY = __ENV.PHANTOM_API_KEY || "phantom-ci-key";

const clickDuration = new Trend("click_duration", true);
const clickErrors = new Rate("click_errors");

export const options = {
  stages: [
    { duration: "30s", target: 40 },
    { duration: "60s", target: 80 },
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
  const html =
    "<html><body><button id='load-btn' style='width:120px;height:40px;'>Click me</button></body></html>";
  const url = `data:text/html,${encodeURIComponent(html)}`;
  const nav = rpcRequest(
    "browser_navigate",
    { url },
    1,
    { tool: "browser_navigate" }
  );
  const navBody = parseBody(nav);
  if (nav.status !== 200 || navBody.error) {
    clickErrors.add(true);
    sleep(0.1);
    return;
  }

  const startedAt = Date.now();
  const click = rpcRequest(
    "browser_click",
    { selector: "#load-btn" },
    2,
    { tool: "browser_click" }
  );
  const elapsedMs = Date.now() - startedAt;
  clickDuration.add(elapsedMs);

  const body = parseBody(click);
  const clicked = body?.result?.clicked === true;
  const ok = check(click, {
    "click: status 200": (res) => res.status === 200,
    "click: no json-rpc error": () => !body.error,
    "click: acknowledged": () => clicked,
    "click: under 500ms minimum": () => elapsedMs < 500,
  });
  clickErrors.add(!ok);

  sleep(0.1);
}
