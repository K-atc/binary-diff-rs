use crate::BinaryDiffChunk;

#[derive(Debug, Eq, PartialEq)]
pub struct DerivesFrom<'a> {
    pub(crate) patched_position: usize,
    pub(crate) relative_position: usize,
    pub(crate) chunk: &'a BinaryDiffChunk,
}

impl<'a> DerivesFrom<'a> {
    pub fn original_position(&self) -> Option<usize> {
        match self.chunk {
            BinaryDiffChunk::Same(original_offset, length) => {
                debug_assert!(&self.relative_position < length);
                Some(original_offset + self.relative_position)
            }
            _ => None,
        }
    }

    pub fn patched_position(&self) -> usize {
        self.patched_position
    }

    pub fn relative_position(&self) -> usize {
        self.relative_position
    }

    pub fn chunk(&self) -> &BinaryDiffChunk {
        self.chunk
    }
}

#[cfg(test)]
mod tests {
    use crate::{DerivesFrom, BinaryDiffChunk};

    #[test]
    fn test_derives_from_same() {
        let chunk = BinaryDiffChunk::Same(1, 2);
        let derives_from_same = DerivesFrom {
            patched_position: 2,
            relative_position: 1,
            chunk: &chunk,
        };
        assert_eq!(derives_from_same.original_position(), Some(2));
    }

    #[test]
    fn test_derives_from_insert() {
        let chunk = BinaryDiffChunk::Insert(3, vec![0]);
        let derives_from = DerivesFrom {
            patched_position: 0,
            relative_position: 0,
            chunk: &chunk,
        };
        assert_eq!(derives_from.original_position(), None);
    }
}