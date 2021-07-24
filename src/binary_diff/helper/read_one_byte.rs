use crate::binary_diff::error::BinaryDiffError;
use crate::binary_diff::result::Result;
use std::io::{BufReader, Read, Seek};

pub fn read_one_byte<R: Read + Seek>(reader: &mut BufReader<R>) -> Result<[u8; 1]> {
    let mut buf = [0u8];
    reader
        .read_exact(&mut buf)
        .map_err(BinaryDiffError::IoError)?;
    Ok(buf)
}
