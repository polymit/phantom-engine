use phantom_mcp::telemetry;

#[test]
fn telemetry_init_works() {
    telemetry::init_test();
    tracing::info!("Telemetry is working in test");
}
