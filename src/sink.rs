//! TestSink — programmatic sink node for capturing and asserting DORA outputs.
//!
//! The [`run_test_sink`] function creates a DORA node via
//! [`DoraNode::init_from_env`], accumulates all incoming [`Event::Input`]
//! events, and compares them against expected data loaded from a file.

use std::path::PathBuf;

use dora_node_api::{DoraNode, Event};
use eyre::Context;
use serde::{Deserialize, Serialize};

/// Configuration for a test sink run.
#[derive(Debug, Clone)]
pub struct SinkConfig {
    /// Path to the expected output file (DORA JSON format).
    pub expected_file: PathBuf,
    /// Path to write the comparison result to.
    pub output_file: PathBuf,
    /// If true, exit with non-zero on mismatch.
    pub fail_on_mismatch: bool,
    /// If true, use exact JSON string comparison instead of Arrow semantic comparison.
    pub strict: bool,
}

/// Result of a sink comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkResult {
    /// Whether the received data matched the expected data.
    pub r#match: bool,
    /// Number of expected data items.
    pub expected_count: usize,
    /// Number of received data items.
    pub received_count: usize,
    /// List of differences found.
    pub differences: Vec<Difference>,
}

/// A single difference between expected and received data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    /// Index of the differing value, or None for structural mismatches.
    pub index: Option<usize>,
    /// Human-readable description of the difference.
    pub message: String,
}

/// Run a test sink: receive inputs, compare with expected, write result.
///
/// # Errors
///
/// Returns an error if:
/// - The expected file cannot be read or parsed
/// - `DoraNode::init_from_env()` fails
pub fn run_test_sink(config: SinkConfig) -> eyre::Result<SinkResult> {
    // ── 1. Load expected data ────────────────────────────────────
    let expected_json: serde_json::Value = {
        let contents = std::fs::read_to_string(&config.expected_file)
            .with_context(|| format!("failed to read expected file '{}'", config.expected_file.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("invalid JSON in expected file '{}'", config.expected_file.display()))?
    };

    let expected_data = expected_json
        .get("data")
        .ok_or_else(|| eyre::eyre!("expected file missing 'data' field"))?;

    let expected_elements: Vec<&serde_json::Value> = if let Some(arr) = expected_data.as_array() {
        arr.iter().collect()
    } else {
        vec![expected_data]
    };

    // ── 2. Initialize DORA node ──────────────────────────────────
    let (_node, mut events) =
        DoraNode::init_from_env().context("failed to initialize DORA node")?;

    // ── 3. Accumulate input events ────────────────────────────────
    let mut received: Vec<arrow::array::ArrayRef> = Vec::new();
    while let Some(event) = events.recv() {
        match event {
            Event::Input { data, .. } => {
                // ArrowData.0 is an ArrayRef (Arc<dyn Array>)
                received.push(data.0);
            }
            Event::Stop(_) | Event::InputClosed { .. } => break,
            _ => {}
        }
    }

    // ── 4. Compare ────────────────────────────────────────────────
    let result = if config.strict {
        compare_strict(&expected_elements, &received)
    } else {
        compare_semantic(&expected_elements, &received)
    };

    // ── 5. Write result ──────────────────────────────────────────
    let result_json = serde_json::to_string_pretty(&result)?;
    std::fs::write(&config.output_file, result_json)
        .with_context(|| format!("failed to write result to '{}'", config.output_file.display()))?;

    Ok(result)
}

/// Strict comparison: serialize received Arrow data back to JSON, compare with serde_json::Value equality.
fn compare_strict(
    expected: &[&serde_json::Value],
    received: &[arrow::array::ArrayRef],
) -> SinkResult {
    use arrow::array::RecordBatch;
    use arrow::datatypes::{Field, Schema};
    use arrow_json::writer::{JsonArray, Writer};
    use std::sync::Arc;

    let mut differences = Vec::new();

    // Serialize received data to JSON
    let received_json: Vec<serde_json::Value> = received
        .iter()
        .map(|array| {
            let schema = Schema::new(vec![Field::new("data", array.data_type().clone(), true)]);
            let batch =
                RecordBatch::try_new(Arc::new(schema), vec![array.clone()]).expect("valid batch");

            let mut buf = Vec::new();
            let mut writer = Writer::<_, JsonArray>::new(&mut buf);
            writer.write(&batch).expect("write should succeed");
            writer.finish().expect("finish should succeed");

            let json_str = String::from_utf8(buf).expect("valid utf-8");
            serde_json::from_str::<serde_json::Value>(&json_str).unwrap_or(serde_json::Value::Null)
        })
        .collect();

    // Flatten JSON array output into individual elements for comparison.
    // Each element produced by arrow_json::Writer<JsonArray> wraps the data
    // in a RecordBatch structure: `{"data": actual_value}`. We unwrap it
    // here so that the comparison works against raw expected JSON values.
    let received_flat: Vec<&serde_json::Value> = received_json
        .iter()
        .flat_map(|v| {
            if let Some(arr) = v.as_array() {
                arr.iter().collect::<Vec<_>>()
            } else {
                vec![v]
            }
        })
        .map(|v| {
            if let Some(obj) = v.as_object() {
                obj.get("data").unwrap_or(v)
            } else {
                v
            }
        })
        .collect();

    if received_flat.len() != expected.len() {
        differences.push(Difference {
            index: None,
            message: format!(
                "count mismatch: expected {} but got {}",
                expected.len(),
                received_flat.len()
            ),
        });
    }

    let max_len = expected.len().max(received_flat.len());
    for i in 0..max_len {
        let exp = expected.get(i);
        let rec = received_flat.get(i);
        match (exp, rec) {
            (Some(e), Some(r)) if e == r => {} // match
            (Some(e), Some(r)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("value mismatch at index {i}: expected {e}, got {r}"),
                });
            }
            (Some(_), None) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("missing received value at index {i}"),
                });
            }
            (None, Some(_)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("unexpected extra value at index {i}"),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    SinkResult {
        r#match: differences.is_empty(),
        expected_count: expected.len(),
        received_count: received_flat.len(),
        differences,
    }
}

/// Semantic comparison: parse expected JSON into Arrow arrays, compare with received Arrow data.
fn compare_semantic(
    expected: &[&serde_json::Value],
    received: &[arrow::array::ArrayRef],
) -> SinkResult {
    let mut differences = Vec::new();

    // Convert expected JSON values to Arrow arrays
    let expected_arrays: Vec<arrow::array::ArrayRef> = expected
        .iter()
        .filter_map(|v| {
            // Use the same conversion logic as source (inline for simplicity)
            match v {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Some(std::sync::Arc::new(arrow::array::Int64Array::from(vec![i])) as arrow::array::ArrayRef)
                    } else if let Some(f) = n.as_f64() {
                        Some(std::sync::Arc::new(arrow::array::Float64Array::from(vec![f])) as arrow::array::ArrayRef)
                    } else {
                        None
                    }
                }
                serde_json::Value::String(s) => {
                    Some(std::sync::Arc::new(arrow::array::StringArray::from(vec![s.as_str()])) as arrow::array::ArrayRef)
                }
                serde_json::Value::Bool(b) => {
                    Some(std::sync::Arc::new(arrow::array::BooleanArray::from(vec![*b])) as arrow::array::ArrayRef)
                }
                _ => None,
            }
        })
        .collect();

    // received is already Vec<ArrayRef>, just clone it
    let received_arrays: Vec<arrow::array::ArrayRef> = received.to_vec();

    if received_arrays.len() != expected_arrays.len() {
        differences.push(Difference {
            index: None,
            message: format!(
                "count mismatch: expected {} but got {}",
                expected_arrays.len(),
                received_arrays.len()
            ),
        });
    }

    let max_len = expected_arrays.len().max(received_arrays.len());
    for i in 0..max_len {
        let exp = expected_arrays.get(i);
        let rec = received_arrays.get(i);
        match (exp, rec) {
            (Some(e), Some(r)) if e == r => {} // match
            (Some(e), Some(r)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!(
                        "value mismatch at index {i}: expected {e:?}, got {r:?}"
                    ),
                });
            }
            (Some(_), None) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("missing received value at index {i}"),
                });
            }
            (None, Some(_)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("unexpected extra value at index {i}"),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    SinkResult {
        r#match: differences.is_empty(),
        expected_count: expected_arrays.len(),
        received_count: received_arrays.len(),
        differences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;

    /// Helper: write expected data to a temp file and return the path.
    #[allow(dead_code)]
    fn write_expected_file(data: &serde_json::Value) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{}", serde_json::to_string(data).unwrap()).unwrap();
        file
    }

    /// Helper: create a SinkConfig pointing at a temp expected file.
    #[allow(dead_code)]
    fn sink_config(expected: &serde_json::Value) -> (SinkConfig, tempfile::NamedTempFile) {
        let file = write_expected_file(expected);
        let output = tempfile::NamedTempFile::new().unwrap();
        let config = SinkConfig {
            expected_file: file.path().to_path_buf(),
            output_file: output.path().to_path_buf(),
            fail_on_mismatch: true,
            strict: false,
        };
        (config, file)
    }

    #[test]
    fn test_compare_semantic_exact_match() {
        let v42 = serde_json::json!(42);
        let v99 = serde_json::json!(99);
        let expected: Vec<&serde_json::Value> = vec![&v42, &v99];
        // Create equivalent Arrow arrays
        let received: Vec<arrow::array::ArrayRef> = vec![
            Arc::new(arrow::array::Int64Array::from(vec![42])),
            Arc::new(arrow::array::Int64Array::from(vec![99])),
        ];
        let result = compare_semantic(&expected, &received);
        assert!(result.r#match);
        assert_eq!(result.expected_count, 2);
        assert_eq!(result.received_count, 2);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn test_compare_semantic_count_mismatch() {
        let v1 = serde_json::json!(1);
        let v2 = serde_json::json!(2);
        let expected: Vec<&serde_json::Value> = vec![&v1, &v2];
        let received: Vec<arrow::array::ArrayRef> =
            vec![Arc::new(arrow::array::Int64Array::from(vec![1]))];
        let result = compare_semantic(&expected, &received);
        assert!(!result.r#match);
        assert_eq!(result.expected_count, 2);
        assert_eq!(result.received_count, 1);
        assert!(!result.differences.is_empty());
    }

    #[test]
    fn test_compare_semantic_value_mismatch() {
        let v42 = serde_json::json!(42);
        let expected: Vec<&serde_json::Value> = vec![&v42];
        let received: Vec<arrow::array::ArrayRef> =
            vec![Arc::new(arrow::array::Int64Array::from(vec![99]))];
        let result = compare_semantic(&expected, &received);
        assert!(!result.r#match);
        assert_eq!(result.differences.len(), 1);
        assert_eq!(result.differences[0].index, Some(0));
    }

    #[test]
    fn test_compare_semantic_empty_input() {
        let v1 = serde_json::json!(1);
        let expected: Vec<&serde_json::Value> = vec![&v1];
        let received: Vec<arrow::array::ArrayRef> = vec![];
        let result = compare_semantic(&expected, &received);
        assert!(!result.r#match);
        assert_eq!(result.received_count, 0);
        assert_eq!(result.expected_count, 1);
    }

    #[test]
    fn test_compare_strict_match() {
        let v42 = serde_json::json!(42);
        let expected: Vec<&serde_json::Value> = vec![&v42];
        let received: Vec<arrow::array::ArrayRef> =
            vec![Arc::new(arrow::array::Int64Array::from(vec![42]))];
        let result = compare_strict(&expected, &received);
        assert!(result.r#match);
    }

    #[test]
    fn test_compare_strict_mismatch() {
        let v42 = serde_json::json!(42);
        let expected: Vec<&serde_json::Value> = vec![&v42];
        let received: Vec<arrow::array::ArrayRef> =
            vec![Arc::new(arrow::array::Int64Array::from(vec![99]))];
        let result = compare_strict(&expected, &received);
        assert!(!result.r#match);
    }
}
