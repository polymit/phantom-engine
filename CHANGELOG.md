# Release Notes — v0.1.2-alpha

This release transitions the Phantom Engine from hardcoded resource limits to a configurable, production-ready orchestration model. We've focused on eliminating systemic bottlenecks that previously restricted high-concurrency browser automation and complex JavaScript execution.

## Core Infrastructure

### Resource Budgeting and Scaling
- **Configurable CPU Quotas**: Replaced the hardcoded 1000ms per-second CPU limit with `PHANTOM_CPU_QUOTA_MS`. This allows the engine to handle heavy layout calculations and long-running JS execution without triggering budget-exceeded errors.
- **Dynamic Tier 1 Pools**: Introduced `PHANTOM_QUICKJS_POOL_SIZE`. The execution pool is no longer locked at 5 sessions; it can now be scaled up based on available system memory to support high-concurrency workloads.
- **Native Environment Loading**: Integrated `dotenvy` directly into the MCP entry point. The server now automatically bootstraps its environment from `.env` files in the working directory.

## Tooling and Developer Experience

### Interactive CLI Bootstrap
- **ph setup init**: Rewrote the setup command as an interactive wizard. It now prompts for API keys, session limits, and pool sizes while maintaining secure defaults for internal encryption keys.
- **Enhanced Diagnostics**: Updated `ph setup doctor` to verify system-level requirements, including storage permissions and environment variable integrity.

## Stability and Bug Fixes

- **Session Lifecycle**: Resolved a systemic race condition where sessions would terminate prematurely on high-latency pages.
- **Configuration Propagation**: Fixed an issue where engine parameters were not correctly synchronized from the MCP entry point to the session adapter.
- **Workspace Alignment**: Synchronized all 10 project crates to version `0.1.2-alpha` to ensure dependency compatibility across the workspace.

---

**Upgrade Note**: New configuration variables (`PHANTOM_CPU_QUOTA_MS`, `PHANTOM_QUICKJS_POOL_SIZE`) are required. Run `ph setup init` to generate a fresh `.env` or update your existing configuration manually.
