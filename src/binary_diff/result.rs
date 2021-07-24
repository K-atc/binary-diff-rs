use crate::binary_diff::error::BinaryDiffError;

pub type Result<T> = std::result::Result<T, BinaryDiffError>;
