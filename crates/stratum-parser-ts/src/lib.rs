//! TypeScript/JavaScript extractor using OXC.

#![forbid(unsafe_code)]

pub mod extractor;
pub mod imports;
pub mod oxc_ts;
pub mod source_span;

pub use extractor::{ExtractError, ExtractedData, LanguageExtractor, ParserDiagnostic, RawImport};
pub use oxc_ts::OxcTsExtractor;
pub use source_span::{SourceSpan, offset_to_line_col};
