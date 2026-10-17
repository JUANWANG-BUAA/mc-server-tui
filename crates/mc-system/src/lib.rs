//! Local host discovery used by doctor and startup preflight operations.

mod doctor;
mod java;

pub use doctor::run_doctor;
