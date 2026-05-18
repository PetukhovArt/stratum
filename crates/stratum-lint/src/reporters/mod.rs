use stratum_core::violation::Violation;

pub mod json;
pub mod sarif;
pub mod terminal;

pub use json::JsonReporter;
pub use sarif::SarifReporter;
pub use terminal::TerminalReporter;

pub trait Reporter {
    /// Write a complete report for the given violations to `out`.
    ///
    /// # Errors
    /// Returns any I/O error produced while writing.
    fn write(&self, violations: &[Violation], out: &mut dyn std::io::Write) -> std::io::Result<()>;
}
