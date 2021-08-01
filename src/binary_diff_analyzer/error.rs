#[derive(Debug)]
pub enum BinaryDiffAnalyzerError {
    IoError(std::io::Error),
}
