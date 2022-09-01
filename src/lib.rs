#![feature(const_ptr_is_null)]
#![feature(const_mut_refs)]
#![feature(const_raw_ptr_comparison)]
#![feature(const_trait_impl)]
#![feature(strict_provenance)]
mod alloc;
mod arena;
mod bitmap;
mod block;
mod constants;
mod debug;
mod heap;
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
