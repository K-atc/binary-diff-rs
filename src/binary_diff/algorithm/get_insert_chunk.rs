use super::super::binary_diff_chunk::BinaryDiffChunk;
use super::super::error::BinaryDiffError;
use super::super::helper::{read_bytes};
use super::super::result::Result;
use bcmp::{longest_common_substring, AlgoSpec};
use std::cmp::min;
use std::io::{BufReader, Read, Seek};

// get_insert_chunk() should satisfy following requirements:
//   - Maximize length of `bytes` of Insert(offset, bytes)
pub fn get_insert_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old.stream_position().map_err(BinaryDiffError::IoError)? as usize;
    #[allow(non_snake_case)]
    let N = new_size - new.stream_position().map_err(BinaryDiffError::IoError)? as usize;
    log::trace!("offset = {}, N = {}", offset, N);

    if N == 0 {
        return Ok(None);
    }

    if offset < old_size {
        for window in [4, 8, 16, 32, 64] {
            let old_bytes = read_bytes(old, min(window, old_size - offset))?;
            let new_bytes = read_bytes(new, min(window, N))?;

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
                return if lcs.second_pos > 0 {
                    new.seek_relative(lcs.second_pos as i64)
                        .map_err(BinaryDiffError::IoError)?;
                    Ok(Some(BinaryDiffChunk::Insert(
                        offset,
                        new_bytes[0..lcs.second_pos].to_vec(),
                    )))
                } else {
                    Ok(None)
                };
            }
        }

        let mut bytes = vec![];
        let old_byte = read_bytes(old, 1)?;
        old.seek_relative(-(old_byte.len() as i64))
            .map_err(BinaryDiffError::IoError)?;

        // Insert bytes until same byte appears on old
        for i in 0..N {
            let new_byte = read_bytes(new, 1)?;
            if new_byte == old_byte {
                return if i > 0 {
                    new.seek_relative(-(new_byte.len() as i64))
                        .map_err(BinaryDiffError::IoError)?;
                    Ok(Some(BinaryDiffChunk::Insert(offset, bytes)))
                } else {
                    // Should generate same chunk
                    Ok(None)
                };
            }
            bytes.push(new_byte[0])
        }

        Ok(Some(BinaryDiffChunk::Insert(offset, bytes)))
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
