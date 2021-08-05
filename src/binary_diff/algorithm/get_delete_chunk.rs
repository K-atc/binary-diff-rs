use super::super::binary_diff_chunk::BinaryDiffChunk;
use super::super::error::BinaryDiffError;
use super::super::helper::{find, read_bytes};
use super::super::result::Result;
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
    log::trace!("[*] get_delete_chunk(): offset = {}, N = {}", offset, N);

    if N > 0 {
        let old_window = min(32, old_size - offset);
        let new_window = min(32, new_size - new_position);

        let old_bytes = read_bytes(old, old_window)?;
        old.seek_relative(-(old_bytes.len() as i64))
            .map_err(BinaryDiffError::IoError)?;
        let new_bytes = read_bytes(new, new_window)?;
        new.seek_relative(-(new_bytes.len() as i64))
            .map_err(BinaryDiffError::IoError)?;

        // Find offset that minimizes `offset` of next Same(offset, length)
        if let Some((next_same_offset, _)) = (0..old_window)
            .map(|i| (i, find(new_bytes.as_slice(), &[old_bytes[i]])))
            .filter(|(_, v)| v.is_some())
            .min_by_key(|(_, v)| v.clone())
        {
            old.seek_relative(next_same_offset as i64)
                .map_err(BinaryDiffError::IoError)?;
            return if next_same_offset > 0 {
                Ok(Some(BinaryDiffChunk::Delete(offset, next_same_offset)))
            } else {
                Ok(None)
            };
        }

        // Delete bytes until same byte appears
        let new_byte = read_bytes(new, 1)?;
        new.seek_relative(-(new_byte.len() as i64)).map_err(BinaryDiffError::IoError)?;
        for i in offset..old_size {
            let old_byte =  read_bytes(old, 1)?;
            if old_byte == new_byte {
                old.seek_relative(-(old_byte.len() as i64)).map_err(BinaryDiffError::IoError)?;
                return Ok(Some(BinaryDiffChunk::Delete(i, i - offset)))
            }
        }
        Ok(Some(BinaryDiffChunk::Delete(offset, old_size - offset)))
    } else {
        Ok(None)
    }
}
