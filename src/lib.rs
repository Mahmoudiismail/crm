#![forbid(unsafe_code)]
#![warn(
    future_incompatible,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

pub mod crm;
pub mod manifest;
pub mod runner;
pub mod tasker;
pub mod utils;
pub mod yasweb;
