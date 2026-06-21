//! Extension traits for converting test data into DORA input formats.
//!
//! The [`IntoInputData`] trait allows [`NodeHarness::send_data`] to
//! accept multiple data types — currently [`serde_json::Value`] and
//! [`arrow::array::ArrayData`].

use dora_node_api::integration_testing::integration_testing_format::InputData;

/// Convert test data into an [`InputData`] variant.
///
/// Implementations produce [`InputData`] values suitable for injection
/// through [`NodeHarness::send_data`](crate::NodeHarness::send_data).
pub trait IntoInputData {
    fn into_input_data(self) -> InputData;
}

impl IntoInputData for serde_json::Value {
    fn into_input_data(self) -> InputData {
        InputData::JsonObject {
            data: self,
            data_type: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_input_data_for_json_value() {
        let value = serde_json::json!({"key": "value"});
        let input_data = value.into_input_data();
        // Should produce JsonObject variant
        match input_data {
            InputData::JsonObject { data, data_type } => {
                assert_eq!(data, serde_json::json!({"key": "value"}));
                assert!(data_type.is_none());
            }
            _ => panic!("Expected JsonObject variant"),
        }
    }
}
