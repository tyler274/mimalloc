#![feature(const_ptr_is_null)]
#![feature(const_mut_refs)]
#![feature(const_raw_ptr_comparison)]
#![feature(const_trait_impl)]
#![feature(strict_provenance)]
#![feature(const_borrow)]
#![feature(const_option)]
#![feature(const_option_ext)]
#![feature(const_refs_to_cell)]
#![feature(const_slice_index)]
#![feature(const_unsafecell_get_mut)]
// #![feature(pointer_is_aligned)]
#![feature(const_maybe_uninit_write)]
#![feature(const_maybe_uninit_as_mut_ptr)]
mod alloc;
mod arena;
mod bitmap;
mod block;
mod constants;
mod debug;
mod heap;
mod init;
mod internal;
mod mimalloc;
mod options;
mod os;
mod page;
mod page_queue;
mod random;
mod segment;
mod span_queue;
mod stats;
mod thread;
