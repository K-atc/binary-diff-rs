pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    for (i, _) in haystack.iter().enumerate() {
        if haystack[i..haystack.len()].starts_with(needle) {
            return Some(i);
        }
    }
    None
}

// fn find<R: Read + Seek>(
//     reader: &mut BufReader<R>,
//     bytes: &[u8],
//     window: usize,
// ) -> Result<Option<usize>> {
//     let original_position = reader.stream_position().map_err(BinaryDiffError::IoError)?;
//
//     // NOTE: windows mut be equal to or smaller than remaining buffer
//     let mut buf = vec![];
//     buf.resize(window, 0u8); // Apply window size
//     reader
//         .read_exact(&mut buf)
//         .map_err(BinaryDiffError::IoError)?;
//
//     reader
//         .seek(SeekFrom::Start(original_position))
//         .map_err(BinaryDiffError::IoError)?;
//
//     for i in 0..window {
//         if buf[i..buf.len()].starts_with(bytes) {
//             return Ok(Some(i));
//         }
//     }
//     Ok(None)
// }
