//! TypeScript/JavaScript extractor using OXC.

#![forbid(unsafe_code)]

pub mod extractor;
pub mod source_span;

pub use extractor::{ExtractError, ExtractedData, LanguageExtractor, ParserDiagnostic, RawImport};
pub use source_span::{SourceSpan, offset_to_line_col};
