use crate::binary_diff::error::BinaryDiffError;
use crate::binary_diff::result::Result;
use std::io::{BufReader, Seek, SeekFrom};

pub fn get_buffer_length<R: Seek>(reader: &mut BufReader<R>) -> Result<usize> {
    let size = reader
        .seek(SeekFrom::End(0))
        .map_err(BinaryDiffError::IoError)?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(BinaryDiffError::IoError)?;
    Ok(size as usize)
}
