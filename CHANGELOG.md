# Release Notes — v0.2.1-alpha

This patch fixes three silent configuration bugs where environment variables defined in `.env` were ignored by the engine at runtime. Users who had customized `PHANTOM_STORAGE_DIR`, `PHANTOM_LOG_FORMAT`, or `PHANTOM_V8_POOL_SIZE` were unknowingly running on hardcoded defaults.

## Bug Fixes

### PHANTOM_STORAGE_DIR was ignored
The `SessionStorageManager` in `engine.rs` was initialized with a hardcoded `"./storage"` path. The `PHANTOM_STORAGE_DIR` variable defined in `.env` was never read. A new `EngineAdapter::new_with_storage()` constructor now accepts and propagates the configured path. The original `new()` method delegates to it with the default, preserving backward compatibility.

### PHANTOM_LOG_FORMAT was silently mismatched
The telemetry module read `LOG_FORMAT` from the environment, but the `.env` file and `ph setup init` both write `PHANTOM_LOG_FORMAT`. This naming mismatch caused the log format to always fall back to `compact`, regardless of what was configured. The variable name in `telemetry.rs` now matches the documented convention.

### PHANTOM_V8_POOL_SIZE was hardcoded
The V8 pool size was passed as a literal `5` in the `EngineAdapter::new()` call inside `phantom.rs`. The `PHANTOM_V8_POOL_SIZE` environment variable was defined in `.env` and written by `ph setup init`, but never consumed. It is now read and parsed alongside `PHANTOM_QUICKJS_POOL_SIZE`.

## Affected Files

- `phantom-mcp/src/bin/phantom.rs` — reads `PHANTOM_V8_POOL_SIZE` and `PHANTOM_STORAGE_DIR`
- `phantom-mcp/src/engine.rs` — new `new_with_storage()` constructor
- `phantom-mcp/src/telemetry.rs` — corrected env var name

## Upgrade Notes

No action required. If you were relying on the hardcoded defaults, your existing `.env` values will now take effect. Verify your `PHANTOM_STORAGE_DIR` path exists before starting the engine.
