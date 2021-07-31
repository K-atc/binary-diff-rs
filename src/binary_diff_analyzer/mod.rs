use crate::{BinaryDiff, BinaryDiffChunk};

pub struct BinaryDiffAnalyzer<'a> {
    diff: &'a BinaryDiff,
}

impl<'a> BinaryDiffAnalyzer<'a> {
    pub fn new(diff: &'a BinaryDiff) -> Self {
        Self { diff }
    }

    pub fn derives_from(&self, new_offset: usize, value: u8) -> Option<&BinaryDiffChunk> {
        let mut applied_new_offset = 0usize;
        for chunk in self.diff.chunks().iter() {
            match chunk {
                BinaryDiffChunk::Same(_, length) => {
                    if applied_new_offset <= new_offset && new_offset < applied_new_offset + length
                    {
                        return Some(chunk);
                    }
                }
                BinaryDiffChunk::Insert(_, bytes) => {
                    if applied_new_offset <= new_offset
                        && new_offset < applied_new_offset + bytes.len()
                    {
                        return if value == bytes[new_offset - applied_new_offset] {
                            Some(chunk)
                        } else {
                            None
                        };
                    }
                }
                BinaryDiffChunk::Replace(_, _, bytes) => {
                    if applied_new_offset <= new_offset
                        && new_offset < applied_new_offset + bytes.len()
                    {
                        return if value == bytes[new_offset - applied_new_offset] {
                            Some(chunk)
                        } else {
                            None
                        };
                    }
                }
                BinaryDiffChunk::Delete(_, _) => (),
            }

            applied_new_offset += chunk.patched_length()
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::binary_diff::binary_diff_chunk::BinaryDiffChunk::{Delete, Insert, Replace, Same};
    use crate::binary_diff_analyzer::BinaryDiffAnalyzer;
    use crate::BinaryDiff;

    #[test]
    fn test_binary_diff_analyzer_none() {
        let diff = BinaryDiff::from(&vec![Same(0, 4)]);
        let analyzer = BinaryDiffAnalyzer::new(&diff);
        assert_eq!(analyzer.derives_from(5, 0), None);
    }

    #[test]
    fn test_binary_diff_analyzer_delete_same() {
        let chunk = Same(6, 2);
        let diff = BinaryDiff::from(&vec![Same(0, 4), Delete(4, 2), chunk.clone()]);
        let analyzer = BinaryDiffAnalyzer::new(&diff);
        assert_eq!(analyzer.derives_from(4, 0), Some(&chunk));
        assert_eq!(analyzer.derives_from(5, 0), Some(&chunk));
        assert_eq!(analyzer.derives_from(6, 0), None);
    }

    #[test]
    fn test_binary_diff_analyzer_insert() {
        let chunk = Insert(0, vec![0, 1, 2, 3]);
        let diff = BinaryDiff::from(&vec![chunk.clone()]);
        let analyzer = BinaryDiffAnalyzer::new(&diff);
        assert_eq!(analyzer.derives_from(0, 0), Some(&chunk));
        assert_eq!(analyzer.derives_from(1, 1), Some(&chunk));
        assert_eq!(analyzer.derives_from(2, 2), Some(&chunk));
        assert_eq!(analyzer.derives_from(3, 3), Some(&chunk));
        assert_eq!(analyzer.derives_from(4, 4), None);
    }

    #[test]
    fn test_binary_diff_analyzer_replace() {
        let chunk = Replace(4, 2, vec![0, 1, 2, 3]);
        let diff = BinaryDiff::from(&vec![Same(0, 4), chunk.clone()]);
        let analyzer = BinaryDiffAnalyzer::new(&diff);
        assert_eq!(analyzer.derives_from(4, 0), Some(&chunk));
        assert_eq!(analyzer.derives_from(5, 1), Some(&chunk));
        assert_eq!(analyzer.derives_from(6, 2), Some(&chunk));
        assert_eq!(analyzer.derives_from(7, 3), Some(&chunk));
        assert_eq!(analyzer.derives_from(8, 0), None);
    }

    #[test]
    fn test_binary_diff_analyzer_replace_same() {
        let chunk = Same(8, 2);
        let diff = BinaryDiff::from(&vec![
            Same(0, 4),
            Replace(4, 2, vec![0, 1, 2, 3]),
            chunk.clone(),
        ]);
        let analyzer = BinaryDiffAnalyzer::new(&diff);
        assert_eq!(analyzer.derives_from(8, 0), Some(&chunk));
        assert_eq!(analyzer.derives_from(9, 0), Some(&chunk));
        assert_eq!(analyzer.derives_from(10, 0), None);
    }
}
