// page size (initialized properly in `os_init`)
const os_page_size: usize = 4096;

// minimal allocation granularity
const os_alloc_granularity: usize = 4096;

// if non-zero, use large page allocation
const large_os_page_size: usize = 0;

// is memory overcommit allowed?
// set dynamically in _mi_os_init (and if true we use MAP_NORESERVE)
const os_overcommit: bool = true;

const fn _mi_os_has_overcommit() -> bool {
    os_overcommit
}

// OS (small) page size
pub fn _mi_os_page_size() -> usize {
    os_page_size
}
