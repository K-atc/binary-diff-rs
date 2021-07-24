use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
use crate::binary_diff::error::BinaryDiffError;
use bcmp::{longest_common_substring, AlgoSpec};
use result::Result;
use std::cmp::min;
use std::io::{BufReader, Read, Seek, SeekFrom};
// use alloc::vec::Vec;

pub mod binary_diff_chunk;
pub mod error;
pub mod result;

fn get_buffer_length<R: Seek>(reader: &mut BufReader<R>) -> Result<usize> {
    let size = reader
        .seek(SeekFrom::End(0))
        .map_err(BinaryDiffError::IoError)?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(BinaryDiffError::IoError)?;
    Ok(size as usize)
}

fn read_one_byte<R: Read + Seek>(reader: &mut BufReader<R>) -> Result<[u8; 1]> {
    let mut buf = [0u8];
    reader
        .read_exact(&mut buf)
        .map_err(BinaryDiffError::IoError)?;
    Ok(buf)
}

// fn find<R: Read + Seek>(
//     reader: &mut BufReader<R>,
//     bytes: &[u8],
//     window: usize,
// ) -> Result<Option<usize>> {
//     let original_position = reader.stream_position().map_err(BinaryDiffError::IoError)?;
//
//     // NOTE: windows mut be equal to or smaller than remaining buffer
//     let mut buf = vec![];
//     buf.resize(window, 0u8); // Apply window size
//     reader
//         .read_exact(&mut buf)
//         .map_err(BinaryDiffError::IoError)?;
//
//     reader
//         .seek(SeekFrom::Start(original_position))
//         .map_err(BinaryDiffError::IoError)?;
//
//     for i in 0..window {
//         if buf[i..buf.len()].starts_with(bytes) {
//             return Ok(Some(i));
//         }
//     }
//     Ok(None)
// }

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    for (i, _) in haystack.iter().enumerate() {
        if haystack[i..haystack.len()].starts_with(needle) {
            return Some(i);
        }
    }
    None
}

fn read_bytes<R: Read + Seek>(reader: &mut BufReader<R>, length: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; length];
    reader
        .read_exact(&mut buf.as_mut_slice())
        .map_err(BinaryDiffError::IoError)?;
    debug_assert_eq!(buf.len(), length);
    Ok(buf)
}

// get_same_chunk() should satisfy following requirements:
//   - Maximize `length` of Same(offset, length)
fn get_same_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old.stream_position().map_err(BinaryDiffError::IoError)? as usize;

    #[allow(non_snake_case)]
    let N = min(
        old_size - offset,
        new_size - new.stream_position().map_err(BinaryDiffError::IoError)? as usize,
    );
    log::trace!("[*] get_same_chunk():   offset = {}, N = {}", offset, N);

    if N == 0 {
        return Ok(None);
    }

    for i in 0usize..N {
        let old_buf = read_one_byte(old)?;
        let new_buf = read_one_byte(new)?;

        if old_buf != new_buf {
            old.seek_relative(-1).map_err(BinaryDiffError::IoError)?;
            new.seek_relative(-1).map_err(BinaryDiffError::IoError)?;

            return if i > 0 {
                Ok(Some(BinaryDiffChunk::Same(offset, i)))
            } else {
                Ok(None)
            };
        }
    }

    Ok(Some(BinaryDiffChunk::Same(offset, N)))
}

// get_delete_chunk() should satisfy following requirements:
//   - Minimize `length` of Delete(offset, length)
fn get_delete_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old.stream_position().map_err(BinaryDiffError::IoError)? as usize;

    let new_position = new.stream_position().map_err(BinaryDiffError::IoError)? as usize;

    if new_position == new_size {
        // Remaining bytes in `old` might be deleted
        old.seek(SeekFrom::End(0))
            .map_err(BinaryDiffError::IoError)?;
        let length = old_size - offset;
        return if length > 0 {
            Ok(Some(BinaryDiffChunk::Delete(offset, length)))
        } else {
            Ok(None)
        };
    }

    #[allow(non_snake_case)]
    let N = min(old_size - offset, new_size - new_position);
    log::trace!("[*] get_delete_chunk(): offset = {}, N = {}", offset, N);

    if N > 0 {
        let window = min(32, new_size - new_position);

        let new_bytes_in_window = read_bytes(new, window)?;
        new.seek_relative(-(window as i64))
            .map_err(BinaryDiffError::IoError)?;
        let old_buf = read_bytes(old, N)?;

        // Find offset that minimizes `offset` of next Same(offset, length)
        if let Some((next_same_offset, _)) = (0..N)
            .map(|i| (i, find(new_bytes_in_window.as_slice(), &[old_buf[i]])))
            .filter(|(_, v)| v.is_some())
            .min_by_key(|(_, v)| v.clone())
        {
            old.seek_relative(-(N as i64) + next_same_offset as i64)
                .map_err(BinaryDiffError::IoError)?;
            return if next_same_offset > 0 {
                Ok(Some(BinaryDiffChunk::Delete(offset, next_same_offset)))
            } else {
                Ok(None)
            };
        }

        Ok(Some(BinaryDiffChunk::Delete(offset, N)))
    } else {
        Ok(None)
    }
}

// get_insert_chunk() should satisfy following requirements:
//   - Maximize length of `bytes` of Insert(offset, bytes)
fn get_insert_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old.stream_position().map_err(BinaryDiffError::IoError)? as usize;
    #[allow(non_snake_case)]
    let N = new_size - new.stream_position().map_err(BinaryDiffError::IoError)? as usize;
    let window = min(N, 16);
    log::trace!("[*] get_insert_chunk(): offset = {}, N = {}, window = {}", offset, N, window);

    if N == 0 {
        return Ok(None);
    }

    if offset < old_size {
        let old_bytes = read_bytes(old, min(window, old_size - offset))?;
        let new_bytes = read_bytes(new, window)?;

        let lcs = longest_common_substring(
            old_bytes.as_slice(),
            new_bytes.as_slice(),
            AlgoSpec::HashMatch(1),
        );
        log::trace!("[*] get_insert_chunk(): old_bytes = {:?}", old_bytes);
        log::trace!("[*] get_insert_chunk(): new_bytes = {:?}", new_bytes);
        log::trace!("[*] get_insert_chunk(): lcs = {:?}", lcs);
        assert_eq!(
            lcs.first_pos, 0,
            "There must be bytes deleted beforehand in `old`"
        );

        old.seek_relative(-(old_bytes.len() as i64))
            .map_err(BinaryDiffError::IoError)?;
        new.seek_relative(-(new_bytes.len() as i64) + (lcs.second_pos as i64))
            .map_err(BinaryDiffError::IoError)?;

        if lcs.length > 0 {
            if lcs.second_pos > 0 {
                Ok(Some(BinaryDiffChunk::Insert(
                    offset,
                    new_bytes[0..lcs.second_pos].to_vec(),
                )))
            } else {
                // This is case of old_bytes[0..k] == new_bytes[0..k]
                debug_assert_eq!(old_bytes[0..lcs.length], new_bytes[0..lcs.length]);
                log::trace!("[*] get_insert_chunk(): old_bytes[0..k] == new_bytes[0..k]");
                Ok(None)
            }
        } else {
            let old_next_byte = read_one_byte(old)?;
            old.seek_relative(-1).map_err(BinaryDiffError::IoError)?;
            let mut inserted_bytes = vec![];

            // `new_bytes` does not have same bytes in `old_bytes`, so insert bytes until a byte appears in old
            for _ in 0usize..N {
                let new_byte = read_one_byte(new)?;
                if new_byte == old_next_byte {
                    new.seek_relative(-1).map_err(BinaryDiffError::IoError)?;
                    break;
                }
                inserted_bytes.extend_from_slice(&new_byte);
            }

            if inserted_bytes.len() > 0 {
                Ok(Some(BinaryDiffChunk::Insert(offset, inserted_bytes)))
            } else {
                // inserted_bytes.len() must be larger than 0 since N > 0, but fail safe
                Ok(None)
            }
        }
    } else {
        // Remaining bytes in `new` might be inserted
        log::trace!("[*] get_insert_chunk(): Remaining bytes in `new` might be inserted");
        let mut inserted_bytes = vec![];
        new.read_to_end(&mut inserted_bytes)
            .map_err(BinaryDiffError::IoError)?;
        if inserted_bytes.len() > 0 {
            Ok(Some(BinaryDiffChunk::Insert(offset, inserted_bytes)))
        } else {
            // inserted_bytes.len() must be larger than 0 since N > 0, but fail safe
            Ok(None)
        }
    }
}

// binary_diff() should satisfy following requirements:
//   - Minimize the length of the return value
//   - An item and its next one is NOT the same
pub fn binary_diff<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
) -> Result<Vec<BinaryDiffChunk>> {
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

        if old.stream_position().map_err(BinaryDiffError::IoError)? == old_size as u64
            && new.stream_position().map_err(BinaryDiffError::IoError)? == new_size as u64
        {
            break;
        }

        if (old_position, new_position)
            == (
                old.stream_position().map_err(BinaryDiffError::IoError)?,
                new.stream_position().map_err(BinaryDiffError::IoError)?,
            )
        {
            return Err(BinaryDiffError::InfiniteLoopError(
                old_position as usize,
                new_position as usize,
            ));
        }
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Delete, Insert, Same};
    use crate::binary_diff::binary_diff;
    use crate::binary_diff::result::Result;
    use std::io::{BufReader, Cursor};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn diff_wrapper(old: &Vec<u8>, new: &Vec<u8>) -> Result<Vec<BinaryDiffChunk>> {
        binary_diff(
            &mut BufReader::new(Cursor::new(old)),
            &mut BufReader::new(Cursor::new(new)),
        )
    }

    #[test]
    fn test_chunks_same() {
        init();

        let old = vec![0, 1, 2, 3];
        let new = vec![0, 1, 2, 3];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, vec![Same(0, 4)]);
        }
    }

    #[test]
    fn test_chunks_same_delete() {
        init();

        let old = vec![0, 1, 2, 3];
        let new = vec![0, 1];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, vec![Same(0, 2), Delete(2, 2)]);
        }
    }

    #[test]
    fn test_chunks_same_insert() {
        init();

        let old = vec![0, 1];
        let new = vec![0, 1, 2, 3];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Same(0, 2), Insert(2, new[2..=3].to_vec())]
            );
        }
    }

    #[test]
    fn test_chunks_same_insert_same() {
        init();

        let old = vec![0x00, 0x0b, 0x01, 0x00, 0x03, 0xfe, 0x00, 0x03];
        let new = vec![0x00, 0x0b, 0x01, 0xfd, 0x03, 0xfe, 0x00, 0x03];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![
                    Same(0, 3),
                    Delete(3, 1),
                    Insert(4, new[3..=3].to_vec()),
                    Same(4, 4)
                ]
            );
        }
    }

    #[test]
    fn test_chunks_delete() {
        init();

        let old = vec![0, 1];
        let new = vec![];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, vec![Delete(0, 2)]);
        }
    }

    #[test]
    fn test_chunks_delete_insert() {
        init();

        let old = vec![0, 1];
        let new = vec![2, 3];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Delete(0, 2), Insert(2, new[0..=1].to_vec())]
            );
        }
    }

    #[test]
    fn test_chunks_delete_insert_same() {
        init();

        let old = vec![0, 1, 4];
        let new = vec![2, 3, 4];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Delete(0, 2), Insert(2, new[0..=1].to_vec()), Same(2, 1)]
            );
        }
    }

    #[test]
    fn test_chunks_delete_insert_same_subset() {
        init();

        let old = vec![1, 2, 0, 0];
        let new = vec![0, 3, 0, 0];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Delete(0, 2), Insert(2, new[0..=1].to_vec()), Same(2, 2)]
            );
        }
    }

    #[test]
    fn test_chunks_delete_same_insert() {
        init();

        let old = vec![0, 1, 2];
        let new = vec![2, 3, 4];
        let diff_chunks = diff_wrapper(&old, &new);
        log::trace!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Delete(0, 2), Same(2, 1), Insert(3, new[1..=2].to_vec())]
            );
        }
    }
}
