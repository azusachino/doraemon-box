//! Backend for takekoputaa

extern crate redis as redis_rs;
extern crate strum;
extern crate strum_macros;

pub use anyhow::{Context, Result};

#[macro_use]
extern crate quick_error;

pub mod config;
pub mod db;
pub mod errors;

pub mod prelude;
