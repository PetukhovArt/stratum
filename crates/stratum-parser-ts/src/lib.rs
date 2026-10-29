//! TypeScript/JavaScript extractor using OXC.

#![forbid(unsafe_code)]

mod annotations;
mod calls;
mod comments;
mod exports;
mod extractor;
mod imports;
mod oxc_ts;
mod resolver;
mod source_span;

pub use annotations::extract_purity;
pub use extractor::{
    ExtractError, ExtractedData, InvalidCallEntry, LanguageExtractor, RawCall, RawExportDecl,
    RawExports, RawImport, RawSuppression, TrackedCalls,
};
pub use imports::{Collected, collect};
pub use oxc_span::SourceType;
pub use oxc_ts::OxcTsExtractor;
pub use resolver::{Overlaid, Overlay, PathResolver, ResolveError};
pub use source_span::{LineIndex, SourceSpan, offset_to_line_col};
