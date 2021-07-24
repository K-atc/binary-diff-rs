use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
use crate::binary_diff::error::BinaryDiffError;
use result::Result;
use std::cmp::min;
use std::io::{BufReader, Read, Seek, SeekFrom};

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

fn contains<R: Read + Seek>(
    reader: &mut BufReader<R>,
    bytes: &[u8],
    window: usize,
) -> Result<bool> {
    let original_position = reader.stream_position().map_err(BinaryDiffError::IoError)?;

    // NOTE: windows mut be equal to or smaller than remaining buffer
    let mut buf = vec![];
    buf.resize(window, 0u8); // Apply window size
    reader
        .read_exact(&mut buf)
        .map_err(BinaryDiffError::IoError)?;

    reader
        .seek(SeekFrom::Start(original_position))
        .map_err(BinaryDiffError::IoError)?;

    Ok(bytes
        .iter()
        .fold(true, |result, v| result && buf.contains(v)))
}

// fn read_bytes<R: Read + Seek>(reader: &mut BufReader<R>, length: usize) -> Result<Vec<u8>> {
//     let mut buf = vec![0u8; length];
//     reader
//         .read_exact(&mut buf.as_mut_slice())
//         .map_err(BinaryDiffError::IoError)?;
//     Ok(buf)
// }

fn get_same_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old
        .stream_position()
        .map_err(BinaryDiffError::StreamPositionError)? as usize;

    #[allow(non_snake_case)]
    let N = min(
        old_size - offset,
        new_size
            - new
                .stream_position()
                .map_err(BinaryDiffError::StreamPositionError)? as usize,
    );
    println!("[*] get_same_chunk():   offset = {}, N = {}", offset, N);

    for i in 0usize..N {
        let old_buf = read_one_byte(old)?;
        let new_buf = read_one_byte(new)?;

        if old_buf != new_buf {
            old.seek_relative(-1).map_err(BinaryDiffError::IoError)?;
            new.seek_relative(-1).map_err(BinaryDiffError::IoError)?;

            return if i == 0 {
                Ok(None)
            } else {
                Ok(Some(BinaryDiffChunk::Same(offset, i)))
            };
        }
    }
    if N > 0 {
        Ok(Some(BinaryDiffChunk::Same(offset, N)))
    } else {
        Ok(None)
    }
}

fn get_delete_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old
        .stream_position()
        .map_err(BinaryDiffError::StreamPositionError)? as usize;

    let new_position = new
        .stream_position()
        .map_err(BinaryDiffError::StreamPositionError)? as usize;
    let window = min(32, new_size - new_position);

    #[allow(non_snake_case)]
    let N = min(old_size - offset, new_size - new_position);
    println!("[*] get_delete_chunk(): offset = {}, N = {}", offset, N);

    if N > 0 {
        for i in 0usize..N {
            if contains(new, &read_one_byte(old)?, window)? {
                old.seek_relative(-1).map_err(BinaryDiffError::IoError)?;
                return if i > 0 {
                    Ok(Some(BinaryDiffChunk::Delete(offset, i)))
                } else {
                    Ok(None)
                };
            }
        }
    }

    if new
        .stream_position()
        .map_err(BinaryDiffError::StreamPositionError)?
        == new_size as u64
    {
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

    if N > 0 {
        Ok(Some(BinaryDiffChunk::Delete(offset, N)))
    } else {
        Ok(None)
    }
}

fn get_insert_chunk<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
    old_size: usize,
    new_size: usize,
) -> Result<Option<BinaryDiffChunk>> {
    let offset = old
        .stream_position()
        .map_err(BinaryDiffError::StreamPositionError)? as usize;

    #[allow(non_snake_case)]
    let N = new_size
        - new
            .stream_position()
            .map_err(BinaryDiffError::StreamPositionError)? as usize;
    println!("[*] get_insert_chunk(): offset = {}, N = {}", offset, N);

    let mut inserted_bytes = vec![];

    if N > 0 {
        if offset < old_size {
            let old_next_byte = read_one_byte(old)?;
            old.seek_relative(-1)
                .map_err(BinaryDiffError::StreamPositionError)?;

            for _ in 0usize..N {
                let new_byte = read_one_byte(new)?;
                if new_byte == old_next_byte {
                    new.seek_relative(-1)
                        .map_err(BinaryDiffError::StreamPositionError)?;
                    break;
                }
                inserted_bytes.extend_from_slice(&new_byte);
            }
        } else {
            // Remaining bytes in `new` might be inserted
            println!("[*] get_insert_chunk(): Remaining bytes in `new` might be inserted");
            new.read_to_end(&mut inserted_bytes)
                .map_err(BinaryDiffError::IoError)?;
        }
    }

    if inserted_bytes.len() > 0 {
        Ok(Some(BinaryDiffChunk::Insert(offset, inserted_bytes)))
    } else {
        Ok(None)
    }
}

pub fn diff<R: Read + Seek>(
    old: &mut BufReader<R>,
    new: &mut BufReader<R>,
) -> Result<Vec<BinaryDiffChunk>> {
    let old_size = get_buffer_length(old)?;
    let new_size = get_buffer_length(new)?;
    println!("[*] old_size, new_size = {}, {}", old_size, new_size);

    let mut chunks = vec![];

    // Identify diff chunks using greedy algorithm
    loop {
        if let Some(chunk) = get_same_chunk(old, new, old_size, new_size)? {
            chunks.push(chunk);
        }
        if let Some(chunk) = get_delete_chunk(old, new, old_size, new_size)? {
            chunks.push(chunk);
        }
        if let Some(chunk) = get_insert_chunk(old, new, old_size, new_size)? {
            chunks.push(chunk);
        }

        if old
            .stream_position()
            .map_err(BinaryDiffError::StreamPositionError)?
            == old_size as u64
            && new
                .stream_position()
                .map_err(BinaryDiffError::StreamPositionError)?
                == new_size as u64
        {
            break;
        }
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Delete, Insert, Same};
    use crate::binary_diff::diff;
    use crate::binary_diff::result::Result;
    use std::io::{BufReader, Cursor};

    fn diff_wrapper(old: &Vec<u8>, new: &Vec<u8>) -> Result<Vec<BinaryDiffChunk>> {
        diff(
            &mut BufReader::new(Cursor::new(old)),
            &mut BufReader::new(Cursor::new(new)),
        )
    }

    #[test]
    fn test_chunks_same() {
        let old = vec![0, 1, 2, 3];
        let new = vec![0, 1, 2, 3];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, vec![Same(0, 4)]);
        }
    }

    #[test]
    fn test_chunks_same_delete() {
        let old = vec![0, 1, 2, 3];
        let new = vec![0, 1];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, vec![Same(0, 2), Delete(2, 2)]);
        }
    }

    #[test]
    fn test_chunks_same_insert() {
        let old = vec![0, 1];
        let new = vec![0, 1, 2, 3];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Same(0, 2), Insert(2, new[2..=3].to_vec())]
            );
        }
    }

    #[test]
    fn test_chunks_delete() {
        let old = vec![0, 1];
        let new = vec![];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(diff_chunks, vec![Delete(0, 2)]);
        }
    }

    #[test]
    fn test_chunks_delete_insert() {
        let old = vec![0, 1];
        let new = vec![2, 3];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
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
        let old = vec![0, 1, 4];
        let new = vec![2, 3, 4];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Delete(0, 2), Insert(2, new[0..=1].to_vec()), Same(2, 1)]
            );
        }
    }

    #[test]
    fn test_chunks_delete_same_insert() {
        let old = vec![0, 1, 2];
        let new = vec![2, 3, 4];
        let diff_chunks = diff_wrapper(&old, &new);
        println!("[*] diff() = {:?}", diff_chunks);
        assert!(diff_chunks.is_ok());
        if let Ok(diff_chunks) = diff_chunks {
            assert_eq!(
                diff_chunks,
                vec![Delete(0, 2), Same(2, 1), Insert(3, new[1..=2].to_vec())]
            );
        }
    }
}
