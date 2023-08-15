// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::BTreeMap, ffi::OsStr, path::Path};

use anyhow::{anyhow, Context};

use crate::printer::Printer;

static BUILTIN_COMMANDS_DIR: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/commands");

fn validate_command_name(name: &str) -> anyhow::Result<()> {
    if name
        .chars()
        .take(1)
        .all(|c| c.is_ascii_lowercase() || c == '_')
        && name
            .chars()
            .skip(1)
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        Ok(())
    } else {
        Err(anyhow!("Invalid command name {name}"))
    }
}

/// Meta-information about an `Input`
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Input {
    Basic(String),
    Full { name: String, help: Option<String> },
}

impl Input {
    pub fn name(&self) -> String {
        match self {
            Input::Basic(name) => name.clone(),
            Input::Full { name, .. } => name.clone(),
        }
    }

    pub fn help(&self) -> Option<String> {
        match self {
            Input::Basic(_) => None,
            Input::Full { help, .. } => help.clone(),
        }
    }
}

/// Meta-information about a `Command`
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Command {
    /// Help about the `Command`
    help: Option<String>,
    /// Can this command get aliased?
    #[serde(default)]
    pub can_alias: bool,
    /// Input to the command
    #[serde(default)]
    inputs: Vec<Input>,

    /// Script snippet
    pub script: String,
}

impl Command {
    pub fn inputs(&self) -> impl Iterator<Item = &Input> {
        self.inputs.iter()
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(help) = &self.help {
            writeln!(f, "    {help}\n")?;
        }

        let inputs = &self.inputs;
        if inputs.is_empty() {
            writeln!(f, "  inputs: <none>")?
        } else {
            writeln!(f, "  inputs:")?;
            for i in inputs {
                let help = if let Some(help) = i.help() {
                    format!(":\t{help}")
                } else {
                    String::new()
                };
                writeln!(f, "    {}{}", i.name(), help)?
            }
        }
        writeln!(f)
    }
}

/// The toml file structure holding a `Command` as defined
#[derive(Debug, serde::Deserialize)]
struct TomlCommand {
    /// The `Data`about the `Command`
    command: Command,
}

impl TomlCommand {
    fn read_from_file(command: &Path) -> anyhow::Result<Command> {
        let contents = std::fs::read_to_string(command)
            .context(format!("Failed to read command definition in {command:?}"))?;

        Self::from_str(&contents).context(format!("Failed to parse {command:?}"))
    }

    fn from_str(contents: &str) -> anyhow::Result<Command> {
        let header = toml::from_str::<TomlCommand>(contents)
            .context("Failed to parse command definition")?;

        Ok(header.command)
    }
}

#[derive(Clone, Debug)]
pub struct CommandManagerBuilder {
    commands: BTreeMap<String, Command>,
}

impl Default for CommandManagerBuilder {
    fn default() -> Self {
        let mut result = Self {
            commands: Default::default(),
        };

        // Add builtin commands:
        for f in BUILTIN_COMMANDS_DIR.files() {
            let contents = f.contents_utf8().unwrap();
            let name = f.path().file_stem().unwrap().to_string_lossy().to_string();

            let cmd = TomlCommand::from_str(contents).unwrap();
            result.commands.insert(name, cmd);
        }

        result
    }
}

impl CommandManagerBuilder {
    pub fn build(&self) -> CommandManager {
        CommandManager {
            commands: self.commands.clone(),
        }
    }

    pub fn scan_for_commands(
        &mut self,
        command_directory: &Path,
        can_alias: bool, // Allows to force aliasing to off for untrusted commands
        printer: &Printer,
    ) -> anyhow::Result<()> {
        let contents = std::fs::read_dir(command_directory).context(format!(
            "Failed to scan for commands in {command_directory:?}"
        ))?;

        for p in contents {
            let p = p?.path();
            if p.is_file() && p.extension() == Some(OsStr::new("toml")) {
                let mut cmd = TomlCommand::read_from_file(&p)
                    .context(format!("Failed to read command from {p:?}"))?;

                cmd.can_alias = cmd.can_alias && can_alias; // Force aliasing off
                let name = p.file_stem().unwrap().to_string_lossy().to_string();
                validate_command_name(&name)?;

                if self.commands.insert(name.clone(), cmd).is_some() {
                    printer.info(&format!("Re-defined command {}", name));
                }
            } else {
                printer.trace(&format!(
                    "Not considering {p:?} as command: Not a toml file"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CommandManager {
    commands: BTreeMap<String, Command>,
}

impl CommandManager {
    pub fn command(&self, name: &str) -> anyhow::Result<&Command> {
        if let Some(command) = self.commands.get(name) {
            Ok(command)
        } else {
            Err(anyhow::anyhow!("Command {name:?} not found"))
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = (&String, &Command)> {
        self.commands.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn alias(&mut self, from: &str, to: &str) -> anyhow::Result<()> {
        let Some(from_cmd) = self.commands.get(from) else {
            return Err(anyhow!("Can not find {from} to alias"));
        };
        if !from_cmd.can_alias {
            return Err(anyhow!("I may not alias {from}. Check the command definition or maybe it was loaded from a user directory?"));
        }

        validate_command_name(to).context(format!("{to} is not a valid name to alias to"))?;
        let aliased_command = Command {
            script: format!("{from} \"${{@}}\""),
            can_alias: false,
            help: Some(format!("Alias of {from}")),
            inputs: Vec::new(),
        };

        self.commands.insert(to.to_string(), aliased_command);

        Ok(())
    }

    pub fn list_commands(&self, verbose: bool) -> String {
        let mut result = String::new();
        let default_value = "no help".to_string();

        if verbose {
            self.commands.iter().for_each(|(name, command)| {
                result += &format!("    {name}:\n");
                result += &format!("      {command}");
            });
        } else {
            self.commands.iter().for_each(|(name, command)| {
                let help: &str = command.help.as_ref().unwrap_or(&default_value);
                result += &format!("    {name}:\t{help}\n");
            });
        }

        result
    }

    pub fn show_command(&self, name: &str) -> anyhow::Result<String> {
        let Some(command) = self.commands.get(name) else {
            return Err(anyhow::anyhow!("Unknown command: {name}"));
        };

        Ok(format!("{command}"))
    }
}

impl std::fmt::Display for CommandManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.list_commands(false))
    }
}
