type Offset = usize;
type Length = usize;
type Bytes = Vec<u8>;

#[derive(Debug, Eq, PartialEq)]
pub enum BinaryDiffChunk {
    Same(Offset, Length),
    Insert(Offset, Bytes),
    Delete(Offset, Length),
}
