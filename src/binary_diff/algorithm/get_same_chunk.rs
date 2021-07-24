use super::super::binary_diff_chunk::BinaryDiffChunk;
use super::super::error::BinaryDiffError;
use super::super::result::Result;
use crate::binary_diff::helper::read_one_byte;
use std::cmp::min;
use std::io::{BufReader, Read, Seek};

// get_same_chunk() should satisfy following requirements:
//   - Maximize `length` of Same(offset, length)
pub fn get_same_chunk<R: Read + Seek>(
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
