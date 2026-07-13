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
    /// Sender for runtime event injection.
    /// Wrapped in `Option` so [`close_input`](Self::close_input) can drop the sender
    /// to unblock the daemon thread, making [`send_output`](Self::send_output) safe.
    pub(crate) input_tx: Option<flume::Sender<TimedIncomingEvent>>,
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
            input_tx: Some(input_tx),
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
    /// Panics if [`close_input`](Self::close_input) was already called
    /// (the input sender has been dropped).  Also panics if the node's
    /// background thread has terminated (channel disconnected).
    pub fn send_input(&mut self, event: TimedIncomingEvent) {
        self.input_tx
            .as_ref()
            .expect("NodeHarness: input channel closed — close_input() was already called")
            .send(event)
            .expect("NodeHarness: input channel disconnected — node may have panicked");
    }

    /// Convenience: inject input data by ID.
    ///
    /// Wraps `data` in a [`TimedIncomingEvent`] and delegates to
    /// [`send_input`](Self::send_input).  The data type must implement
    /// [`IntoInputData`] — currently [`serde_json::Value`] and
    /// [`arrow::array::ArrayData`].
    ///
    /// # Panics
    ///
    /// Panics if [`close_input`](Self::close_input) was already called,
    /// if the channel is disconnected, or if `input_id` is not a valid
    /// [`DataId`](dora_node_api::DataId).
    ///
    /// After calling this method, the input channel is still open. To safely
    /// call [`send_output`](Self::send_output) afterward, you must first
    /// close the input channel via [`close_input`](Self::close_input) or
    /// [`run_to_completion`](Self::run_to_completion).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // JSON data — the most common case
    /// harness.send_data("image", serde_json::json!({"width": 640}));
    ///
    /// // Arrow data
    /// let array = Int32Array::from(vec![1, 2, 3]).into_data();
    /// harness.send_data("numbers", array);
    /// ```
    pub fn send_data(&mut self, input_id: &str, data: impl crate::IntoInputData) {
        use dora_node_api::integration_testing::integration_testing_format::{
            IncomingEvent, TimedIncomingEvent,
        };

        self.send_input(TimedIncomingEvent {
            time_offset_secs: 0.0,
            event: IncomingEvent::Input {
                id: input_id.parse().unwrap_or_else(|e| {
                    panic!("NodeHarness::send_data: invalid input_id '{input_id}': {e}")
                }),
                metadata: None,
                data: Some(Box::new(data.into_input_data())),
            },
        });
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

    /// Close the input channel, unblocking the daemon thread.
    ///
    /// After calling this, no more inputs can be sent via
    /// [`send_input`](Self::send_input). But [`send_output`](Self::send_output)
    /// and [`recv_output`](Self::recv_output) become safe to call without
    /// risk of deadlock — the daemon thread's `rx.recv()` returns
    /// `Disconnected` and it resumes processing `DaemonRequest::SendMessage`.
    ///
    /// [`run_to_completion`](Self::run_to_completion) calls this automatically
    /// after the event stream is exhausted.
    pub fn close_input(&mut self) {
        self.input_tx.take();
    }

    /// Send an output from the node under test.
    ///
    /// Delegates to the underlying [`DoraNode::send_output`].  The output is
    /// captured by [`TestingOutput::ToChannel`] and can be retrieved via
    /// [`recv_output`](Self::recv_output).
    ///
    /// This method automatically calls [`close_input`](Self::close_input) to
    /// unblock the daemon thread before sending.  After this method returns,
    /// no more inputs can be sent via [`send_input`](Self::send_input).
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
        // Parse the output_id before closing the input channel, so that
        // a parse error doesn't leave the input channel permanently closed
        // (which would break subsequent send_input / send_data calls).
        let data_id = output_id
            .parse()
            .map_err(|e| NodeError::Output(format!("invalid output_id '{output_id}': {e}")))?;

        // Close the input channel to unblock the daemon thread.  The daemon
        // is single-threaded and blocks on `input_rx.recv()` while processing
        // the eagerly-issued NextEvent request.  Dropping the sender causes
        // `recv()` to return Disconnected, the daemon returns to its request
        // loop, and our SendMessage becomes processable.
        //
        // The brief sleep forces a context switch so the daemon thread has
        // time to unwind the NextEvent path before we enqueue SendMessage.
        // Without this, the daemon may still be inside recv() when our
        // blind oneshot reply wait starts, causing a permanent hang on
        // resource-constrained CI runners.
        self.close_input();
        std::thread::sleep(std::time::Duration::from_millis(50));

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

    /// Run the node to completion, pumping events until the event stream
    /// is exhausted, a [`Stop`](Event::Stop) is received, or an
    /// [`InputClosed`](Event::InputClosed) arrives.
    ///
    /// A [`Stop`](Event::Stop) is injected automatically at the end of the
    /// input queue, so callers do not need to pre-load one — the method
    /// always terminates.  If caller already sent a Stop, the extra one is
    /// harmless (consumed after the stream ends).
    ///
    /// Returns all events processed during the run, up to and including the
    /// first terminal event.  After this method returns,
    /// [`close_input`](Self::close_input) has been called automatically —
    /// [`send_output`](Self::send_output) and
    /// [`recv_output`](Self::recv_output) are safe to use.
    ///
    /// ```ignore
    /// harness.send_input(my_input);
    /// // No need to call send_stop() — run_to_completion handles it.
    /// let events = harness.run_to_completion();
    /// assert!(events.iter().any(|e| matches!(e, Event::Stop(..))));
    ///
    /// // Now safe: daemon thread is unblocked
    /// harness.send_output("out", my_array).unwrap();
    /// let outputs = harness.recv_output("out");
    /// ```
    pub fn run_to_completion(&mut self) -> Vec<Event> {
        // Inject Stop at end of queue so the daemon thread never blocks
        // indefinitely in next_event() → rx.recv().  If the caller already
        // sent a Stop, the extra one is silently consumed after termination.
        self.send_stop();
        let mut events = Vec::new();
        while let Some(event) = self.tick() {
            let is_stop = matches!(event, Event::Stop(..));
            let is_input_closed = matches!(event, Event::InputClosed { .. });
            events.push(event);
            if is_stop || is_input_closed {
                break;
            }
        }
        // Unblock the daemon thread so send_output won't deadlock.
        self.close_input();
        events
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
            } else {
                // Don't silently drop outputs that lack a string "id" field —
                // store them under a sentinel key so the test author can debug.
                self.output_buffers
                    .entry("<missing-id>".to_string())
                    .or_default()
                    .push(output);
            }
        }
    }
}

impl Drop for NodeHarness {
    fn drop(&mut self) {
        // Close the input channel before the default field drops
        // proceed.  This unblocks the daemon thread, which replies to
        // the event stream thread and lets it exit.  The brief sleep
        // gives the daemon thread a scheduling quantum on
        // resource-constrained CI runners where the OS scheduler can
        // starve background threads for longer than the 1s timeout
        // used by EventStreamThreadHandle::drop.
        //
        // An unresponsive daemon at drop time causes a permanent hang:
        // the node sends cleanup requests (EventStreamDropped,
        // CloseOutputs, OutputsDone) to the daemon via a blocking
        // oneshot, but the daemon is still stuck in recv().
        self.close_input();

        // On resource-constrained CI runners (2 vCPU), the daemon
        // thread can be starved by the OS scheduler.  A brief sleep
        // deschedules the current thread, forcing a context switch
        // that gives the daemon time to wake from input_rx.recv() and
        // process the disconnect before the subsequent node drop sends
        // blocking cleanup requests.  Without this, the test hangs
        // permanently — the daemon is stuck in recv() and never
        // responds to EventStreamDropped / CloseOutputs / OutputsDone.
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_data_json() {
        let mut harness = NodeHarness::new().expect("harness should be created");

        harness.send_data("test_id", serde_json::json!([1, 2, 3]));

        // After send_data, the input should be queued. Drive with tick.
        let event = harness.tick().expect("should receive Input event");
        match event {
            dora_node_api::Event::Input { id, data, .. } => {
                assert_eq!(id.to_string(), "test_id");
                assert!(!data.0.is_empty(), "data should be non-empty");
            }
            other => panic!("expected Input event, got {other:?}"),
        }
    }

    #[test]
    fn test_send_data_arrow() {
        use arrow::array::{Array, Int32Array};

        let mut harness = NodeHarness::new().expect("harness should be created");

        let array = Int32Array::from(vec![42, 99]).into_data();
        harness.send_data("arrow_in", array);

        let event = harness.tick().expect("should receive Input event");
        match event {
            dora_node_api::Event::Input { id, data, .. } => {
                assert_eq!(id.to_string(), "arrow_in");
                assert!(!data.0.is_empty(), "data should be non-empty");
            }
            other => panic!("expected Input event, got {other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "NodeHarness: input channel closed")]
    fn test_send_data_panics_after_close_input() {
        let mut harness = NodeHarness::new().expect("harness should be created");
        harness.close_input();
        harness.send_data("x", serde_json::json!(42));
    }
}
