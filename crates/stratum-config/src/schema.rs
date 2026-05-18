use schemars::schema_for;
use serde_json::Value;

use crate::Config;

/// Produce the JSON Schema for `stratum.config.jsonc`.
///
/// # Errors
/// Returns the underlying `serde_json::Error` if the schemars-emitted schema
/// cannot be re-serialised (in practice this never happens).
pub fn schema() -> Result<Value, serde_json::Error> {
    serde_json::to_value(schema_for!(Config))
}
