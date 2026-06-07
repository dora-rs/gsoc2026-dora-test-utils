//! Mock types that replace the DORA daemon connection with
//! in-memory channels.
//!
//! `dora-node-api` provides two key I/O surfaces for a node:
//!
//! | Real type | Role | Mock replacement |
//! |---|---|---|
//! | `EventStream` | Stream of incoming events | [`MockEventStream`](event_stream::MockEventStream) |
//! | `send_output()` on `DoraNode` | Send outgoing data | [`MockOutputSender`](output::MockOutputSender) |
//!
//! The mocks implement the same trait(s) as the real types but
//! substitute the daemon connection with [`tokio::sync::mpsc`]
//! channels, letting test code inject inputs and capture outputs
//! programmatically.

pub mod event_stream;
pub mod output;
