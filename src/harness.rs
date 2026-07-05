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
//! The harness replaces the DORA daemon connection with
//! [`MockEventStream`](crate::mock::MockEventStream) and
//! [`MockOutputSender`](crate::mock::MockOutputSender), which route
//! events and outputs through [`tokio::sync::mpsc`] channels.

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
///     harness.send_input("image", arrow::array::...);
///
///     // Drive one iteration.
///     harness.tick().await;
///
///     // Assert outputs.
///     let label = harness.recv_output("label");
///     assert_eq!(label, Some("cat"));
/// }
/// ```
pub struct NodeHarness {
    // TODO: wrap the real DoraNode / EventStream once the crate is
    //       integrated with the dora dependency.
    //
    // For now this is a design-time stub — the fields will be:
    //   node: DoraNode,
    //   input_tx: mpsc::Sender<Event>,
    //   output_rx: mpsc::Receiver<(String, ArrowArray)>,
    //   pending_outputs: HashMap<String, Vec<ArrowArray>>,
    _private: (),
}

impl NodeHarness {
    /// Create a new harness.
    ///
    /// In the final implementation this will call
    /// `DoraNode::init_testing()` under the hood but inject mock
    /// channels instead of daemon connections.
    pub fn new() -> Self {
        Self { _private: () }
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
    pub fn send_input<I: Into<String>>(&mut self, _input_id: I, _data: arrow::array::ArrayData) {
        todo!("send_input — will push the event to the mock event stream channel")
    }

    /// Drive the node to process **one** event.
    ///
    /// Equivalent to the node's internal event loop making one pass:
    /// dequeue an event → process → enqueue any outputs.
    pub async fn tick(&mut self) {
        todo!("tick — will poll the node for one iteration")
    }

    /// Drain all available outputs for `output_id` since the last
    /// `tick()` (or since construction).
    ///
    /// Returns `None` if no output with that ID was produced.
    pub fn recv_output<O: Into<String>>(&mut self, _output_id: O) -> Option<Vec<arrow::array::ArrayData>> {
        todo!("recv_output — will drain the output channel for the given id")
    }

    /// Run the node to completion, pumping events until the input
    /// channel is exhausted.
    ///
    /// Useful for batch-testing a node with many inputs.
    pub async fn run_to_completion(&mut self) {
        todo!("run_to_completion — will loop tick() until idle")
    }
}

impl Default for NodeHarness {
    fn default() -> Self {
        Self::new()
    }
}
