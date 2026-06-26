//! TestSource — programmatic source node for injecting test data into DORA dataflows.
//!
//! The [`run_test_source`] function creates a DORA node via
//! [`DoraNode::init_from_env`] and emits pre-loaded data on a
//! configured output.  Designed for both daemon-based dataflows
//! and standalone testing mode (`DORA_TEST_WITH_INPUTS` env var).

use dora_node_api::{DoraNode, MetadataParameters};
use eyre::{Context, Result};

type DataId = dora_node_api::dora_core::config::DataId;

/// Configuration for a test source run.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// Output identifier to emit data on.
    pub output_id: String,
    /// DORA-format JSON payload: `{"data": [...], "data_type": {...}}`.
    pub data: serde_json::Value,
}

/// Run a test source: create a DORA node and emit loaded data.
///
/// # Errors
///
/// Returns an error if:
/// - The `data` JSON is missing the `"data"` field
/// - The `data` array is empty
/// - `DoraNode::init_from_env()` fails
/// - `send_output()` fails
pub fn run_test_source(config: SourceConfig) -> Result<()> {
    // ── 1. Validate and extract data ──────────────────────────────
    let data_array = config
        .data
        .get("data")
        .ok_or_else(|| eyre::eyre!("missing 'data' field in DORA-format input JSON"))?;

    let elements = data_array.as_array().ok_or_else(|| {
        eyre::eyre!("'data' field must be a JSON array, got: {}", data_array)
    })?;

    if elements.is_empty() {
        eyre::bail!("'data' array is empty — nothing to emit");
    }

    // ── 2. Convert each JSON element to an Arrow array ────────────
    let arrays: Vec<_> = elements
        .iter()
        .map(json_value_to_arrow_array)
        .collect::<Result<Vec<_>>>()?;

    // ── 3. Initialize DORA node ───────────────────────────────────
    let (mut node, _events) =
        DoraNode::init_from_env().context("failed to initialize DORA node")?;

    let output_id: DataId = config
        .output_id
        .parse()
        .map_err(|e| eyre::eyre!("invalid output_id '{}': {e}", config.output_id))?;

    // ── 4. Emit each array as a separate output message ───────────
    for array in arrays {
        node.send_output(output_id.clone(), MetadataParameters::default(), array)
            .context("send_output failed")?;
    }

    Ok(())
}

/// Convert a single JSON value to an Arrow array.
///
/// Infers the Arrow type from the JSON value:
/// - JSON number (integer) → Int64Array
/// - JSON number (float) → Float64Array
/// - JSON string → StringArray
/// - JSON bool → BooleanArray
/// - JSON array → wraps in a single-column StructArray via arrow_json
fn json_value_to_arrow_array(value: &serde_json::Value) -> Result<arrow::array::ArrayRef> {
    use arrow::array::{BooleanArray, Float64Array, Int64Array, StringArray};
    use std::sync::Arc;

    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Arc::new(Int64Array::from(vec![i])))
            } else if let Some(f) = n.as_f64() {
                Ok(Arc::new(Float64Array::from(vec![f])))
            } else {
                eyre::bail!("unsupported number value: {n}")
            }
        }
        serde_json::Value::String(s) => Ok(Arc::new(StringArray::from(vec![s.as_str()]))),
        serde_json::Value::Bool(b) => Ok(Arc::new(BooleanArray::from(vec![*b]))),
        serde_json::Value::Array(arr) => {
            // Nested array — wrap in a single-column struct via arrow_json
            json_array_to_arrow_struct(arr)
        }
        serde_json::Value::Object(_) => {
            // Object — wrap in a single-row struct via arrow_json
            json_obj_to_arrow_struct(value)
        }
        serde_json::Value::Null => {
            eyre::bail!("null values are not supported as standalone output")
        }
    }
}

/// Convert a JSON object to a single-row Arrow StructArray.
fn json_obj_to_arrow_struct(obj: &serde_json::Value) -> Result<arrow::array::ArrayRef> {
    use arrow::array::RecordBatch;
    use arrow::datatypes::Schema;
    use arrow_json::ReaderBuilder;
    use std::io::BufReader;
    use std::sync::Arc;

    let json_bytes = serde_json::to_vec(&vec![obj])?;
    let reader = BufReader::new(&json_bytes[..]);

    // Use empty schema for auto-inference
    let schema = Arc::new(Schema::empty());
    let json_reader = ReaderBuilder::new(schema).build(reader).map_err(|e| {
        eyre::eyre!("failed to build arrow_json reader: {e}")
    })?;

    let mut batches = Vec::new();
    for result in json_reader {
        let batch: RecordBatch = result.map_err(|e| eyre::eyre!("arrow_json read error: {e}"))?;
        batches.push(batch);
    }

    if batches.is_empty() {
        eyre::bail!("arrow_json produced no batches from object value");
    }

    // Merge all batches into one and extract the first column
    let merged = arrow::compute::concat_batches(&batches[0].schema(), &batches)
        .map_err(|e| eyre::eyre!("failed to concat batches: {e}"))?;

    if merged.num_columns() == 0 {
        eyre::bail!("arrow_json produced zero columns");
    }

    Ok(merged.column(0).clone())
}

/// Convert a JSON array to a single-column Arrow StructArray.
fn json_array_to_arrow_struct(arr: &[serde_json::Value]) -> Result<arrow::array::ArrayRef> {
    // Wrap each element in {"data": <element>} so arrow_json can parse it
    let wrapped: Vec<serde_json::Value> = arr
        .iter()
        .map(|v| serde_json::json!({"data": v}))
        .collect();

    json_obj_to_arrow_struct(&serde_json::Value::Array(wrapped))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal SourceConfig for testing.
    fn source_config(data: serde_json::Value) -> SourceConfig {
        SourceConfig {
            output_id: "test_out".to_string(),
            data,
        }
    }

    #[test]
    fn test_missing_data_field() {
        let config = source_config(serde_json::json!({"not_data": [1, 2]}));
        let result = run_test_source(config);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("missing 'data' field"),
            "error should mention missing 'data' field"
        );
    }

    #[test]
    fn test_empty_data_array() {
        let config = source_config(serde_json::json!({"data": []}));
        let result = run_test_source(config);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("empty"),
            "error should mention empty array"
        );
    }

    #[test]
    fn test_data_not_array() {
        let config = source_config(serde_json::json!({"data": 42}));
        let result = run_test_source(config);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("must be a JSON array"),
            "error should mention must be array"
        );
    }

    #[test]
    fn test_json_to_arrow_int64() {
        let arr = json_value_to_arrow_array(&serde_json::json!(42)).unwrap();
        assert_eq!(arr.len(), 1);
        let int_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::Int64Array>()
            .expect("should be Int64Array");
        assert_eq!(int_arr.value(0), 42);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_json_to_arrow_float64() {
        let arr = json_value_to_arrow_array(&serde_json::json!(3.14)).unwrap();
        assert_eq!(arr.len(), 1);
        let float_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::Float64Array>()
            .expect("should be Float64Array");
        assert!((float_arr.value(0) - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_json_to_arrow_string() {
        let arr = json_value_to_arrow_array(&serde_json::json!("hello")).unwrap();
        assert_eq!(arr.len(), 1);
        let str_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .expect("should be StringArray");
        assert_eq!(str_arr.value(0), "hello");
    }

    #[test]
    fn test_json_to_arrow_bool() {
        let arr = json_value_to_arrow_array(&serde_json::json!(true)).unwrap();
        assert_eq!(arr.len(), 1);
        let bool_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::BooleanArray>()
            .expect("should be BooleanArray");
        assert!(bool_arr.value(0));
    }

    #[test]
    fn test_json_to_arrow_null_panics() {
        let result = json_value_to_arrow_array(&serde_json::Value::Null);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("null"),
            "error should mention null"
        );
    }

}
