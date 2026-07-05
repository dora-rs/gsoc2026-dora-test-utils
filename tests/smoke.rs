//! Smoke tests for the dora-test-utils crate.
//!
//! These tests verify that the public API types can be constructed and
//! composed with a real DORA node in testing mode.

use dora_test_utils::mock::event_stream::MockEventStream;
use dora_test_utils::mock::output::MockOutputSender;
use dora_test_utils::NodeHarness;

/// `NodeHarness::new()` creates a real DORA node with live channels,
/// and `send_input` → `tick` drives one event through the node.
///
/// Uses `#[test]` (not `#[tokio::test]`) because `init_testing()`
/// internally uses `blocking_recv`, which panics inside a tokio runtime.
#[test]
fn harness_construction_and_tick() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Send Stop so the node's event loop exits cleanly.
    harness.send_stop();

    // Drive one iteration — the node should receive Stop.
    let event = harness.tick();
    assert!(event.is_some());
}

/// `MockEventStream::new()` should return both a stream and a sender.
#[test]
fn mock_event_stream_pair() {
    let (_stream, _tx) = MockEventStream::new();
}

/// `MockOutputSender::new()` should return a sender and a collector
/// that starts empty.
#[test]
fn mock_output_sender_pair() {
    let (_sender, mut collector) = MockOutputSender::new();
    assert!(collector.is_empty());
    assert_eq!(collector.len(), 0);
    // drain on a missing key returns None
    assert!(collector.drain("nonexistent").is_none());
}
