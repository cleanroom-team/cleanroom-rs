// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

// cSpell: ignore peekable

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
    /// A command was not executable
    #[error("{command:?} {} failed with exit status {status:?}: {message}", args.join(&OsString::from(" ")).to_string_lossy())]
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
    /// A command was not executable
    #[error("\"{0:?}\" is not executable")]
    CommandNotExecutable(PathBuf),
    /// Failed to set up containment
    #[error("Failed to set up containment: {0}")]
    ContainmentFailure(String),
    /// Some low level error bubbled up
    #[error(transparent)]
    FsError(#[from] std::io::Error),
    /// Some low level error bubbled up
    #[error(transparent)]
    UtilError(#[from] util::Error),
    /// `root` credentials are required
    #[error("Failed to get root access: {0}")]
    RootNeeded(String),
    /// Run environment validation failed
    #[error("Run environment validation failed: {0}")]
    RunEnvironment(String),
    /// Run environment validation failed
    #[error("Failed to parse {0} into a binding")]
    BindingParseFailed(String),
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
    pub fn rw(source: &(impl AsRef<Path> + ?Sized), target: &(impl AsRef<Path> + ?Sized)) -> Self {
        Self::RW(BindMap {
            source: source.as_ref().to_path_buf(),
            target: target.as_ref().to_path_buf(),
        })
    }

    /// Create a new `RO` `Binding`
    #[must_use]
    pub fn ro(source: &(impl AsRef<Path> + ?Sized), target: &(impl AsRef<Path> + ?Sized)) -> Self {
        Self::RO(BindMap {
            source: source.as_ref().to_path_buf(),
            target: target.as_ref().to_path_buf(),
        })
    }

    /// Create a new `TmpFS` `Binding`
    #[must_use]
    pub fn tmpfs(target: &(impl AsRef<Path> + ?Sized)) -> Self {
        Self::TmpFS(target.as_ref().to_path_buf())
    }

    /// Create a new `Inaccessible` `Binding`
    #[must_use]
    pub fn inaccessible(target: &(impl AsRef<Path> + ?Sized)) -> Self {
        Self::Inaccessible(target.as_ref().to_path_buf())
    }

    /// Create a new `Overlay` `Binding`
    #[must_use]
    pub fn overlay(
        sources: &[&(impl AsRef<Path> + ?Sized)],
        target: &(impl AsRef<Path> + ?Sized),
    ) -> Self {
        Self::Overlay(OverlayMap {
            sources: sources.iter().map(|p| p.as_ref().to_path_buf()).collect(),
            target: target.as_ref().to_path_buf(),
        })
    }

    /// Create a new `OverlayRO` `Binding`
    #[must_use]
    pub fn overlay_ro(
        sources: &[&(impl AsRef<Path> + ?Sized)],
        target: &(impl AsRef<Path> + ?Sized),
    ) -> Self {
        Self::Overlay(OverlayMap {
            sources: sources.iter().map(|p| p.as_ref().to_path_buf()).collect(),
            target: target.as_ref().to_path_buf(),
        })
    }
}

impl TryFrom<&str> for Binding {
    type Error = crate::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let error = || Err(Self::Error::BindingParseFailed(value.to_string()));

        let mut parts = value.split(':').peekable();

        let kind = parts.next();

        match kind {
            Some("rw") => {
                let Some(from) = parts.next() else {
                    return error();
                };
                let Some(to) = parts.next() else {
                    return error();
                };
                if parts.next().is_some() {
                    return error();
                }
                Ok(Self::rw(from, to))
            }
            Some("ro") => {
                let Some(from) = parts.next() else {
                    return error();
                };
                let Some(to) = parts.next() else {
                    return error();
                };
                if parts.next().is_some() {
                    return error();
                }
                Ok(Self::ro(from, to))
            }
            Some("tmpfs") => {
                let Some(from) = parts.next() else {
                    return error();
                };
                if parts.next().is_some() {
                    return error();
                }
                Ok(Self::tmpfs(from))
            }
            Some("inaccessible") => {
                let Some(from) = parts.next() else {
                    return error();
                };
                if parts.next().is_some() {
                    return error();
                }
                Ok(Self::inaccessible(from))
            }
            Some("overlay") => {
                let args = parts.collect::<Vec<_>>();
                let len = args.len();
                if len >= 2 {
                    Ok(Self::overlay(&args[..len - 1], &args[len - 1]))
                } else {
                    error()
                }
            }
            Some("overlay_ro") => {
                let args = parts.collect::<Vec<_>>();
                let len = args.len();
                if len >= 2 {
                    Ok(Self::overlay_ro(&args[..len - 1], &args[len - 1]))
                } else {
                    error()
                }
            }
            _ => error(),
        }
    }
}

/// The environment to use as a container
#[derive(Clone, Debug)]
pub enum RunEnvironment {
    /// Run inside an file system image
    Image(PathBuf),
    /// Run inside an directory
    Directory(PathBuf),
}

fn validate_image(img: &Path) -> Result<PathBuf> {
    let img = img.canonicalize().map_err(|e| {
        Error::RunEnvironment(format!(
            "Failed to canonicalize bootstrap image {img:?}: {e}"
        ))
    })?;
    if img.is_file() {
        Ok(img)
    } else {
        Err(Error::RunEnvironment(format!(
            "Bootstrap image {img:?} is not a file"
        )))
    }
}

fn validate_directory(dir: &Path) -> Result<PathBuf> {
    let dir = dir.canonicalize().map_err(|e| {
        Error::RunEnvironment(format!(
            "Failed to canonicalize bootstrap directory {dir:?}: {e}"
        ))
    })?;
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(Error::RunEnvironment(format!(
            "Bootstrap directory {dir:?} is not a directory"
        )))
    }
}

impl RunEnvironment {
    /// Create a new environment with either a directory or an image
    ///
    /// # Errors
    ///
    /// Things may go wrong...
    pub fn new(directory: &Option<PathBuf>, image: &Option<PathBuf>) -> Result<Self> {
        match (directory, image) {
            (Some(dir), None) => {
                let dir = validate_directory(dir)?;
                Ok(RunEnvironment::Directory(dir))
            }
            (None, Some(img)) => {
                let img = validate_image(img)?;
                Ok(RunEnvironment::Image(img))
            }
            _ => Err(Error::RunEnvironment(
                "Either --bootstrap-directory or --bootstrap-image is needed to `run`".to_string(),
            )),
        }
    }

    /// Validate the environment
    ///
    /// # Errors
    ///
    /// Things may go wrong...
    pub fn validate(&self) -> Result<()> {
        match self {
            RunEnvironment::Directory(dir) => {
                let _ = validate_directory(dir)?;
                Ok(())
            }
            RunEnvironment::Image(img) => {
                let _ = validate_image(img)?;
                Ok(())
            }
        }
    }
}

// - Modules:
// ----------------------------------------------------------------------

mod command;
pub use command::Command;

mod runner;
pub use runner::{Nspawn, Runner, Runtime};
