use serde_json::Value;

pub fn json_str(value: &Value, pointer: &str) -> Option<String> {
    value.pointer(pointer).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

pub fn json_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|v| v.as_i64())
}
