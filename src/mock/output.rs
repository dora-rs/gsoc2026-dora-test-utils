//! Mock replacement for `DoraNode::send_output()` (`dora-node-api`).
//!
//! The real `send_output()` (in `apis/rust/node/src/node/mod.rs`) sends
//! data to the DORA daemon over a socket.  This mock replaces that socket
//! with a [`tokio::sync::mpsc::Sender`], so test code can capture and
//! assert on node outputs.
//!
//! ## Integration plan
//!
//! `MockOutputSender` will hold the `mpsc::Sender` side.  Each
//! `send_output(id, data)` call pushes `(id, data)` onto the channel.
//! Test code drains the receiver via [`NodeHarness::recv_output`].

use std::collections::HashMap;

/// Captures calls to `send_output` in memory instead of sending them to
/// the daemon.
///
/// # Example
///
/// ```ignore
/// let (sender, mut rx) = MockOutputSender::new();
/// sender.send("label".into(), arrow_array);
/// let outputs: Vec<_> = rx.drain().collect();
/// assert_eq!(outputs.len(), 1);
/// ```
pub struct MockOutputSender {
    // Fields will be:
    //   tx: tokio::sync::mpsc::Sender<(String, arrow::array::ArrayData)>,
    _private: (),
}

/// Accumulator that collects and indexes outputs by ID for assertion.
///
/// This is the receiver side of a `MockOutputSender` pair.
pub struct OutputCollector {
    /// All outputs received so far, keyed by output ID.
    ///
    /// Each `send_output(id, data)` call appends `data` to the vector
    /// for that ID.
    pub buffers: HashMap<String, Vec<arrow::array::ArrayData>>,
}

impl MockOutputSender {
    /// Create a new mock output sender and a corresponding collector.
    pub fn new() -> (Self, OutputCollector) {
        (
            Self { _private: () },
            OutputCollector {
                buffers: HashMap::new(),
            },
        )
    }

    /// Send an output — mirrors the real `DoraNode::send_output(id, data)`.
    pub fn send(&self, _output_id: String, _data: arrow::array::ArrayData) {
        todo!("send — will push (id, data) onto the internal mpsc channel")
    }
}

impl OutputCollector {
    /// Drain and return all outputs for the given `output_id`, or `None`
    /// if no outputs were recorded.
    pub fn drain(&mut self, output_id: &str) -> Option<Vec<arrow::array::ArrayData>> {
        self.buffers.remove(output_id)
    }

    /// Return the number of distinct output IDs recorded.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Return `true` if no outputs have been recorded.
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }
}
