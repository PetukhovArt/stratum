#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod cli;
pub mod commands;
mod git;
pub mod report;
pub mod reporters;
pub mod scope;
pub mod watch;
