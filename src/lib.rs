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
//! standard `#[test]` or `#[tokio::test]` functions.
//!
//! ```ignore
//! use dora_test_utils::NodeHarness;
//!
//! #[tokio::test]
//! async fn test_classifier_node() {
//!     let mut harness = NodeHarness::new();
//!     harness.send_input("image", arrow_data);
//!     harness.tick().await;
//!     let output = harness.recv_output("label");
//!     assert_eq!(output, Some(vec![...]));
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
//! | [`NodeHarness`] (struct + `new()`) | ✅ Implemented — wraps [`DoraNode::init_testing()`][init] |
//! | [`NodeHarness::send_input()`] | ⏳ Week 3 |
//! | [`NodeHarness::tick()`] | ⏳ Week 3 |
//! | [`NodeHarness::recv_output()`] | ⏳ Week 3 |
//! | [`MockEventStream`] | ✅ Fully implemented + 3 tests |
//! | [`MockOutputSender`] / [`OutputCollector`] | ✅ Fully implemented + 3 tests |
//! | TestSource / TestSink binaries | ⏳ Week 5 |
//! | Integration tests | ⏳ Week 6–8 |
//! | Record / Replay | ⏳ Week 13–17 (extended) |
//!
//! ## Relationship to upstream DORA
//!
//! This crate extends the foundation in `dora-node-api`'s
//! [`integration_testing`][dora-it] module ([`DoraNode::init_testing()`][init]).
//! It does **not** replace it — rather it adds the programmatic,
//! in-memory harness and assertion helpers that `init_testing()`
//! currently lacks.
//!
//! Our mock types ([`MockEventStream`], [`MockOutputSender`]) replace
//! the daemon connection with [`tokio::sync::mpsc`] channels, enabling
//! test code to inject inputs and capture outputs imperatively instead
//! of pre-declaring them in JSON files.
//!
//! [init]: https://docs.rs/dora-node-api/latest/dora_node_api/struct.DoraNode.html#method.init_testing
//! [dora-it]: https://docs.rs/dora-node-api/latest/dora_node_api/integration_testing/

pub mod harness;
pub mod mock;

// Re-export the key types for convenience.
pub use harness::NodeHarness;
pub use mock::event_stream::MockEventStream;
pub use mock::output::{MockOutputSender, OutputCollector};
