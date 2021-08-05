use super::super::binary_diff_chunk::BinaryDiffChunk;
use super::super::error::BinaryDiffError;
use super::super::helper::{read_bytes, read_one_byte};
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
    let window = min(N, 16);
    log::trace!("offset = {}, N = {}, window = {}", offset, N, window);

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
        log::trace!("old_bytes = {:?}", old_bytes);
        log::trace!("new_bytes = {:?}", new_bytes);
        log::trace!("lcs = {:?}", lcs);

        // Restore original position
        old.seek_relative(-(old_bytes.len() as i64))
            .map_err(BinaryDiffError::IoError)?;
        new.seek_relative(-(new_bytes.len() as i64))
            .map_err(BinaryDiffError::IoError)?;

        if lcs.first_pos == 0 {
            if lcs.length > 0 {
                if lcs.second_pos > 0 {
                    new.seek_relative(lcs.second_pos as i64)
                        .map_err(BinaryDiffError::IoError)?;
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
            // Insert bytes until same byte appears on old
            for (i, new_byte) in new_bytes.iter().enumerate() {
                if new_byte == &old_bytes[0] {
                    return if i > 0 {
                        new.seek_relative(i as i64)
                            .map_err(BinaryDiffError::IoError)?;
                        Ok(Some(BinaryDiffChunk::Insert(
                            offset,
                            new_bytes[0..i].to_vec(),
                        )))
                    } else {
                        // Should generate same chunk
                        Ok(None)
                    };
                }
            }
            unreachable!("LCS exists both in old_bytes and new_bytes, so this is unreachable")
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
