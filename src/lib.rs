// #![no_std]
// Cannot apply `no_std` since BufReader is std::io::BufReader

mod binary_diff;
mod binary_diff_analyzer;

// Exported objects
pub use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
pub use crate::binary_diff::BinaryDiff;
pub use crate::binary_diff_analyzer::BinaryDiffAnalyzer;

// extern crate alloc;
extern crate bcmp;
extern crate log;
