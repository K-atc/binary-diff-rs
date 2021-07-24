type Offset = usize;

#[derive(Debug)]
pub enum BinaryDiffError {
    IoError(std::io::Error),
    InfiniteLoopError(Offset, Offset),
}
