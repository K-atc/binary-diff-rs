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

    // Returns how much bytes will be affected in original binary
    pub fn length(&self) -> Length {
        match self {
            BinaryDiffChunk::Same(_, length) => length.clone(),
            BinaryDiffChunk::Insert(_, bytes) => bytes.len(),
            BinaryDiffChunk::Delete(_, length) => length.clone(),
            BinaryDiffChunk::Replace(_, length, _) => length.clone(),
        }
    }

    // Returns how much bytes will be introduced to patched binary
    pub fn patched_length(&self) -> Length {
        match self {
            BinaryDiffChunk::Delete(_, _) => 0,
            BinaryDiffChunk::Replace(_, _, bytes) => bytes.len(),
            _ => self.length(),
        }
    }

    pub fn end(&self) -> Offset {
        self.offset() + self.length()
    }

    pub fn name(&self) -> &str {
        match self {
            BinaryDiffChunk::Same(_, _) => "Same",
            BinaryDiffChunk::Delete(_, _) => "Delete",
            BinaryDiffChunk::Insert(_, _) => "Insert",
            BinaryDiffChunk::Replace(_, _, _) => "Replace",
        }
    }
}

impl Ord for BinaryDiffChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        let res = self.offset().cmp(&other.offset());
        if res.is_eq() {
            match (self, other) {
                (BinaryDiffChunk::Same(_, _), _) => Ordering::Greater,
                (_, BinaryDiffChunk::Same(_, _)) => Ordering::Less,
                _ => Ordering::Equal,
            }
        } else {
            res
        }
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
                "Insert (offset={:#x}, bytes={})",
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

#[cfg(test)]
mod tests {
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Insert, Same};

    #[test]
    fn test_binary_diff_chunk_ordering() {
        assert!(Insert(1, vec![1]) < Same(1, 1));
    }
}
