#[allow(dead_code)]
pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    for (i, _) in haystack.iter().enumerate() {
        if haystack[i..haystack.len()].starts_with(needle) {
            log::trace!(
                "find(haystack={:?}, needle={:?}) = {:?}",
                haystack,
                needle,
                Some(i)
            );
            return Some(i);
        }
    }
    log::trace!("find(haystack={:?}, needle={:?}) = None", haystack, needle);
    None
}
