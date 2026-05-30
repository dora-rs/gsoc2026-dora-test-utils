//! # dora-test-utils
//!
//! Testing utilities for the [DORA](https://dora-rs.ai/) dataflow framework.
//!
//! This crate provides three layers of testing support:
//!
//! ## 1. Unit testing — [`NodeHarness`]
//!
//! Drive a single DORA node with synthetic inputs and assert its outputs
//! **without** starting the DORA daemon.  Works inside standard `#[test]`
//! functions.
//!
//! ```ignore
//! let mut harness = NodeHarness::new(my_node);
//! harness.send_input("image", arrow_data);
//! harness.tick().await;
//! let output = harness.recv_output("label");
//! assert_eq!(output.as_str(), Some("cat"));
//! ```
//!
//! ## 2. Integration testing — TestSource / TestSink
//!
//! Reusable binary nodes that emit test data from files and capture + assert
//! outputs.  Drop them into a real YAML dataflow alongside the node under test.
//!
//! ## 3. Regression testing — Record / Replay (extended scope)
//!
//! Record real dataflow I/O to disk and replay it later to detect behavioral
//! regressions.
//!
//! ## Relationship to upstream DORA
//!
//! This crate extends the foundation in `dora-node-api`'s
//! `integration_testing` module (`DoraNode::init_testing()`).  It does **not**
//! replace it — rather it adds the programmatic, in-memory harness and
//! assertion helpers that `init_testing()` currently lacks.

pub mod harness;
pub mod mock;

// Re-export the key types for convenience.
pub use harness::NodeHarness;
pub use mock::event_stream::MockEventStream;
pub use mock::output::MockOutputSender;
