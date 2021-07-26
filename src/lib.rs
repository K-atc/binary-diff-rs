// #![no_std]
// Cannot apply `no_std` since BufReader is std::io::BufReader

mod binary_diff;

// Exported objects
pub use binary_diff::BinaryDiff;
pub use binary_diff::binary_diff_chunk::BinaryDiffChunk;

// extern crate alloc;
extern crate bcmp;
extern crate log;