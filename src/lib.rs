mod alloc;
mod arena;
mod bitmap;
mod heap;
mod internal;
mod mimalloc;
mod options;
mod os;
mod page;
mod page_queue;
mod random;
mod segment;
mod stats;
mod types;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = types::MI_SLICES_PER_SEGMENT;
        assert_eq!(result, 1024);
    }
}
