// Style: phantom-js
// This file initializes the V8 engines and defines the tier hierarchy.
#![allow(clippy::needless_doctest_main)]

pub mod behavior;
pub mod error;
pub mod shims;
pub mod tier1;
pub mod tier2;

pub use behavior::BehaviorEngine;
pub use error::PhantomJsError;

/// Initialise the V8 platform.
///
/// # CRITICAL — CALL THIS IN main() BEFORE TOKIO STARTS.
///
/// V8 must be initialised on the main OS thread before any
/// Tokio worker thread pool is spawned. If called from a Tokio
/// task or worker thread, V8 will crash immediately with a
/// PKU (Memory Protection Keys) memory fault. This crash
/// cannot be caught — it terminates the process.
///
/// Correct usage:
/// ```rust,no_run
/// fn main() {
///     phantom_js::init_v8_platform();  // FIRST
///     let rt = tokio::runtime::Builder::new_multi_thread()
///         .enable_all().build().unwrap();
///     async fn run() {}
///     rt.block_on(async { run().await });
/// }
/// ```
pub fn init_v8_platform() {
    // Use new_unprotected_default_platform — avoids PKU crash
    // when V8 is initialised on a system with Memory Protection Keys.
    // This is the correct choice for server deployments.
    // See: https://github.com/denoland/rusty_v8/issues/1381
    let platform = deno_core::v8::new_unprotected_default_platform(0, false).make_shared();
    deno_core::v8::V8::initialize_platform(platform);
    deno_core::v8::V8::initialize();
    tracing::info!(
        "V8 platform initialised — version: {}",
        deno_core::v8::V8::get_version()
    );
}
