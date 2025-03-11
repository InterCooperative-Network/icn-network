//! Result type for the Intercooperative Network

use crate::error::Error;

/// Result type for the Intercooperative Network
pub type Result<T> = std::result::Result<T, Error>; 