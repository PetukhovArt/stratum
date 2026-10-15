use schemars::schema_for;
use serde_json::{Value, json};

use crate::Config;

/// Produce the JSON Schema for `stratum.config.jsonc`.
///
/// # Errors
/// Returns the underlying `serde_json::Error` if the schemars-emitted schema
/// cannot be re-serialised (in practice this never happens).
pub fn schema() -> Result<Value, serde_json::Error> {
    let mut schema = serde_json::to_value(schema_for!(Config))?;
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        properties.insert("$schema".into(), json!({ "type": "string" }));
    }
    Ok(schema)
}
