//! Mock types that replace the DORA daemon connection with
//! in-memory channels for pure-mock testing (no real node).
//!
//! These are **standalone** building blocks.  When you want to test
//! against a real `DoraNode`, use [`crate::NodeHarness`] instead — it
//! wires up [`TestingOutput::ToChannel`] to capture actual node
//! outputs.  The mock types here are useful for unit-testing
//! node-internal event-handling logic in isolation, or as plumbing
//! for a pure-mock event loop.
//!
//! `dora-node-api` provides two key I/O surfaces for a node:
//!
//! | Real type | Role | Mock replacement |
//! |---|---|---|
//! | `EventStream` | Stream of incoming events | [`MockEventStream`](event_stream::MockEventStream) |
//! | `send_output()` on `DoraNode` | Send outgoing data | [`MockOutputSender`](output::MockOutputSender) |
//!
//! The mocks substitute the daemon connection with
//! [`tokio::sync::mpsc`] channels, letting test code inject inputs
//! and capture outputs programmatically.

pub mod event_stream;
pub mod output;
