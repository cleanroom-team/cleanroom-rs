// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Functionality related to running a command in a container

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::let_unit_value)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

// ----------------------------------------------------------------------
// - Error Handling:
// ----------------------------------------------------------------------

/// The `Error` enum for this crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{command:?} {:?} failed with exit status {status:?}: {message}", args.join(&OsString::from(" ")))]
    /// A command was not executable
    CommandFailed {
        /// The command that failed
        command: PathBuf,
        /// Arguments:
        args: Vec<OsString>,
        /// Error message
        message: String,
        /// Exit code (if any)
        status: Option<i32>,
    },
    #[error("\"{0:?}\" is not executable")]
    /// A command was not executable
    CommandNotExecutable(PathBuf),
    #[error("Failed to set up containment: {0}")]
    /// Failed to set up containment
    ContainmentFailure(String),
    #[error(transparent)]
    /// Some low level error bubbled up
    FsError(#[from] std::io::Error),
    #[error(transparent)]
    /// Some low level error bubbled up
    UtilError(#[from] util::Error),
    #[error("Failed to get root access: {0}")]
    /// `root` credentials are required
    RootNeeded(String),
}

/// `contained_command` `Result` type
pub type Result<T> = std::result::Result<T, Error>;

// ----------------------------------------------------------------------
// - Binding:
// ----------------------------------------------------------------------

/// A mapping of outside file system location to a in-container path
#[derive(Clone, Debug)]
pub struct BindMap {
    source: PathBuf,
    target: PathBuf,
}

/// A mapping for a overlay file system into the container
#[derive(Clone, Debug)]
pub struct OverlayMap {
    sources: Vec<PathBuf>,
    target: PathBuf,
}

/// A `Binding` definition for mount points
#[derive(Clone, Debug)]
pub enum Binding {
    /// A read/write binding
    RW(BindMap),
    /// A read only binding
    RO(BindMap),
    /// Put a tmpfs into the specified path inside the container
    TmpFS(PathBuf),
    /// Make a path inside the container inaccessible
    Inaccessible(PathBuf),
    /// Overlay some directory with another
    Overlay(OverlayMap),
    /// Overlay some directory with another
    OverlayRO(OverlayMap),
}

impl Binding {
    /// Create a new `RW` `Binding`
    #[must_use]
    pub fn rw(source: &impl AsRef<Path>, target: &impl AsRef<Path>) -> Self {
        Self::RW(BindMap {
            source: source.as_ref().to_path_buf(),
            target: target.as_ref().to_path_buf(),
        })
    }

    /// Create a new `RO` `Binding`
    #[must_use]
    pub fn ro(source: &impl AsRef<Path>, target: &impl AsRef<Path>) -> Self {
        Self::RO(BindMap {
            source: source.as_ref().to_path_buf(),
            target: target.as_ref().to_path_buf(),
        })
    }

    /// Create a new `TmpFS` `Binding`
    #[must_use]
    pub fn tmpfs(target: &impl AsRef<Path>) -> Self {
        Self::TmpFS(target.as_ref().to_path_buf())
    }

    /// Create a new `Inaccessible` `Binding`
    #[must_use]
    pub fn inaccessible(target: &impl AsRef<Path>) -> Self {
        Self::Inaccessible(target.as_ref().to_path_buf())
    }

    /// Create a new `Overlay` `Binding`
    #[must_use]
    pub fn overlay(sources: &[&impl AsRef<Path>], target: &impl AsRef<Path>) -> Self {
        Self::Overlay(OverlayMap {
            sources: sources.iter().map(|p| p.as_ref().to_path_buf()).collect(),
            target: target.as_ref().to_path_buf(),
        })
    }

    /// Create a new `OverlayRO` `Binding`
    #[must_use]
    pub fn overlay_ro(sources: &[&impl AsRef<Path>], target: &impl AsRef<Path>) -> Self {
        Self::Overlay(OverlayMap {
            sources: sources.iter().map(|p| p.as_ref().to_path_buf()).collect(),
            target: target.as_ref().to_path_buf(),
        })
    }
}

// - Modules:
// ----------------------------------------------------------------------

mod command;
pub use command::Command;

mod runner;
pub use runner::{Nspawn, Runner, Runtime};
