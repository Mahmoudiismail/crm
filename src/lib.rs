#![forbid(unsafe_code)]
#![warn(
    future_incompatible,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

//! Shared contracts and runtime utilities used by the independent CRM Tool processes.
//! Application implementations live in their owning packages under apps/.

pub mod manifest;
pub mod tasker;
pub mod utils;
