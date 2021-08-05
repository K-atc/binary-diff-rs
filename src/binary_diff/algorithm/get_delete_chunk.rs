use super::super::binary_diff_chunk::BinaryDiffChunk;
use super::super::error::BinaryDiffError;
use super::super::helper::find;
use super::super::helper::read_bytes;
use super::super::result::Result;
use bcmp::{longest_common_substring, AlgoSpec};
use std::cmp::min;
use std::io::{BufReader, Read, Seek, SeekFrom};

// get_delete_chunk() should satisfy following requirements:
//   - Minimize `length` of Delete(offset, length)
pub fn get_delete_chunk<R: Read + Seek>(
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
    log::trace!("offset = {}, N = {}", offset, N);

    if N > 0 {
        // NOTE: More values of the window are fine grained, more Delete() chunk become precise.
        for window in [4, 6, 8, 16, 32, 64] {
            // Find offset that minimizes `offset` of next Same(offset, length)

            let old_window = min(window, old_size - offset);
            let new_window = min(window, new_size - new_position);
            if old_window == 0 || new_window == 0 {
                break;
            }

            let old_bytes = read_bytes(old, old_window)?;
            let new_bytes = read_bytes(new, new_window)?;

            // Restore original position
            old.seek_relative(-(old_bytes.len() as i64))
                .map_err(BinaryDiffError::IoError)?;
            new.seek_relative(-(new_bytes.len() as i64))
                .map_err(BinaryDiffError::IoError)?;

            if window >= 8 {
                // Algorithm (1): For wide window
                let lcs = longest_common_substring(
                    old_bytes.as_slice(),
                    new_bytes.as_slice(),
                    AlgoSpec::HashMatch(1),
                );
                log::trace!("old_bytes = {:?}", old_bytes);
                log::trace!("new_bytes = {:?}", new_bytes);
                log::trace!("lcs = {:?}", lcs);

                if lcs.length > 0 {
                    return if lcs.first_pos > 0 {
                        old.seek_relative(lcs.first_pos as i64)
                            .map_err(BinaryDiffError::IoError)?;
                        Ok(Some(BinaryDiffChunk::Delete(offset, lcs.first_pos)))
                    } else {
                        // Nothing to be deleted
                        // Next chunk is Insert(offset, new_bytes[0..lcs.second_pos])
                        Ok(None)
                    };
                }
            } else {
                // Algorithm (2): For short window
                // NOTE: window=4 seems to be too short. LCS algorithm dismisses Same() chunk in the next of window

                let next_same_chunk_offset_map = (0..min(window, old_bytes.len()))
                    .map(|i| (i, find(new_bytes.as_slice(), &[old_bytes[i]])));

                let next_same_offset = {
                    let allowing_insert_chunk = next_same_chunk_offset_map
                        .clone()
                        .filter(|(_, v)| match v {
                            Some(v) => v > &0,
                            None => false,
                        })
                        .min_by_key(|(_, v)| v.clone());
                    let disallowing_insert_chunk = next_same_chunk_offset_map
                        .filter(|(_, v)| match v {
                            Some(v) => v == &0,
                            None => false,
                        })
                        .min_by_key(|(v, _)| v.clone());

                    // Determine next chunk by checking
                    // which next possible Insert() or Same() chunk is CLOSED to current Delete() chunk.
                    match (allowing_insert_chunk, disallowing_insert_chunk) {
                        // To decrease number of Insert() chunk
                        (_, Some((offset_if_disallowed, _))) => Some(offset_if_disallowed),
                        (Some((offset_if_allowed, _)), None) => Some(offset_if_allowed),
                        (None, None) => None,
                    }
                };

                match next_same_offset {
                    Some(next_same_offset) => return if next_same_offset > 0 {
                        old.seek_relative(next_same_offset as i64)
                            .map_err(BinaryDiffError::IoError)?;
                        Ok(Some(BinaryDiffChunk::Delete(offset, next_same_offset)))
                    } else {
                        // Next chunk is Insert(offset, new_bytes[0..min(find(...))])
                        Ok(None)
                    },
                    None => (), // Continue loop to check next window
                }
            }
        }

        // Algorithm (3): Delete bytes until same byte appears
        // NOTE: Next chunk CANNOT be Insert()
        let new_byte = read_bytes(new, 1)?;
        new.seek_relative(-(new_byte.len() as i64))
            .map_err(BinaryDiffError::IoError)?;
        for i in offset..old_size {
            let old_byte = read_bytes(old, 1)?;
            if old_byte == new_byte {
                old.seek_relative(-(old_byte.len() as i64))
                    .map_err(BinaryDiffError::IoError)?;
                return Ok(Some(BinaryDiffChunk::Delete(offset, i - offset)));
            }
        }

        // Remaining all bytes to be deleted
        Ok(Some(BinaryDiffChunk::Delete(offset, old_size - offset)))
    } else {
        Ok(None)
    }
}
