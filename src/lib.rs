#![forbid(unsafe_code)]
#![warn(
    future_incompatible,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

//! # CRM Tool Core Library
//!
//! This library provides the core, shared implementations for the suite of CRM Tool binaries.
//! It includes configuration handling, date parsing, UI manifestation logic, and core operations
//! for runner and tasker executables.
//!
//! - `crm`: CRM fetching and downloading APIs.
//! - `runner`: Task scheduling and UI integration.
//! - `tasker`: Background workers (CSV manipulation, email automation, Webex).
//! - `yasweb`: Headless web automation implementations.
//! - `manifest`: Configuration schemas for external app integration.
//! - `utils`: Cross-binary utilities such as logging, config management, and date parsing.

pub mod crm;
pub mod manifest;
pub mod runner;
pub mod tasker;
pub mod utils;
pub mod yasweb;
