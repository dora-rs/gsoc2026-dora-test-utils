//! Smoke tests for the dora-test-utils crate.
//!
//! These tests verify that the public API types can be constructed and
//! composed.  They don't yet exercise the internal logic, which depends
//! on wiring up real DORA types (pending the dora-node-api dependency).

use dora_test_utils::mock::output::MockOutputSender;
use dora_test_utils::mock::event_stream::MockEventStream;
use dora_test_utils::NodeHarness;

/// `NodeHarness` should be constructable via `new()` and `Default`.
#[test]
fn harness_construction() {
    let _h1 = NodeHarness::new();
    let _h2 = NodeHarness::default();
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
