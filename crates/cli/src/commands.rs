// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context;

use crate::context::Context as ClrContext;

/// The trait that all `Command`s need to implement
pub trait Command: CommandInfo {
    /// Phase 2: Run the actual command
    fn run(&self, ctx: &ClrContext, arguments: &[String]) -> anyhow::Result<()>;

    /// Phase 3: Test the image once everything is done.
    fn test(&self, ctx: &ClrContext, arguments: &[String]) -> anyhow::Result<()>;
}

/// A helper crate to get information on a Command
pub trait CommandInfo {
    /// Phase 0: Help message
    fn name(&self) -> &str;

    /// Phase 0: Help message
    fn usage(&self) -> String;

    /// Phase 0: Help message
    fn help(&self) -> String;

    /// Phase 1: Validate all commands as they are read in
    fn validate(&self, arguments: &[String]) -> anyhow::Result<()>;
}

/// A helper crate to get a clap command for something
pub trait ClapCommandInfo {
    fn clap_command(&self) -> &clap::Command;
}

/// A blanket-implementation to ensure all `ClapCommandInfo` also implement
/// `CommandInfo`
impl CommandInfo for dyn ClapCommandInfo {
    fn name(&self) -> &str {
        self.clap_command().get_name()
    }

    fn usage(&self) -> String {
        let mut cc = self.clap_command().clone();
        format!("{}", cc.render_usage())
    }

    fn help(&self) -> String {
        let mut cc = self.clap_command().clone();
        format!("{}", cc.render_help())
    }

    fn validate(&self, arguments: &[String]) -> anyhow::Result<()> {
        let name = self.name().to_owned();
        let cc = self.clap_command().clone();
        cc.try_get_matches_from(arguments)
            .context(format!("Failed to parse arguments to {}", &name))?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Manager {}
