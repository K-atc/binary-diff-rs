use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
use crate::binary_diff::error::BinaryDiffError;
use result::Result;
use std::io::{BufReader, Read, Seek};
// use alloc::vec::Vec;

pub mod binary_diff_chunk;
pub mod error;
pub mod result;

// Internal use only
mod algorithm;
mod helper;

use algorithm::{get_delete_chunk, get_insert_chunk, get_same_chunk, introduce_replace_chunk};
use helper::get_buffer_length;

#[derive(Debug, Eq, PartialEq)]
pub struct BinaryDiff {
    chunks: Vec<BinaryDiffChunk>,
}

impl BinaryDiff {
    // BinaryDiff.chunk should satisfy following requirements:
    //   - Minimize the length of the return value
    //   - An item and its next one is NOT the same
    pub fn new<R: Read + Seek>(old: &mut BufReader<R>, new: &mut BufReader<R>) -> Result<Self> {
        let old_size = get_buffer_length(old)?;
        let new_size = get_buffer_length(new)?;
        log::trace!("[*] old_size, new_size = {}, {}", old_size, new_size);

        let mut chunks = vec![];

        // Identify diff chunks using greedy algorithm
        loop {
            let old_position = old.stream_position().map_err(BinaryDiffError::IoError)?;
            let new_position = new.stream_position().map_err(BinaryDiffError::IoError)?;

            if let Some(chunk) = get_same_chunk(old, new, old_size, new_size)? {
                chunks.push(chunk);
            }
            if let Some(chunk) = get_delete_chunk(old, new, old_size, new_size)? {
                chunks.push(chunk);
            }
            if let Some(chunk) = get_insert_chunk(old, new, old_size, new_size)? {
                chunks.push(chunk);
            }

            let (current_old_position, current_new_position) = (
                old.stream_position().map_err(BinaryDiffError::IoError)?,
                new.stream_position().map_err(BinaryDiffError::IoError)?,
            );
            // Seek to end of both of buffers, so exit
            if (old_size, new_size)
                == (current_old_position as usize, current_new_position as usize)
            {
                break;
            }
            // Infinite loop detection
            if (old_position, new_position) == (current_old_position, current_new_position) {
                return Err(BinaryDiffError::InfiniteLoopError(
                    old_position as usize,
                    new_position as usize,
                ));
            }
        }

        Ok(Self { chunks })
    }

    pub fn enhance(&self) -> Result<Self> {
        let mut enhanced_chunks = self.chunks.clone();

        introduce_replace_chunk(&mut enhanced_chunks);

        Ok(Self {
            chunks: enhanced_chunks,
        })
    }

    pub fn from(chunks: &Vec<BinaryDiffChunk>) -> Self {
        Self {
            chunks: chunks.to_vec(),
        }
    }

    pub fn chunks(&self) -> &Vec<BinaryDiffChunk> {
        &self.chunks
    }
}

#[cfg(test)]
mod tests {
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Delete, Insert, Same};
    use crate::binary_diff::result::Result;
    use crate::binary_diff::BinaryDiff;
    use std::io::{BufReader, Cursor};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn binary_diff_wrapper(old: &Vec<u8>, new: &Vec<u8>) -> Result<BinaryDiff> {
        BinaryDiff::new(
            &mut BufReader::new(Cursor::new(old)),
            &mut BufReader::new(Cursor::new(new)),
        )
    }

    #[test]
    fn test_chunks_same() {
        init();

        let old = vec![0, 1, 2, 3];
        let new = vec![0, 1, 2, 3];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, BinaryDiff::from(&vec![Same(0, 4)]));
        }
    }

    #[test]
    fn test_chunks_same_delete() {
        init();

        let old = vec![0, 1, 2, 3];
        let new = vec![0, 1];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![Same(0, 2), Delete(2, 2)])
            );
        }
    }

    #[test]
    fn test_chunks_same_insert() {
        init();

        let old = vec![0, 1];
        let new = vec![0, 1, 2, 3];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![Same(0, 2), Insert(2, new[2..=3].to_vec())])
            );
        }
    }

    #[test]
    fn test_chunks_same_insert_same() {
        init();

        let old = vec![0x00, 0x0b, 0x01, 0x00, 0x03, 0xfe, 0x00, 0x03];
        let new = vec![0x00, 0x0b, 0x01, 0xfd, 0x03, 0xfe, 0x00, 0x03];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![
                    Same(0, 3),
                    Delete(3, 1),
                    Insert(4, new[3..=3].to_vec()),
                    Same(4, 4)
                ])
            );
        }
    }

    #[test]
    fn test_chunks_delete() {
        init();

        let old = vec![0, 1];
        let new = vec![];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, BinaryDiff::from(&vec![Delete(0, 2)]));
        }
    }

    #[test]
    fn test_chunks_delete_insert() {
        init();

        let old = vec![0, 1];
        let new = vec![2, 3];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![Delete(0, 2), Insert(2, new[0..=1].to_vec())])
            );
        }
    }

    #[test]
    fn test_chunks_delete_insert_same() {
        init();

        let old = vec![0, 1, 4];
        let new = vec![2, 3, 4];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![
                    Delete(0, 2),
                    Insert(2, new[0..=1].to_vec()),
                    Same(2, 1)
                ])
            );
        }
    }

    #[test]
    fn test_chunks_delete_insert_same_subset() {
        init();

        let old = vec![1, 2, 0, 0];
        let new = vec![0, 3, 0, 0];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![
                    Delete(0, 2),
                    Insert(2, new[0..=1].to_vec()),
                    Same(2, 2)
                ])
            );
        }
    }

    #[test]
    fn test_chunks_delete_same_insert() {
        init();

        let old = vec![0, 1, 2];
        let new = vec![2, 3, 4];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![
                    Delete(0, 2),
                    Same(2, 1),
                    Insert(3, new[1..=2].to_vec())
                ])
            );
        }
    }
}
