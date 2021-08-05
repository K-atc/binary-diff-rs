use super::super::binary_diff_chunk::BinaryDiffChunk;
use super::super::error::BinaryDiffError;
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
        for window in [4, 8, 16, 32, 64] {
            // Find offset that minimizes `offset` of next Same(offset, length)

            let old_window = min(window, old_size - offset);
            let new_window = min(window, new_size - new_position);
            if old_window == 0 || new_window == 0 {
                break;
            }

            let old_bytes = read_bytes(old, old_window)?;
            let new_bytes = read_bytes(new, new_window)?;

            let lcs = longest_common_substring(
                old_bytes.as_slice(),
                new_bytes.as_slice(),
                AlgoSpec::HashMatch(1),
            );
            log::trace!("old_bytes = {:?}", old_bytes);
            log::trace!("new_bytes = {:?}", new_bytes);
            log::trace!("lcs = {:?}", lcs);

            // Restore original position
            old.seek_relative(-(old_bytes.len() as i64))
                .map_err(BinaryDiffError::IoError)?;
            new.seek_relative(-(new_bytes.len() as i64))
                .map_err(BinaryDiffError::IoError)?;

            if lcs.length > 0 {
                return if lcs.first_pos > 0 {
                    old.seek_relative(lcs.first_pos as i64)
                        .map_err(BinaryDiffError::IoError)?;
                    Ok(Some(BinaryDiffChunk::Delete(offset, lcs.first_pos)))
                } else {
                    // Nothing to be deleted
                    Ok(None)
                };
            }
        }

        // Delete bytes until same byte appears
        let new_byte = read_bytes(new, 1)?;
        new.seek_relative(-(new_byte.len() as i64))
            .map_err(BinaryDiffError::IoError)?;
        for i in offset..old_size {
            let old_byte = read_bytes(old, 1)?;
            if old_byte == new_byte {
                old.seek_relative(-(old_byte.len() as i64))
                    .map_err(BinaryDiffError::IoError)?;
                return Ok(Some(BinaryDiffChunk::Delete(i, i - offset)));
            }
        }

        // Remaining all bytes to be deleted
        Ok(Some(BinaryDiffChunk::Delete(offset, old_size - offset)))
    } else {
        Ok(None)
    }
}
