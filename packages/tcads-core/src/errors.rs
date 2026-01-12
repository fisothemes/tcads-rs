use std::io;
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, Hash)]
#[error("Invalid AmsNetId format: expected 6 numbers separated by dots (e.g. '5.1.2.3.1.1')")]
pub struct ParseNetIdError;
