//! The [`NodeHarness`] — a self-contained unit-test driver for DORA nodes.
//!
//! ## Design
//!
//! `NodeHarness` wraps a DORA node created via
//! [`DoraNode::init_testing()`][init] and drives it with programmatic
//! inputs via in-memory channels.  Unlike the file-based
//! `IntegrationTestInput` workflow, this harness works inside a
//! standard `#[test]` function with zero external setup (NOT
//! `#[tokio::test]` — see [`tick`](NodeHarness::tick)).
//!
//! **Inputs** are injected at runtime through a live
//! [`TestingInput::Channel`]; **outputs** are captured through
//! [`TestingOutput::ToChannel`].  This gives test code complete
//! control over what the node receives and the ability to assert on
//! everything it produces.
//!
//! [init]: https://docs.rs/dora-node-api/latest/dora_node_api/struct.DoraNode.html#method.init_testing
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────┐  flume channel (input)  ┌──────────────────┐
//! │   Test code      │ ──────────────────────▶ │  DORA node       │
//! │  send_input()    │                         │  (the thing      │
//! │  tick()          │                         │   under test)    │
//! │  recv_output() ◀─│── flume channel (output)─│                  │
//! └──────────────────┘                         └──────────────────┘
//! ```
//!
//! The harness uses [`TestingInput::Channel`] (added upstream in
//! dora-rs for this project) to inject events at runtime, and
//! [`TestingOutput::ToChannel`] to capture outputs.

use std::collections::HashMap;

use dora_node_api::{
    integration_testing::{
        integration_testing_format::TimedIncomingEvent, TestingInput, TestingOptions, TestingOutput,
    },
    DoraNode, Event, EventStream, NodeError,
};

/// The main unit-test harness for a single DORA node.
///
/// # Example
///
/// ```ignore
/// use dora_test_utils::NodeHarness;
/// use dora_node_api::integration_testing::integration_testing_format::{
///     IncomingEvent, TimedIncomingEvent,
/// };
///
/// #[test]
/// fn test_classifier_node() {
///     let mut harness = NodeHarness::new()
///         .expect("failed to create harness");
///
///     // Inject an input event.
///     harness.send_input(TimedIncomingEvent {
///         time_offset_secs: 0.0,
///         event: IncomingEvent::Stop,
///     });
///
///     // Drive one iteration (blocking — init_testing uses blocking_recv).
///     harness.tick();
///
///     // Assert outputs.
///     let outputs = harness.recv_output("label");
///     assert!(outputs.is_some());
/// }
/// ```
pub struct NodeHarness {
    // ── Drop-order note ───────────────────────────────────────────
    // Rust drops struct fields in declaration order (top to bottom).
    // `input_tx` MUST be dropped before `event_stream` and `node`:
    // dropping the sender disconnects the flume input channel, which
    // unblocks the daemon thread's `rx.recv()`, allowing it to process
    // `EventStreamDropped` and `OutputsDone` during the subsequent
    // `event_stream`/`node` drops.
    //
    // DO NOT reorder these fields without understanding the two-thread
    // cleanup protocol described above.
    /// Sender for runtime event injection — pushes events into the
    /// node's integration-testing daemon connection via
    /// [`TestingInput::Channel`].
    pub(crate) input_tx: flume::Sender<TimedIncomingEvent>,
    /// Receiver for outputs captured via [`TestingOutput::ToChannel`].
    pub(crate) output_rx: flume::Receiver<serde_json::Map<String, serde_json::Value>>,
    /// Buffered outputs indexed by output ID (the `"id"` field in each
    /// JSON output map).
    pub(crate) output_buffers: HashMap<String, Vec<serde_json::Map<String, serde_json::Value>>>,
    /// The event stream returned by [`DoraNode::init_testing`].
    pub(crate) event_stream: EventStream,
    /// The DORA node under test, created via [`DoraNode::init_testing`].
    ///
    /// The node runs in a background thread; this handle is kept for
    /// future use (e.g. `send_output`, graceful shutdown).
    #[allow(dead_code)]
    pub(crate) node: DoraNode,
}

impl NodeHarness {
    /// Create a new harness with live input and output channels.
    ///
    /// Initializes the DORA node in testing mode via
    /// [`DoraNode::init_testing`] with [`TestingInput::Channel`]
    /// and [`TestingOutput::ToChannel`].
    ///
    /// # Errors
    ///
    /// Returns a [`NodeError`] if the underlying
    /// [`DoraNode::init_testing`] call fails.
    pub fn new() -> Result<Self, NodeError> {
        // ── Input channel: runtime event injection ─────────────────
        // Unbounded — test code controls pacing, so backpressure
        // from the node side would only complicate test logic.
        let (input_tx, input_rx) = flume::unbounded::<TimedIncomingEvent>();

        // ── Output channel: capture real node outputs ──────────────
        let (output_tx, output_rx) =
            flume::bounded::<serde_json::Map<String, serde_json::Value>>(256);

        let inputs = TestingInput::Channel(input_rx);
        let outputs = TestingOutput::ToChannel(output_tx);
        let options = TestingOptions {
            skip_output_time_offsets: true,
        };

        let (node, event_stream) = DoraNode::init_testing(inputs, outputs, options)?;

        Ok(Self {
            input_tx,
            output_rx,
            output_buffers: HashMap::new(),
            event_stream,
            node,
        })
    }

    /// Inject a synthetic input event at runtime.
    ///
    /// The event is delivered to the node through the live
    /// [`TestingInput::Channel`].  The node receives it on its next
    /// [`EventStream::recv`] call.
    ///
    /// # Panics
    ///
    /// Panics if the node's background thread has terminated (channel
    /// disconnected).  In normal test usage this shouldn't happen
    /// unless the node panicked or `init_testing` failed silently.
    pub fn send_input(&mut self, event: TimedIncomingEvent) {
        self.input_tx
            .send(event)
            .expect("NodeHarness: input channel disconnected — node may have panicked");
    }

    /// Convenience: inject a [`Stop`](dora_node_api::integration_testing::integration_testing_format::IncomingEvent::Stop)
    /// event (delivered immediately).
    pub fn send_stop(&mut self) {
        self.send_input(TimedIncomingEvent {
            time_offset_secs: 0.0,
            event:
                dora_node_api::integration_testing::integration_testing_format::IncomingEvent::Stop,
        });
    }

    /// Send an output from the node under test.
    ///
    /// Delegates to the underlying [`DoraNode::send_output`].  The output is
    /// captured by [`TestingOutput::ToChannel`] and can be retrieved via
    /// [`recv_output`](Self::recv_output).
    ///
    /// # Errors
    ///
    /// Returns a [`NodeError`] if `output_id` is invalid or the underlying
    /// `send_output` call fails.
    pub fn send_output(
        &mut self,
        output_id: &str,
        data: impl arrow::array::Array,
    ) -> Result<(), NodeError> {
        let data_id = output_id
            .parse()
            .map_err(|e| NodeError::Output(format!("invalid output_id '{output_id}': {e}")))?;
        self.node.send_output(data_id, Default::default(), data)
    }

    /// Drive the node to process **one** event from the [`EventStream`].
    ///
    /// After the event is received, any outputs produced by the node
    /// are collected into the internal buffers (accessible via
    /// [`recv_output`](Self::recv_output)).
    ///
    /// Returns the event that was processed, or `None` if the stream
    /// is exhausted (i.e. the input channel was closed and all events
    /// have been consumed).
    ///
    /// This is a **synchronous** call: `init_testing()` uses
    /// `blocking_recv` internally and cannot run inside a tokio
    /// runtime.  Use `#[test]` (not `#[tokio::test]`) for tests
    /// that drive the harness.
    pub fn tick(&mut self) -> Option<Event> {
        let event = self.event_stream.recv();

        // Collect any outputs the node produced during this tick.
        self.collect_pending_outputs();

        event
    }

    /// Drain all available outputs for `output_id` since the last
    /// call to [`tick`](Self::tick) (or since construction).
    ///
    /// Returns `None` if no output with that ID was produced.
    pub fn recv_output<O: Into<String>>(
        &mut self,
        output_id: O,
    ) -> Option<Vec<serde_json::Map<String, serde_json::Value>>> {
        // Collect any straggling outputs before draining.
        self.collect_pending_outputs();
        self.output_buffers.remove(&output_id.into())
    }

    /// Run the node to completion, pumping events until the input
    /// channel is exhausted.
    ///
    /// Useful for batch-testing a node with many inputs.
    ///
    /// **Not yet implemented** — will loop [`tick`](Self::tick) until
    /// the event stream returns `None` (Week 4).
    pub fn run_to_completion(&mut self) {
        todo!("run_to_completion — will loop tick() until idle (Week 4)")
    }

    // ── private helpers ────────────────────────────────────────────

    /// Collect all pending outputs from the flume channel into
    /// `output_buffers`, indexed by the `"id"` field in each JSON map.
    fn collect_pending_outputs(&mut self) {
        while let Ok(output) = self.output_rx.try_recv() {
            if let Some(id) = output.get("id").and_then(|v| v.as_str()) {
                self.output_buffers
                    .entry(id.to_string())
                    .or_default()
                    .push(output);
            }
        }
    }
}
