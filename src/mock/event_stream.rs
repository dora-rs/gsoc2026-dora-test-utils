//! Mock replacement for `EventStream` (`dora-node-api`).
//!
//! The real `EventStream` (in `apis/rust/node/src/event_stream/mod.rs`)
//! receives events from the DORA daemon over a socket.  This mock replaces
//! that socket with a [`tokio::sync::mpsc::Receiver`], so test code can
//! inject pre-built events directly.

use dora_node_api::Event;
use tokio::sync::mpsc;

/// A mock event stream that yields events pushed from test code.
///
/// # Example
///
/// ```ignore
/// let (mut stream, tx) = MockEventStream::new();
/// tx.send(Event::Input { id: "image".into(), metadata: ..., data: arrow_array }).await;
/// // Pass `stream` to the node under test...
/// while let Some(event) = stream.recv().await {
///     println!("Got event: {:?}", event);
/// }
/// ```
pub struct MockEventStream {
    rx: mpsc::Receiver<Event>,
}

impl MockEventStream {
    /// Create a new mock stream and the matching sender handle.
    ///
    /// The sender is handed to test code; the receiver is passed to
    /// the node under test (wrapped inside `NodeHarness`).
    pub fn new() -> (Self, mpsc::Sender<Event>) {
        let (tx, rx) = mpsc::channel::<Event>(256);
        (Self { rx }, tx)
    }

    /// Receive the next event (async).
    ///
    /// In the real `EventStream` this calls `recv()` on the daemon socket.
    pub async fn recv(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

impl Default for MockEventStream {
    fn default() -> Self {
        Self::new().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_event_stream_single_event() {
        let (mut stream, tx) = MockEventStream::new();

        let event = Event::Error("test error".to_string());
        tx.send(event).await.unwrap();

        let received = stream.recv().await;
        assert!(received.is_some());
        match received.unwrap() {
            Event::Error(msg) => assert_eq!(msg, "test error"),
            _ => panic!("unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_mock_event_stream_multiple_events() {
        let (mut stream, tx) = MockEventStream::new();

        tx.send(Event::Error("error 1".to_string())).await.unwrap();
        tx.send(Event::Error("error 2".to_string())).await.unwrap();
        tx.send(Event::Error("error 3".to_string())).await.unwrap();

        assert!(stream.recv().await.is_some());
        assert!(stream.recv().await.is_some());
        assert!(stream.recv().await.is_some());

        // Stream exhausted — drop sender to close the channel,
        // otherwise recv().await blocks forever.
        drop(tx);
        assert!(stream.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_mock_event_stream_multiple_senders() {
        let (mut stream, tx1) = MockEventStream::new();
        let tx2 = tx1.clone();

        tx1.send(Event::Error("from tx1".to_string()))
            .await
            .unwrap();
        tx2.send(Event::Error("from tx2".to_string()))
            .await
            .unwrap();

        let e1 = stream.recv().await;
        assert!(e1.is_some());

        let e2 = stream.recv().await;
        assert!(e2.is_some());

        // Both messages received — drop all senders before checking None
        drop(tx1);
        drop(tx2);
        assert!(stream.recv().await.is_none());
    }
}
