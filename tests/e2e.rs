//! End-to-end test for the dora-test-utils harness.
//!
//! Exercises the full input pipeline: send_input → tick → receive events.

use dora_node_api::integration_testing::integration_testing_format::{
    IncomingEvent, InputData, TimedIncomingEvent,
};
use dora_node_api::Event;
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

/// Output path: close_input → send_output → recv_output (no tick needed).
///
/// The daemon thread is single-threaded: it blocks on `input_rx.recv()`
/// when processing `NextEvent`.  We must call `close_input()` first to
/// unblock the daemon before `send_output` will be processed.
#[test]
fn e2e_send_output_and_recv() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Close the input channel to unblock the daemon thread so that
    // send_output requests are processed without deadlocking.
    harness.close_input();

    // Send an output via the harness (delegates to DoraNode::send_output).
    let output_id = "test_output";
    let array = arrow::array::Int32Array::from(vec![10, 20, 30]);
    harness
        .send_output(output_id, array)
        .expect("send_output should succeed after close_input");

    // Retrieve the output.
    let outputs = harness
        .recv_output(output_id)
        .expect("should have captured output for 'test_output'");
    assert_eq!(outputs.len(), 1, "expected one output message");
    assert!(
        outputs[0].contains_key("data"),
        "output should contain data"
    );
    assert!(outputs[0].contains_key("id"), "output should contain id");
    let output_id_value = outputs[0].get("id").and_then(|v| v.as_str());
    assert_eq!(
        output_id_value,
        Some("test_output"),
        "output id should match"
    );
}

/// run_to_completion: pre-load Input + Stop, verify all events returned.
///
/// After run_to_completion() returns, the input channel is closed
/// (close_input was called), so send_output is safe.
#[test]
fn e2e_run_to_completion_returns_events() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Pre-load an Input event with data.
    harness.send_input(TimedIncomingEvent {
        time_offset_secs: 0.0,
        event: IncomingEvent::Input {
            id: "step1".parse().unwrap(),
            metadata: None,
            data: Some(Box::new(InputData::JsonObject {
                data: serde_json::json!([42]),
                data_type: None,
            })),
        },
    });

    // Pre-load Stop so the daemon thread won't block.
    harness.send_stop();

    // Run to completion.
    let events = harness.run_to_completion();

    // Should have received both Input and Stop.
    assert!(
        events.len() >= 2,
        "expected at least 2 events (Input + Stop), got {}",
        events.len()
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Input { .. })),
        "should contain an Input event"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Stop(..))),
        "should contain a Stop event"
    );

    // After run_to_completion(), send_output should work (close_input was called).
    let array = arrow::array::Int32Array::from(vec![99]);
    harness
        .send_output("post_run", array)
        .expect("send_output should succeed after run_to_completion");

    let outputs = harness.recv_output("post_run");
    assert!(outputs.is_some(), "should have captured output after run");
}

/// Full pipeline: send_input → tick through input → send_output → recv_output.
///
/// Verifies that the input and output paths both work within the same
/// harness lifecycle.
#[test]
fn e2e_full_pipeline_input_to_output() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Phase 1: Send input + stop, drive to completion.
    harness.send_input(TimedIncomingEvent {
        time_offset_secs: 0.0,
        event: IncomingEvent::Input {
            id: "data_in".parse().unwrap(),
            metadata: None,
            data: Some(Box::new(InputData::JsonObject {
                data: serde_json::json!([1, 2, 3, 4, 5]),
                data_type: None,
            })),
        },
    });
    harness.send_stop();

    let events = harness.run_to_completion();
    assert!(
        events.iter().any(|e| matches!(e, Event::Stop(..))),
        "should have received Stop"
    );

    // Phase 2: After completion, send outputs (close_input was called).
    let array1 = arrow::array::Float64Array::from(vec![1.1, 2.2, 3.3]);
    harness
        .send_output("results", array1)
        .expect("send_output should succeed after run_to_completion");

    let array2 = arrow::array::Float64Array::from(vec![4.4, 5.5]);
    harness
        .send_output("results", array2)
        .expect("second send_output should also succeed");

    // Phase 3: Retrieve all outputs for "results".
    let outputs = harness
        .recv_output("results")
        .expect("should have captured outputs for 'results'");
    assert_eq!(outputs.len(), 2, "expected 2 output messages for 'results'");
    for output in &outputs {
        assert!(
            output.contains_key("data"),
            "each output should contain 'data'"
        );
    }
}
