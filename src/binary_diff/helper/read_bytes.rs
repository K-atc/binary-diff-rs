use crate::binary_diff::error::BinaryDiffError;
use crate::binary_diff::result::Result;
use std::io::{BufReader, Read, Seek};

pub fn read_bytes<R: Read + Seek>(reader: &mut BufReader<R>, length: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; length];
    reader
        .read_exact(&mut buf.as_mut_slice())
        .map_err(BinaryDiffError::IoError)?;
    debug_assert_eq!(buf.len(), length);
    Ok(buf)
}
