//! # dora-test-utils
//!
//! Testing utilities for the [DORA](https://dora-rs.ai/) dataflow framework.
//!
//! This crate provides three layers of testing support, from lightweight
//! unit tests to full integration and regression testing:
//!
//! ## 1. Unit testing — [`NodeHarness`]
//!
//! Drive a single DORA node with synthetic inputs and assert its outputs
//! **without** starting the DORA daemon or coordinator.  Works inside
//! standard `#[test]` functions (NOT `#[tokio::test]` — `init_testing()`
//! uses `blocking_recv` internally, which panics inside a tokio runtime).
//!
//! ```ignore
//! use dora_test_utils::NodeHarness;
//! use dora_node_api::integration_testing::integration_testing_format::{
//!     IncomingEvent, TimedIncomingEvent,
//! };
//!
//! #[test]
//! fn test_classifier_node() {
//!     let mut harness = NodeHarness::new()
//!         .expect("failed to create harness");
//!
//!     // Inject an input event at runtime
//!     harness.send_input(TimedIncomingEvent {
//!         time_offset_secs: 0.0,
//!         event: IncomingEvent::Stop,
//!     });
//!
//!     // Drive one iteration (blocking — init_testing uses blocking_recv)
//!     harness.tick();
//!
//!     // Assert outputs
//!     let outputs = harness.recv_output("label");
//!     assert!(outputs.is_some());
//! }
//! ```
//!
//! ## 2. Integration testing — TestSource / TestSink
//!
//! Reusable binary nodes that emit test data from files and capture + assert
//! outputs.  Drop them into a real YAML dataflow alongside the node under
//! test. *(Planned — Week 5–8)*
//!
//! ## 3. Regression testing — Record / Replay
//!
//! Record real dataflow I/O to disk and replay it later to detect behavioral
//! regressions. *(Extended scope — Week 13–17)*
//!
//! ## Implementation Status
//!
//! | Component | Status |
//! |-----------|--------|
//! | [`NodeHarness`] (struct + `new()`) | Implemented — wraps [`DoraNode::init_testing()`][init] with live [`TestingInput::Channel`] + [`TestingOutput::ToChannel`] |
//! | [`NodeHarness::send_input()`] | Implemented — pushes [`TimedIncomingEvent`] through live flume channel |
//! | [`NodeHarness::tick()`] | Implemented — polls real [`EventStream`], collects outputs |
//! | [`NodeHarness::recv_output()`] | Implemented — drains output buffers; returns `Option<Vec<Map<String, Value>>>` |
//! | [`MockEventStream`] | Fully implemented + 3 tests |
//! | [`MockOutputSender`] / [`OutputCollector`] | Fully implemented + 3 tests |
//! | TestSource / TestSink binaries | Week 5 |
//! | Integration tests | Week 6–8 |
//! | Record / Replay | Week 13–17 (extended) |
//!
//! ## Relationship to upstream DORA
//!
//! This crate extends the foundation in `dora-node-api`'s
//! [`integration_testing`][dora-it] module ([`DoraNode::init_testing()`][init]).
//! It adds the **runtime event injection** that `init_testing()` currently
//! lacks — via a new [`TestingInput::Channel`] variant added upstream — and
//! the output-capture + assertion helpers.
//!
//! The harness uses live [`flume`] channels for both directions: input events
//! flow from test code to the node through [`TestingInput::Channel`]; outputs
//! flow back through [`TestingOutput::ToChannel`].  No file I/O or daemon
//! connection required.
//!
//! For pure-mock testing (no real node), the standalone mock types
//! ([`MockEventStream`], [`MockOutputSender`]) use
//! [`tokio::sync::mpsc`] channels.
//!
//! [init]: https://docs.rs/dora-node-api/latest/dora_node_api/struct.DoraNode.html#method.init_testing
//! [dora-it]: https://docs.rs/dora-node-api/latest/dora_node_api/integration_testing/

pub mod harness;
pub mod mock;

// Re-export the key types for convenience.
pub use harness::NodeHarness;
pub use mock::event_stream::MockEventStream;
pub use mock::output::{MockOutputSender, OutputCollector};
