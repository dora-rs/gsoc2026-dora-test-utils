//! The [`NodeHarness`] — a self-contained unit-test driver for DORA nodes.
//!
//! ## Design
//!
//! `NodeHarness` wraps a DORA node and drives it with programmatic inputs via
//! in-memory channels.  Unlike the existing [`DoraNode::init_testing()`][init]
//! (which requires environment variables and file-based I/O), this harness
//! works inside a standard `#[test]` function with zero external setup.
//!
//! [init]: https://docs.rs/dora-node-api/latest/dora_node_api/struct.DoraNode.html#method.init_testing
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────┐    in-memory     ┌──────────────┐
//! │  Test code   │ ─── channels ──▶ │  DORA node   │
//! │ (send_input, │                  │  (the thing  │
//! │  tick,       │ ◀── channels ──  │   under test)│
//! │  recv_output)│                  └──────────────┘
//! └──────────────┘
//! ```
//!
//! The harness replaces the DORA daemon connection with mock types
//! ([`crate::MockEventStream`], [`crate::MockOutputSender`]) that route
//! events and outputs through [`tokio::sync::mpsc`] channels.

use arrow::array::ArrayData;
use dora_node_api::{
    integration_testing::{
        integration_testing_format::{IncomingEvent, TimedIncomingEvent},
        IntegrationTestInput, TestingInput, TestingOptions, TestingOutput,
    },
    DoraNode, Event, EventStream,
};
use tokio::sync::mpsc;

use crate::mock::{
    event_stream::MockEventStream,
    output::{MockOutputSender, OutputCollector},
};

/// The main unit-test harness for a single DORA node.
///
/// # Example
///
/// ```ignore
/// use dora_test_utils::NodeHarness;
///
/// #[tokio::test]
/// async fn test_classifier_node() {
///     let mut harness = NodeHarness::new();
///
///     // Inject an input event.
///     harness.send_input("image", arrow_array);
///
///     // Drive one iteration.
///     harness.tick().await;
///
///     // Assert outputs.
///     let label = harness.recv_output("label");
///     assert_eq!(label, Some("cat"));
/// }
/// ```
#[allow(dead_code)]
pub struct NodeHarness {
    /// The DORA node under test, created via [`DoraNode::init_testing`].
    pub(crate) node: DoraNode,
    /// The event stream returned by [`DoraNode::init_testing`].
    pub(crate) event_stream: EventStream,
    /// Sender half of the mock event stream — used by [`send_input`](Self::send_input)
    /// to inject synthetic events into the node.
    pub(crate) input_tx: mpsc::Sender<Event>,
    /// Collector half of the mock output sender — captures outputs from
    /// `DoraNode::send_output()` calls so test code can inspect them.
    pub(crate) output_collector: OutputCollector,
}

impl NodeHarness {
    /// Create a new harness.
    ///
    /// Initializes the DORA node in testing mode via
    /// [`DoraNode::init_testing`] and wires up mock channels for
    /// programmatic input injection and output capture.
    ///
    /// # Panics
    ///
    /// Panics if the underlying [`DoraNode::init_testing`] call fails.
    /// In practice this should never happen with a valid test setup.
    pub fn new() -> Self {
        // Create mock channels for dynamic I/O (used by send_input / recv_output).
        let (_mock_stream, input_tx) = MockEventStream::new();
        let (_mock_sender, output_collector) = MockOutputSender::new();

        // ── Bootstrap the DORA node in testing mode ──────────────────
        // We pass a single Stop event so the node starts cleanly and
        // does not block waiting for initial input.  Test code injects
        // additional events at runtime via the mock input_tx sender.
        let events = vec![TimedIncomingEvent {
            time_offset_secs: 0.0,
            event: IncomingEvent::Stop,
        }];
        let inputs = TestingInput::Input(IntegrationTestInput::new(
            "test-node".parse().unwrap(),
            events,
        ));

        // Discard DORA's built-in testing output — the harness captures
        // outputs via the OutputCollector mock channel so test code can
        // assert on them with recv_output().
        let outputs = TestingOutput::ToWriter(Box::new(std::io::sink()));

        let options = TestingOptions {
            skip_output_time_offsets: true,
        };

        let (node, event_stream) = DoraNode::init_testing(inputs, outputs, options)
            .expect("NodeHarness::new: failed to initialize DoraNode in testing mode");

        Self {
            node,
            event_stream,
            input_tx,
            output_collector,
        }
    }

    /// Inject a synthetic input event identified by `input_id`.
    ///
    /// The data must be an Arrow array, matching the format real nodes
    /// receive from `EventStream::recv()`.
    ///
    /// # Panics
    ///
    /// May panic if the node does not declare this input in its
    /// YAML configuration (validation TBD — stretch goal).
    pub fn send_input<I: Into<String>>(&mut self, _input_id: I, _data: ArrayData) {
        todo!("send_input — will push the event to the mock event stream channel (Week 3)")
    }

    /// Drive the node to process **one** event.
    ///
    /// Equivalent to the node's internal event loop making one pass:
    /// dequeue an event → process → enqueue any outputs.
    pub async fn tick(&mut self) {
        todo!("tick — will poll the node for one iteration (Week 3)")
    }

    /// Drain all available outputs for `output_id` since the last
    /// `tick()` (or since construction).
    ///
    /// Returns `None` if no output with that ID was produced.
    pub fn recv_output<O: Into<String>>(&mut self, _output_id: O) -> Option<Vec<ArrayData>> {
        todo!("recv_output — will drain the output channel for the given id (Week 3)")
    }

    /// Run the node to completion, pumping events until the input
    /// channel is exhausted.
    ///
    /// Useful for batch-testing a node with many inputs.
    pub async fn run_to_completion(&mut self) {
        todo!("run_to_completion — will loop tick() until idle (Week 4)")
    }
}

impl Default for NodeHarness {
    fn default() -> Self {
        Self::new()
    }
}
