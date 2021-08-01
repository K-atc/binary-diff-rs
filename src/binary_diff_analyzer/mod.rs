mod error;
mod result;

use crate::{BinaryDiff, BinaryDiffChunk};
use error::BinaryDiffAnalyzerError;
use result::Result;
use std::io::{BufReader, Read, Seek, SeekFrom};

pub struct BinaryDiffAnalyzer<'a, R: Read + Seek> {
    diff: &'a BinaryDiff,
    patched: BufReader<R>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DerivesFrom<'a> {
    position: Option<usize>,
    chunk: &'a BinaryDiffChunk,
}

impl<'a, R> BinaryDiffAnalyzer<'a, R>
where
    R: Read + Seek,
{
    pub fn new(diff: &'a BinaryDiff, patched: R) -> Self {
        Self {
            diff,
            patched: BufReader::new(patched),
        }
    }

    pub fn derives_from(&mut self, offset: usize) -> Result<Option<DerivesFrom<'a>>> {
        let value = {
            self.patched
                .seek(SeekFrom::Start(offset as u64))
                .map_err(BinaryDiffAnalyzerError::IoError)?;
            let mut value = [0u8; 1];
            self.patched
                .read_exact(&mut value)
                .map_err(BinaryDiffAnalyzerError::IoError)?;
            value[0]
        };

        Ok(_derives_from(&self.diff, offset, value))
    }
}

fn _derives_from(diff: &BinaryDiff, new_offset: usize, value: u8) -> Option<DerivesFrom> {
    let mut applied_new_offset = 0usize;
    for chunk in diff.chunks().iter() {
        match chunk {
            BinaryDiffChunk::Same(old_offset, length) => {
                if (applied_new_offset..(applied_new_offset + length)).contains(&new_offset) {
                    return Some(DerivesFrom {
                        position: Some(old_offset + new_offset - applied_new_offset),
                        chunk,
                    });
                }
            }
            BinaryDiffChunk::Insert(_, bytes) | BinaryDiffChunk::Replace(_, _, bytes) => {
                if (applied_new_offset..(applied_new_offset + bytes.len())).contains(&new_offset) {
                    return if value == bytes[new_offset - applied_new_offset] {
                        Some(DerivesFrom {
                            position: None,
                            chunk,
                        })
                    } else {
                        None
                    };
                }
            }
            // NOTE: Delete() chunk does not affect patched files
            BinaryDiffChunk::Delete(_, _) => (),
        }

        applied_new_offset += chunk.patched_length()
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Delete, Insert, Replace, Same};
    use crate::binary_diff_analyzer::{BinaryDiffAnalyzer, DerivesFrom, _derives_from};
    use crate::BinaryDiff;
    use std::io::Cursor;

    #[test]
    fn test_binary_diff_analyzer() {
        let chunk = Insert(0, vec![0, 1, 2, 3]);
        let diff = BinaryDiff::from(&vec![chunk.clone()]);
        let buf = Cursor::new(vec![0, 1, 2, 3]);
        let mut analyzer = BinaryDiffAnalyzer::new(&diff, buf);
        match analyzer.derives_from(1) {
            Ok(result) => assert_eq!(
                result,
                Some(DerivesFrom {
                    position: None,
                    chunk: &chunk
                })
            ),
            Err(why) => assert!(false, "{:?}", why),
        }
    }

    #[test]
    fn test_derives_from_none() {
        let diff = BinaryDiff::from(&vec![Same(0, 4)]);
        assert_eq!(_derives_from(&diff, 5, 0), None);
    }

    #[test]
    fn test_derives_from_delete_same() {
        let chunk = Same(6, 2);
        let diff = BinaryDiff::from(&vec![Same(0, 4), Delete(4, 2), chunk.clone()]);
        assert_eq!(
            _derives_from(&diff, 4, 0),
            Some(DerivesFrom {
                position: Some(6),
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 5, 0),
            Some(DerivesFrom {
                position: Some(7),
                chunk: &chunk
            })
        );
        assert_eq!(_derives_from(&diff, 6, 0), None);
    }

    #[test]
    fn test_derives_from_insert() {
        let chunk = Insert(0, vec![0, 1, 2, 3]);
        let diff = BinaryDiff::from(&vec![chunk.clone()]);
        assert_eq!(
            _derives_from(&diff, 0, 0),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 1, 1),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 2, 2),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 3, 3),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(_derives_from(&diff, 4, 4), None);
    }

    #[test]
    fn test_derives_from_replace() {
        let chunk = Replace(4, 2, vec![0, 1, 2, 3]);
        let diff = BinaryDiff::from(&vec![Same(0, 4), chunk.clone()]);
        assert_eq!(
            _derives_from(&diff, 4, 0),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 5, 1),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 6, 2),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 7, 3),
            Some(DerivesFrom {
                position: None,
                chunk: &chunk
            })
        );
        assert_eq!(_derives_from(&diff, 8, 0), None);
    }

    #[test]
    fn test_derives_from_replace_same() {
        let chunk = Same(6, 2);
        let diff = BinaryDiff::from(&vec![
            Same(0, 4),
            Replace(4, 2, vec![0, 1, 2, 3]),
            chunk.clone(),
        ]);
        assert_eq!(
            _derives_from(&diff, 8, 0),
            Some(DerivesFrom {
                position: Some(6),
                chunk: &chunk
            })
        );
        assert_eq!(
            _derives_from(&diff, 9, 0),
            Some(DerivesFrom {
                position: Some(7),
                chunk: &chunk
            })
        );
        assert_eq!(_derives_from(&diff, 10, 0), None);
    }
}
