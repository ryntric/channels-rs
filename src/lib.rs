#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(not(target_has_atomic = "64"))]
compile_error!("channels-rs requires 64-bit atomic operations");

pub(crate) mod buffer;
pub(crate) mod cache;
pub(crate) mod claim;
pub(crate) mod core;
pub(crate) mod cursor;
pub(crate) mod reservation;
pub(crate) mod ring_buffer;
pub(crate) mod sequence;
pub(crate) mod tracking;

pub mod channels;
pub mod lease;
pub mod prelude;
// pub mod coordinator;
