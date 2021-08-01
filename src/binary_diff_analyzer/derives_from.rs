use crate::BinaryDiffChunk;

#[derive(Debug, Eq, PartialEq)]
pub struct DerivesFrom<'a> {
    pub position: Option<usize>,
    pub relative_position: usize,
    pub chunk: &'a BinaryDiffChunk,
}