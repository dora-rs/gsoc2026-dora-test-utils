//! Mock replacement for `EventStream` (`dora-node-api`).
//!
//! The real `EventStream` (in `apis/rust/node/src/event_stream/mod.rs`)
//! receives events from the DORA daemon over a socket.  This mock replaces
//! that socket with a [`tokio::sync::mpsc::Receiver`], so test code can
//! inject pre-built events directly.
//!
//! ## Integration plan
//!
//! Once we depend on `dora-node-api`, `MockEventStream` will implement
//! whatever trait the real `EventStream` exposes (likely `Stream` or a
//! custom `EventReceiver` trait).  This ensures nodes written against the
//! real stream work with the mock without source changes.

/// A mock event stream that yields events pushed from test code.
///
/// # Example
///
/// ```ignore
/// let (mut stream, tx) = MockEventStream::new();
/// tx.send(Event::Input { id: "image".into(), data: arrow_array }).await;
/// // Pass `stream` to the node under test...
/// ```
pub struct MockEventStream {
    // Fields will be:
    //   rx: tokio::sync::mpsc::Receiver<dora_node_api::Event>,
    _private: (),
}

impl MockEventStream {
    /// Create a new mock stream and the matching sender handle.
    ///
    /// The sender is handed to test code; the receiver is passed to
    /// the node under test (wrapped inside `NodeHarness`).
    pub fn new() -> (Self, tokio::sync::mpsc::Sender<()>) {
        let (_tx, _rx) = tokio::sync::mpsc::channel::<()>(256);
        (Self { _private: () }, _tx)
        // TODO: use real Event type once dora dep is wired up.
        // let (tx, rx) = tokio::sync::mpsc::channel::<dora_node_api::Event>(256);
    }

    /// Receive the next event (async).
    ///
    /// In the real `EventStream` this calls `recv()` on the daemon socket.
    pub async fn recv(&mut self) -> Option<()> {
        todo!("recv — will dequeue from the internal receiver")
    }
}
