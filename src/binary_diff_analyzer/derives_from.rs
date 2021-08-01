use crate::BinaryDiffChunk;

#[derive(Debug, Eq, PartialEq)]
pub struct DerivesFrom<'a> {
    pub position: Option<usize>,
    pub chunk: &'a BinaryDiffChunk,
}