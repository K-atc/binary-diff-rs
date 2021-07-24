use std::cmp::Ordering;
use std::fmt;
// use alloc::fmt;
// use alloc::vec::Vec;

type Offset = usize;
type Length = usize;
type Bytes = Vec<u8>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BinaryDiffChunk {
    Same(Offset, Length),
    Insert(Offset, Bytes),
    Delete(Offset, Length),
    Replace(Offset, Length, Bytes),
}

impl BinaryDiffChunk {
    pub fn offset(&self) -> &Offset {
        match self {
            BinaryDiffChunk::Same(offset, _) => offset,
            BinaryDiffChunk::Insert(offset, _) => offset,
            BinaryDiffChunk::Delete(offset, _) => offset,
            BinaryDiffChunk::Replace(offset, _, _) => offset,
        }
    }
}

impl Ord for BinaryDiffChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset().cmp(&other.offset())
    }
}

impl PartialOrd for BinaryDiffChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn stringify_bytes(bytes: &Bytes) -> String {
    format!(
        "[{}]",
        bytes
            .iter()
            .map(|v| format!("{:02x}", v))
            .collect::<Vec<String>>()
            .join(" ")
    )
}

impl fmt::Display for BinaryDiffChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Same(offset, length) => {
                write!(f, "Same   (offset={:#x}, length={:#x})", offset, length)
            }
            Self::Insert(offset, bytes) => write!(
                f,
                "Insert (offset={:#x}, bytes=[{}])",
                offset,
                stringify_bytes(bytes)
            ),
            Self::Delete(offset, length) => {
                write!(f, "Delete (offset={:#x}, length={:#x})", offset, length)
            }
            Self::Replace(offset, length, bytes) => {
                write!(
                    f,
                    "Replace(offset={:#x}, length={:#x}, bytes={})",
                    offset,
                    length,
                    stringify_bytes(bytes)
                )
            }
        }
    }
}
