// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// A `Command` that is supposed to get run
#[derive(Clone, Debug, Default)]
pub struct Command {
    /// The command to run
    pub command: PathBuf,
    /// The arguments passed to the `command`
    pub arguments: Vec<OsString>,
    /// The current directory for the `command`
    pub current_directory: Option<PathBuf>,
    /// Extra environment variables needed to run the command
    pub environment: Vec<(OsString, OsString)>,
    /// Extra bindings needed to run this command
    pub bindings: Vec<crate::Binding>,
    /// The expected exit code:
    pub expected_exit_code: i32,
}

impl Command {
    /// Create a new `Command` to run later
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            command: PathBuf::from(&program),
            ..Self::default()
        }
    }

    /// Add a single argument to the `Command`
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.arguments.push(OsString::from(&arg));
        self
    }

    /// Add several arguments to the `Command`
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.arguments
            .extend(args.into_iter().map(|s| OsString::from(&s)));
        self
    }

    /// Set expected exit code
    pub fn expect_exit_code(&mut self, code: i32) -> &mut Self {
        self.expected_exit_code = code;
        self
    }
}
