#[derive(Debug)]
pub enum BinaryDiffError {
    IoError(std::io::Error),
}
