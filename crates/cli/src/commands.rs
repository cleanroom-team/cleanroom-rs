// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::BTreeMap, ffi::OsStr, path::Path};

use anyhow::{anyhow, Context};

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

fn validate_variable_name(name: &str) -> anyhow::Result<()> {
    if name
        .chars()
        .take(1)
        .all(|c| c.is_ascii_alphabetic() || c == '_')
        && name
            .chars()
            .skip(1)
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Ok(())
    } else {
        Err(anyhow!("Invalid command name {name}"))
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize)]
pub struct CommandName(String);

impl TryFrom<String> for CommandName {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_command_name(&value).context(format!("{value} is not a valid command name"))?;
        Ok(CommandName(value))
    }
}

impl TryFrom<&Path> for CommandName {
    type Error = anyhow::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let name: CommandName = value
            .file_stem()
            .context(format!("No file stem in {value:?}"))?
            .to_string_lossy()
            .to_string()
            .try_into()?;
        Ok(name)
    }
}

impl std::fmt::Display for CommandName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CommandName {
    pub fn parse_value(value: &str) -> anyhow::Result<Self> {
        validate_command_name(value)?;
        Ok(Self(value.to_string()))
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize)]
pub struct VariableName(String);

impl TryFrom<String> for VariableName {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_variable_name(&value).context(format!("{value} is not a valid variable name"))?;
        Ok(VariableName(value))
    }
}

impl std::fmt::Display for VariableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Meta-information about an `Input`
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Input {
    Basic(VariableName),
    Full {
        name: VariableName,
        help: Option<String>,
        #[serde(default)]
        optional: bool,
    },
}

impl Input {
    pub fn name(&self) -> VariableName {
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

    pub fn optional(&self) -> bool {
        match self {
            Input::Basic(_) => false,
            Input::Full { optional, .. } => *optional,
        }
    }
}

/// Meta-information about a `Command`
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Command {
    /// Help about the `Command`
    help: Option<String>,
    /// Input to the command
    #[serde(default)]
    inputs: Vec<Input>,

    /// Script snippet
    pub script: String,

    /// The source of the command itself.
    #[serde(skip)]
    source_location: String,

    /// The source of the command itself.
    #[serde(skip)]
    source: String,

    /// The definition this command overwrites
    #[serde(skip)]
    overwrote_definition_in: Vec<String>,
}

impl Command {
    pub fn inputs(&self) -> impl Iterator<Item = &Input> {
        self.inputs.iter()
    }

    pub fn dump_source(&self) -> &str {
        &self.source
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
        let overwrote = &self.overwrote_definition_in;
        if !overwrote.is_empty() {
            writeln!(
                f,
                "  *** Overwrote definition in {} ***",
                overwrote.join(", ")
            )?;
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

        Self::from_str(&contents, &command.to_string_lossy())
            .context(format!("Failed to parse {command:?}"))
    }

    fn from_str(contents: &str, source_location: &str) -> anyhow::Result<Command> {
        let mut header = toml::from_str::<TomlCommand>(contents)
            .context("Failed to parse command definition")?;

        header.command.source_location = source_location.to_string();
        header.command.source = contents.to_string();

        Ok(header.command)
    }
}

#[derive(Clone, Debug)]
pub struct CommandManagerBuilder {
    commands: BTreeMap<CommandName, Command>,
}

impl Default for CommandManagerBuilder {
    fn default() -> Self {
        let mut result = Self {
            commands: Default::default(),
        };

        // Add builtin commands:
        for f in BUILTIN_COMMANDS_DIR.files() {
            let contents = f.contents_utf8().unwrap();
            let name = CommandName::try_from(f.path()).unwrap();

            let cmd = TomlCommand::from_str(contents, "<builtin>").unwrap();
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

    pub fn scan_for_commands(&mut self, command_directory: &Path) -> anyhow::Result<()> {
        let contents = std::fs::read_dir(command_directory).context(format!(
            "Failed to scan for commands in {command_directory:?}"
        ))?;

        for p in contents {
            let p = p?.path();
            if p.is_file() && p.extension() == Some(OsStr::new("toml")) {
                let mut cmd = TomlCommand::read_from_file(&p)
                    .context(format!("Failed to read command from {p:?}"))?;

                let name = CommandName::try_from(&p as &Path)?;

                if let Some(old) = self.commands.get(&name) {
                    cmd.overwrote_definition_in = old
                        .overwrote_definition_in
                        .iter()
                        .cloned()
                        .chain(std::iter::once(&old.source_location).cloned())
                        .collect();
                }

                self.commands.insert(name, cmd);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CommandManager {
    commands: BTreeMap<CommandName, Command>,
}

impl CommandManager {
    pub fn command(&self, name: &CommandName) -> anyhow::Result<&Command> {
        if let Some(command) = self.commands.get(name) {
            Ok(command)
        } else {
            Err(anyhow::anyhow!("Command {name:?} not found"))
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = (&CommandName, &Command)> {
        self.commands.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn list_commands(&self, verbose: bool) -> String {
        let mut result = String::new();
        let default_value = "no help".to_string();

        if verbose {
            self.commands.iter().for_each(|(name, command)| {
                result += &format!("{name}:\n");
                result += &format!("{command}");
            });
        } else {
            self.commands.iter().for_each(|(name, command)| {
                let help: &str = command.help.as_ref().unwrap_or(&default_value);
                result += &format!("  {name}:\t{help}\n");
            });
        }

        result
    }

    pub fn show_command(&self, name: &CommandName) -> anyhow::Result<String> {
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
