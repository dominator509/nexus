// Compile-time RX-008 guard: the composition root and telemetry must
// satisfy axum State extractor bounds (Clone + Send + Sync + 'static).
#[test]
fn ep044_rx008_composition_and_telemetry_are_send_sync() {
    fn assert_bounds<T: Clone + Send + Sync + 'static>() {}
    assert_bounds::<nexus_control_plane::composition::RuntimeComposition>();
    assert_bounds::<nexus_control_plane::telemetry::RuntimeTelemetry>();
}
