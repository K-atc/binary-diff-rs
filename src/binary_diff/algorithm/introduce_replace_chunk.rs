use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk;

#[derive(Debug, Eq, PartialEq)]
enum LoopFlag {
    Default,
    Skip,
}

pub fn introduce_replace_chunk(original_chunks: &Vec<BinaryDiffChunk>) -> Vec<BinaryDiffChunk> {
    let mut enhanced_chunks = vec![];
    let mut loop_flag = LoopFlag::Default; // TODO: Dirty. Do refactoring to use Reader<R: Read + Seek>
    for (i, chunk) in original_chunks.iter().enumerate() {
        if loop_flag == LoopFlag::Skip {
            loop_flag = LoopFlag::Default;
            continue;
        }

        if i < original_chunks.len() - 1 {
            if let BinaryDiffChunk::Delete(offset, length) = &chunk {
                if let BinaryDiffChunk::Insert(_, bytes) = &original_chunks[i + 1] {
                    enhanced_chunks.push(BinaryDiffChunk::Replace(
                        offset.clone(),
                        length.clone(),
                        bytes.clone(),
                    ));
                    loop_flag = LoopFlag::Skip;
                    continue;
                }
            }
        }

        enhanced_chunks.push(chunk.clone())
    }
    enhanced_chunks
}

#[cfg(test)]
mod tests {
    use super::introduce_replace_chunk;
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Delete, Insert, Replace, Same};

    #[test]
    fn test_introduce_replace_chunk() {
        let original = vec![
            Same(0, 1),
            Delete(1, 3),
            Same(4, 1),
            Delete(5, 4),
            Insert(8, vec![0, 0]),
        ];
        let enhanced = introduce_replace_chunk(&original);
        assert_eq!(
            enhanced,
            vec![
                original[0].clone(),
                original[1].clone(),
                original[2].clone(),
                Replace(5, 4, vec![0, 0])
            ]
        )
    }

    #[test]
    fn test_introduce_replace_chunk_without_replace() {
        let original = vec![Same(0, 1), Delete(1, 3)];
        let enhanced = introduce_replace_chunk(&original);
        assert_eq!(enhanced, original)
    }
}
