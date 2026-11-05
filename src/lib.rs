#![forbid(unsafe_code)]

#[cfg(not(target_os = "linux"))]
compile_error!("PRYN currently supports Linux targets only");

pub mod app;
pub mod cli;
pub mod model;
pub mod modules;
pub mod platform;
pub mod report;
