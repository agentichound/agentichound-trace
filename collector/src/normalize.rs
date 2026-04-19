use serde::Serialize;
use serde_json::{Map, Value};

fn normalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Map::new();
            for (k, v) in pairs {
                out.insert(k.clone(), normalize_value(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(normalize_value).collect()),
        _ => value.clone(),
    }
}

pub fn canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let as_value = serde_json::to_value(value)?;
    let normalized = normalize_value(&as_value);
    serde_json::to_string(&normalized)
}
