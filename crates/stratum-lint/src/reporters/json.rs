use stratum_core::violation::Violation;

use super::Reporter;

#[derive(Debug, Default)]
pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn write(&self, violations: &[Violation], out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "violations": violations,
        }))
        .map_err(std::io::Error::other)?;
        out.write_all(json.as_bytes())?;
        writeln!(out)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_violations_emit_empty_array() {
        let mut buf = Vec::new();
        JsonReporter.write(&[], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"violations\": []"));
        assert!(s.contains("\"version\": 1"));
    }
}
