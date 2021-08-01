use crate::binary_diff_analyzer::error::BinaryDiffAnalyzerError;

pub type Result<T> = std::result::Result<T, BinaryDiffAnalyzerError>;
