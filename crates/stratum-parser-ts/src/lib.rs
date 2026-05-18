//! TypeScript/JavaScript extractor using OXC.

#![forbid(unsafe_code)]

mod annotations;
mod extractor;
mod imports;
mod oxc_ts;
mod resolver;
mod source_span;

pub use annotations::extract_stage;
pub use extractor::{ExtractError, ExtractedData, LanguageExtractor, ParserDiagnostic, RawImport};
pub use oxc_ts::OxcTsExtractor;
pub use resolver::{PathResolver, ResolveError};
pub use source_span::{SourceSpan, offset_to_line_col};
