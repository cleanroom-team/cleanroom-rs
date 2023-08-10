// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::HashMap;
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
    pub environment: HashMap<OsString, OsString>,
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

    /// Add or change one environment variable
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.environment.insert((&key).into(), (&val).into());
        self
    }

    /// Adds or updates multiple environment variable mappings.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        vars.into_iter().for_each(|(k, v)| {
            self.environment.insert((&k).into(), (&v).into());
        });
        self
    }

    /// Removes an environment variable mapping.
    // pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
    //     self.environment.remove(k);
    //     self
    // }

    /// Clears the entire environment map for the child process.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// # async fn test() { // allow using await
    /// use tokio::process::Command;
    ///
    /// let output = Command::new("ls")
    ///         .env_clear()
    ///         .output().await.unwrap();
    /// # }
    /// ```
    pub fn env_clear(&mut self) -> &mut Command {
        self.environment.clear();
        self
    }
}
