//! End-to-end test for the dora-test-utils harness.
//!
//! Exercises the full input pipeline: send_input → tick → receive events.

use dora_node_api::integration_testing::integration_testing_format::{
    IncomingEvent, InputData, TimedIncomingEvent,
};
use dora_test_utils::NodeHarness;

/// Full input pipeline: inject Input + Stop, then tick through both events.
///
/// Pipeline:
/// 1. Create harness
/// 2. Inject Input event with integer data, then Stop
/// 3. Tick — receive Input, verify id and data present
/// 4. Tick — receive Stop, verify stream ends
#[test]
fn e2e_receive_input_and_stop() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // ── Send Input event with integer data ────────────────────────
    harness.send_input(TimedIncomingEvent {
        time_offset_secs: 0.0,
        event: IncomingEvent::Input {
            id: "numbers".parse().unwrap(),
            metadata: None,
            data: Some(Box::new(InputData::JsonObject {
                data: serde_json::json!([1, 2, 3]),
                data_type: None,
            })),
        },
    });

    // ── Send Stop (preload so the daemon thread doesn't block) ────
    harness.send_stop();

    // ── Tick 1: should receive Input ──────────────────────────────
    let event = harness.tick().expect("tick should return an event");
    match event {
        dora_node_api::Event::Input { id, data, .. } => {
            assert_eq!(id.to_string(), "numbers");
            // Data should be non-empty (3-element Int32 array)
            assert!(data.0.len() > 0, "input data should be non-empty");
        }
        other => panic!("expected Input event, got {other:?}"),
    }

    // ── Tick 2: should receive Stop ───────────────────────────────
    let event = harness.tick().expect("tick should return Stop");
    assert!(
        matches!(event, dora_node_api::Event::Stop(..)),
        "expected Stop event, got {event:?}"
    );

    // ── Stream should be exhausted after Stop ─────────────────────
    assert!(
        harness.tick().is_none(),
        "stream should be exhausted after Stop"
    );
}
