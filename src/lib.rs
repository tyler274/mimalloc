mod mimalloc_types;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = mimalloc_types::MI_SLICES_PER_SEGMENT;
        assert_eq!(result, 1024);
    }
}
