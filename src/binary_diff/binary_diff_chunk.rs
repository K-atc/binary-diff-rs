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
}

impl fmt::Display for BinaryDiffChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Same(offset, length) => {
                write!(f, "Same  (offset={:#x}, length={:#x})", offset, length)
            }
            Self::Insert(offset, bytes) => write!(
                f,
                "Insert(offset={:#x}, bytes=[{}])",
                offset,
                bytes
                    .iter()
                    .map(|v| format!("{:02x}", v))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::Delete(offset, length) => {
                write!(f, "Delete(offset={:#x}, length={:#x})", offset, length)
            }
        }
    }
}
