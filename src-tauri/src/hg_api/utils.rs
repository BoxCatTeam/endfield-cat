use serde_json::Value;

pub fn json_str(value: &Value, pointer: &str) -> Option<String> {
    value.pointer(pointer).and_then(|v| v.as_str()).map(ToOwned::to_owned)
}

pub fn json_i64(value: &Value, key: &str) -> Option<i64> {
    let v = value.get(key)?;

    if let Some(n) = v.as_i64() {
        return Some(n);
    }

    if let Some(n) = v.as_u64() {
        return i64::try_from(n).ok();
    }

    if let Some(s) = v.as_str() {
        let s = s.trim();
        if let Ok(n) = s.parse::<i64>() {
            return Some(n);
        }
        if let Ok(n) = s.parse::<u64>() {
            return i64::try_from(n).ok();
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_i64_accepts_number_and_string() {
        let v: Value = serde_json::json!({
            "a": 0,
            "b": "0",
            "c": " 42 ",
            "d": 42u64
        });

        assert_eq!(json_i64(&v, "a"), Some(0));
        assert_eq!(json_i64(&v, "b"), Some(0));
        assert_eq!(json_i64(&v, "c"), Some(42));
        assert_eq!(json_i64(&v, "d"), Some(42));
        assert_eq!(json_i64(&v, "missing"), None);
    }
}
