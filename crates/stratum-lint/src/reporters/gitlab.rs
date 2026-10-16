use super::Reporter;
use crate::report::Report;

#[derive(Debug)]
pub struct GitlabReporter {
    #[expect(dead_code, reason = "read by the reporter body in the formats slice")]
    prefix: String,
}

impl GitlabReporter {
    #[must_use]
    pub const fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

impl Reporter for GitlabReporter {
    fn write(&self, _report: &Report, _out: &mut dyn std::io::Write) -> std::io::Result<()> {
        Err(std::io::Error::other(
            "--format gitlab is not implemented yet",
        ))
    }
}
