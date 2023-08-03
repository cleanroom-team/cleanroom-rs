// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Tobias Hunger <tobias.hunger@gmail.com>

use std::ffi::OsString;

// Error handling

/// Possible Errors raised by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Some directory in PATH could not get mapped to an absolute directory
    #[error("Could not resolve \"{0:?}\" in environment")]
    InvalidEnv(OsString),
    /// Some directory in PATH could not get mapped to an absolute directory
    #[error("Could not resolve \"{directory:?}\": {reason}")]
    InvalidDirectory { reason: String, directory: OsString },
    #[error("Required executable \"{0}\" not found.")]
    ExecutableNotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Exports:
mod binaries;
pub use binaries::{find_in_path, is_executable_file, require_binary, resolve_directory};

mod users;
pub use users::is_effective_root;
