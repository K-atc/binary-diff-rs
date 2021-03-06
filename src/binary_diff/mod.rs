use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
use crate::binary_diff::error::BinaryDiffError;
use result::Result;
use std::io::{BufReader, Read, Seek};
// use alloc::vec::Vec;

pub(crate) mod binary_diff_chunk;
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
    // BinaryDiff.chunks should satisfy following requirements:
    //   - Minimize the length of the return value
    //   - An chunk and its next one is NOT the same
    //   - Sorted `offset` of chunk(offset, ...) accenting
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
                log::trace!("Added {:?}", chunk);
                chunks.push(chunk);
            }
            if let Some(chunk) = get_delete_chunk(old, new, old_size, new_size)? {
                log::trace!("Added {:?}", chunk);
                chunks.push(chunk);
            }
            if let Some(chunk) = get_insert_chunk(old, new, old_size, new_size)? {
                log::trace!("Added {:?}", chunk);
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
                let err = Err(BinaryDiffError::InfiniteLoopError(
                    old_position as usize,
                    new_position as usize,
                ));
                log::error!("Detected infinite loop. There's a logic error: {:?}", err);
                return err;
            }
        }

        Ok(Self { chunks })
    }

    pub fn enhance(&self) -> Self {
        Self {
            chunks: introduce_replace_chunk(&self.chunks),
        }
    }

    pub fn from(unsorted_chunks: &Vec<BinaryDiffChunk>) -> Self {
        let mut chunks = unsorted_chunks.to_vec();
        chunks.sort();
        Self { chunks }
    }

    pub fn chunks(&self) -> &Vec<BinaryDiffChunk> {
        &self.chunks
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
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
                BinaryDiff::from(&vec![Delete(0, 2), Insert(2, vec![0, 3]), Same(2, 2),])
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

    #[test]
    fn test_lcs_appears_far() {
        // A real world example revealed a bug
        init();

        let old = vec![
            0x00, 0x10, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0xb7, 0x00, 0x30,
        ];
        let new = vec![
            0x00, 0x2e, 0x03, 0x00, 0x00, 0x03, 0xfe, 0xe3, 0xe3, 0x2e, 0x03, 0x00, 0x00, 0x00,
            0xb7, 0x00, 0x30,
        ];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![
                    Same(0, 1),
                    Delete(1, 1),
                    Insert(2, vec![0x2e]),
                    Same(2, 3),
                    Insert(5, vec![0x03, 0xfe, 0xe3, 0xe3, 0x2e, 0x03]),
                    Same(5, 6),
                ])
            );
        }
    }

    #[test]
    fn test_chunks_same_delete_same_delete_insert_same() {
        init();

        // From real world samples
        let old = vec![
            //                                              8
            0x2e, 0x03, 0x00, 0x00, 0x03, 0xfe, 0xe3, 0xe3, 0x2e, 0x03, 0x00, 0x00, 0x00, 0x2e,
            //                                  ~~~~~~~~~~  ~~~~~~~~~~  ~~~~~~~~~~  ~~~~  ~~~~ Same
            //                                  Delete      Same        Same        Delete
            0x03, 0x00, 0x00, 0x03, 0xfe, 0xe3, 0xe3, 0x2e, 0x03, 0x00, 0x00, 0x00, 0xb7, 0x03,
            0x00, 0x00, 0x03, 0xfe,
        ];
        let new = vec![
            0x2e, 0x03, 0x00, 0x00, 0x03, 0xfe, 0x2e, 0x03, 0x18, 0x03, 0x18, 0x00, 0x00, 0x2e,
            //                                  ~~~~~~~~~~  ~~~~~~~~~~~~~~~~  ~~~~~~~~~~  ~~~~ Same
            //                                  Same        Insert            Same
            0x03, 0x00, 0x00, 0x03, 0xfe, 0xe3, 0xe3, 0x2e, 0x03, 0x00, 0x00, 0x00, 0xb7, 0x03,
            0x00, 0x00, 0x03, 0xfe,
        ];
        let diff_chunks = binary_diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                BinaryDiff::from(&vec![
                    Same(0x0, 0x6),
                    Delete(0x6, 0x2),
                    Same(0x8, 0x2),
                    Delete(0xa, 0x1),
                    Insert(0xb, vec![0x18]),
                    Insert(0xb, vec![0x03, 0x18]),
                    Same(0xb, 21),
                ])
            );
        }
    }

    #[test]
    fn test_crash_minimization() {
        init();

        // From real world samples
        let old = vec![
            0x5c, 0x53, 0x3f, 0x5c, 0x43, 0x5c, 0x53, 0x3f, 0x5c, 0x43, 0xd5, 0xac, 0x32, 0x2a,
            //~~                    ~~~~~~~~~~~~~~~~~~~~~~              ~~~~~~~~~~
            0xd5, 0xac, 0x43, 0x5c, 0x53, 0x16,
            //                ~~~~        ~~~~
        ];
        let new = vec![0x5c, 0x43, 0x5c, 0x53, 0x3f, 0xd5, 0xac, 0x16, 0x5c, 0x16];
        //                     ~~~~~  ~~~~~~~~~~~~~~~~~~~~~~  ~~~~~~~~~~
        let diff_chunks = binary_diff_wrapper(&old, &new).unwrap();
        for chunk in diff_chunks.chunks.iter() {
            log::trace!("{}", chunk);
        }
        assert_eq!(
            diff_chunks,
            BinaryDiff::from(&vec![
                Same(0x0, 01),
                Delete(0x1, 3),
                Same(0x4, 4),
                Delete(0x8, 2),
                Same(0xa, 2),
                Delete(0xc, 5),
                Delete(0x11, 0x2),
                Same(0x13, 1),
                Insert(0x14, vec![0x5c, 0x16])
            ])

            // NOTE: Less Insert() is preferred
            // BinaryDiff::from(&vec![
            //     Same(0x0, 01),
            //     Delete(0x1, 3),
            //     Same(0x4, 4),
            //     Delete(0x8, 2),
            //     Same(0xa, 2),
            //     Delete(0xc, 5),
            //     Insert(0x11, vec![0x16]),
            //     Same(0x11, 1),
            //     Delete(0x12, 1),
            //     Same(0x13, 1),
            // ])
        );
    }

    #[test]
    fn realworld_sample() {
        let original = vec![
            0x7B, 0x31, 0x12, 0x00, 0x00, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0xB6,
            0x00,
            0x8C, 0xC9, 0x01, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0xBB, 0x00,
            0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x47, 0x00, 0x03, 0x01,
            0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x75, 0x87, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0xAD, 0x0A, 0x01, 0x03, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x58, 0x0D, 0x01, 0x02, 0x00, 0x0F, 0x00, 0x00, 0x00,
            0x40, 0x01, 0x00, 0x00, 0x11, 0x01, 0x04, 0x00, 0x01, 0x00, 0x04, 0x9D,
        ];
        let edited = vec![
            0xFE, 0x00, 0x12, 0x00, 0x00, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20,
            0x00,
            0xA5, 0xC3, 0x01, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0xA6, 0x00,
            0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x68, 0x00, 0x03, 0x01,
            0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x63, 0x0A, 0x01, 0x03, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0xB0, 0x0D, 0x01, 0x02, 0x00, 0x0F, 0x00, 0x00, 0x00,
            0x40, 0x01, 0x00, 0x00, 0x11, 0x01, 0x04, 0x00, 0x01, 0x00, 0x31, 0x00,
        ];
        let diff_chunks = binary_diff_wrapper(&original, &edited).unwrap();
        for chunk in diff_chunks.chunks.iter() {
            log::debug!("{}", chunk);
        }
        // assert_eq!(diff_chunks, BinaryDiff::from(&vec![]));
    }
}
