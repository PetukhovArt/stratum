use crate::report::Report;

pub mod agent;
pub mod github;
pub mod gitlab;
pub mod json;
pub mod jsonl;
pub mod sarif;
pub mod terminal;

pub use agent::AgentReporter;
pub use github::GithubReporter;
pub use gitlab::GitlabReporter;
pub use json::JsonReporter;
pub use jsonl::JsonlReporter;
pub use sarif::SarifReporter;
pub use terminal::TerminalReporter;

pub trait Reporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()>;
}
