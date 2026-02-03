//! Hord3 is an experimental, highly modular game engine with software-rendered graphics by default
//! 
//! This is mostly a personal learning project, and for the vast majority of usecases, if you want to make a game in Rust, go with any other engine
//! With that said, this game engine sets itself apart with 3 main features
//! - CPU-only rendering by default
//! - a "compile-time ECS (Entity Component System)" using proc-macros to avoid the common patterns of trait objects
//! - an "orchestrator" letting the developper pick the order of game engine tasks within a tick in a multi-threaded environment
//! 
//! This project strives for safety before a 1.0 release, but as of writing this, the default rendering backends are unsound, only producing safe assembly on x86 platforms and otherwise breaking atomicity on platforms without atomic operations by default
//! 
//! There are many common things reimplemented in this project, like serialization/deserialization or array vecs for example. This is knowingly done as a learning exercise, and is very likely worse than more popular implementations
//! 
//! This also requires the nightly toolchain to compile, as it uses the `portable_simd`, `sync_unsafe_cell`, `extend_one` and `mpmc_channel` experimental features

#![feature(portable_simd)]
#![feature(sync_unsafe_cell)]
#![feature(extend_one)]
#![feature(mpmc_channel)]
pub mod horde;
pub mod defaults;
pub mod tests;