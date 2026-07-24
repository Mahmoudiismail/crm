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
//! ## Modules
//!
//! - `crm`: CRM fetching and downloading APIs. Handles authenticated sessions and concurrent downloads.
//! - `runner`: Task scheduling and UI integration. Orchestrates task queues and provides a web GUI.
//! - `tasker`: Background workers (CSV manipulation, email automation, Excel COM automation).
//! - `yasweb`: Headless web automation implementations for legacy MIS modules using Chrome DevTools Protocol.
//! - `manifest`: Configuration schemas for external app integration allowing the runner GUI to dynamically build UI elements.
//! - `utils`: Cross-binary utilities such as logging, config management, and dynamic date parsing.

pub mod crm;
pub mod manifest;
pub mod runner;
pub mod tasker;
pub mod utils;
pub mod yasweb;
