// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::BTreeMap, ffi::OsStr, path::Path, rc::Rc};

use anyhow::{anyhow, Context};

use crate::printer::Printer;

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

fn validate_phase_name(phase_name: &str) -> anyhow::Result<()> {
    if crate::Phases::Invalid.any(|p| {
        eprintln!("Phase: {}", p.to_string());
        p.to_string() == phase_name
    }) {
        Ok(())
    } else {
        Err(anyhow!("{phase_name} is not a valid phase name"))
    }
}

fn validate_sub_phase_name(sub_phase_name: &str) -> anyhow::Result<()> {
    if crate::SubPhases::Invalid.any(|p| {
        eprintln!("SubPhase: {}", p.to_string());
        p.to_string() == sub_phase_name
    }) {
        Ok(())
    } else {
        Err(anyhow!("{sub_phase_name} is not a valid sub-phase name"))
    }
}

/// Meta-information about an `Input`
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
enum Input {
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

/// Meta-information about a `Command` script snippet
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
enum Phase {
    /// The snippet
    Simple(String),
    Complex(BTreeMap<String, String>),
}

impl Phase {
    fn defined_sub_phases(&self) -> impl Iterator<Item = String> + '_ {
        match self {
            Phase::Simple(_) => itertools::Either::Left(std::iter::once(String::from("main"))),
            Phase::Complex(m) => itertools::Either::Right(m.keys().cloned()),
        }
    }

    fn sub_phase(&self, sub_phase: crate::SubPhases) -> Option<String> {
        match self {
            Phase::Simple(value) => {
                if sub_phase == crate::SubPhases::Main {
                    Some(value.clone())
                } else {
                    None
                }
            }
            Phase::Complex(m) => m.get(&sub_phase.to_string()).cloned(),
        }
    }
}

/// Meta-information about a `Command`
#[derive(Clone, Debug, serde::Deserialize)]
struct Command {
    /// Help about the `Command`
    help: Option<String>,
    /// Can this command get aliased?
    #[serde(default)]
    can_alias: bool,
    /// Input to the command
    inputs: Vec<Input>,

    /// Script snippets
    phases: BTreeMap<String, Phase>,
}

/// A `Command` as defined
#[derive(Debug, serde::Deserialize)]
struct TomlCommand {
    /// The `Data`about the `Command`
    command: Command,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(help) = &self.help {
            writeln!(f, "    {help}\n")?;
        }

        let inputs = &self.inputs;
        if inputs.is_empty() {
            writeln!(f, "  inputs: <none>\n")?
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

impl TomlCommand {
    fn read_from_file(command: &Path) -> anyhow::Result<Command> {
        let contents = std::fs::read_to_string(command)
            .context(format!("Failed to read command definition in {command:?}"))?;

        let header = toml::from_str::<TomlCommand>(&contents)
            .context(format!("Failed to parse command from {command:?}"))?;
        Ok(header.command)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandManagerBuilder {
    commands: BTreeMap<String, Rc<Command>>,
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

                for (phase_name, phase) in cmd.phases.iter() {
                    validate_phase_name(phase_name)?;
                    for sub_phase_name in phase.defined_sub_phases() {
                        validate_sub_phase_name(&sub_phase_name)?;
                    }
                }

                if self.commands.insert(name.clone(), Rc::new(cmd)).is_some() {
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
    commands: BTreeMap<String, Rc<Command>>,
}

impl CommandManager {
    pub fn generate_script_snippet_for(
        &self,
        name: &str,
        phase: crate::Phases,
        sub_phase: crate::SubPhases,
    ) -> anyhow::Result<Option<String>> {
        if let Some(command) = self.commands.get(name) {
            let Some(phase) = &command.phases.get(&format!("{}", phase)) else {
                return Ok(None);
            };
            Ok(phase.sub_phase(sub_phase).clone())
        } else {
            Err(anyhow::anyhow!("Command {name:?} not found"))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn alias(&mut self, from: &str, to: &str) -> anyhow::Result<()> {
        let Some(from) = self.commands.get(from) else {
            return Err(anyhow!("Can not find {from} to alias"));
        };
        if !from.can_alias {
            return Err(anyhow!("I may not alias {from}. Check the command definition or maybe it was loaded from a user directory?"));
        }
        validate_command_name(to).context(format!("{to} is not a valid name to alias to"))?;

        self.commands.insert(to.to_string(), from.clone());

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
