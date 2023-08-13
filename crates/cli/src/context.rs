// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `Context` to run in

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{anyhow, Context as AhContext};

#[derive(Clone, Debug)]
pub struct ContextEntry {
    pub name: String,
    pub value: String,
    pub is_read_only: bool,
    pub is_internal: bool,
}

#[derive(Clone, Debug)]
struct ContextData {
    value: OsString,
    is_read_only: bool,
    is_internal: bool,
}

#[derive(Clone, Debug)]
struct ContextMap(BTreeMap<OsString, ContextData>);

impl std::fmt::Display for ContextMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (k, v) in &self.0 {
            let mut extra = String::from("(");
            if v.is_read_only {
                extra += "ro";
            }
            if v.is_internal {
                if v.is_read_only {
                    extra += ", internal"
                } else {
                    extra += "internal"
                }
            }
            extra += ")";
            writeln!(f, "    {k:?}={:?} {}", &v.value, extra)?;
        }
        Ok(())
    }
}

impl ContextMap {
    fn get(&self, name: &str) -> Option<String> {
        self.get_raw(name).map(|v| v.to_string_lossy().to_string())
    }

    fn get_raw(&self, name: &str) -> Option<OsString> {
        self.0.get(&OsString::from(name)).map(|cd| cd.value.clone())
    }

    fn set(
        &mut self,
        name: &str,
        value: &str,
        is_read_only: bool,
        is_internal: bool,
    ) -> anyhow::Result<()> {
        self.set_raw(name, &OsString::from(value), is_read_only, is_internal)
    }

    fn set_raw(
        &mut self,
        name: &str,
        value: &OsStr,
        is_read_only: bool,
        is_internal: bool,
    ) -> anyhow::Result<()> {
        if name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            let name = OsString::from(name);
            if let Some(cd) = self.0.get(&name) {
                if cd.is_read_only {
                    return Err(anyhow!("{name:?} is already set and marked as read-only"));
                }
            }

            self.0.insert(
                name,
                ContextData {
                    value: value.to_os_string(),
                    is_read_only,
                    is_internal,
                },
            );
            Ok(())
        } else {
            Err(anyhow!("Invalid character in variable name \"{name}\""))
        }
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn iter(&self) -> std::collections::btree_map::Iter<'_, OsString, ContextData> {
        self.0.iter()
    }
}

#[derive(Debug)]
pub struct ContextBuilder {
    commands: crate::commands::CommandManagerBuilder,
    printer: crate::printer::Printer,
    timestamp: String,
    version: Option<String>,
}

impl ContextBuilder {
    pub fn new(printer: crate::printer::Printer) -> Self {
        Self {
            commands: crate::commands::CommandManagerBuilder::default(),
            printer,
            timestamp: format!("{}", chrono::Utc::now().format("%Y%m%d%H%M%S")),
            version: None,
        }
    }

    pub fn timestamp(mut self, timestamp: String) -> anyhow::Result<Self> {
        let timestamp = timestamp.trim();

        if timestamp.is_empty() {
            Err(anyhow!("Empty timestamp {timestamp:?} given"))
        } else {
            self.timestamp = timestamp.to_string();
            Ok(self)
        }
    }

    pub fn version(mut self, version: String) -> anyhow::Result<Self> {
        let version = version.trim();

        if version.is_empty() {
            Err(anyhow!("Empty version {version:?} given"))
        } else {
            self.version = Some(version.to_string());
            Ok(self)
        }
    }

    pub fn build(self) -> Context {
        let v = if let Some(v) = self.version {
            v.clone()
        } else {
            self.timestamp.clone()
        };

        Context {
            commands: self.commands,
            printer: Rc::new(self.printer),
            variables: ContextMap(BTreeMap::from([
                (
                    OsString::from(TIMESTAMP),
                    ContextData {
                        value: OsString::from(self.timestamp),
                        is_read_only: true,
                        is_internal: false,
                    },
                ),
                (
                    OsString::from(VERSION),
                    ContextData {
                        value: OsString::from(v),
                        is_read_only: true,
                        is_internal: false,
                    },
                ),
            ])),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    commands: crate::commands::CommandManagerBuilder,
    printer: Rc<crate::printer::Printer>,
    variables: ContextMap,
}

#[derive(Clone, Debug)]
pub struct SystemContext {
    commands: crate::commands::CommandManager,
    printer: Rc<crate::printer::Printer>,
    variables: ContextMap,
}

const AGENT_SCRIPT_DIR: &str = "AGENT_SCRIPT_DIR";
const ARTIFACT_DIR: &str = "ARTIFACT_DIR";
const BUSYBOX_BINARY: &str = "BUSYBOX_BINARY";
const MY_BINARY: &str = "MY_BINARY";
const ROOT_DIR: &str = "ROOT_DIR";
const SCRATCH_DIR: &str = "SCRATCH_DIR";
const SYSTEM_NAME: &str = "SYSTEM_NAME";
const TIMESTAMP: &str = "TIMESTAMP";
const VERSION: &str = "VERSION";
const WORK_DIR: &str = "WORK_DIR";

impl Context {
    #[cfg(test)]
    pub fn test_system(&self) -> SystemContext {
        let mut ctx = SystemContext {
            commands: self.commands.build(),
            printer: self.printer.clone(),
            variables: self.variables.clone(),
        };

        ctx.set(BUSYBOX_BINARY, "/usr/bin/busybox", true, true)
            .unwrap();
        ctx.set(MY_BINARY, "/foo/agent", true, true).unwrap();
        ctx.set(AGENT_SCRIPT_DIR, "/foo/agent_scripts", true, true)
            .unwrap();
        ctx.set(ARTIFACT_DIR, "/foo/artifacts", true, true).unwrap();
        ctx.set(ROOT_DIR, "/foo/work/XXXX/root_fs", true, true)
            .unwrap();
        ctx.set(SCRATCH_DIR, "/foo/work/XXXX", true, true).unwrap();
        ctx.set(SYSTEM_NAME, "test_system", true, false).unwrap();
        ctx.set(WORK_DIR, "/foo/work", true, true).unwrap();

        ctx
    }

    pub fn printer(&self) -> Rc<crate::printer::Printer> {
        self.printer.clone()
    }

    pub fn command_manager_builder(&mut self) -> &mut crate::commands::CommandManagerBuilder {
        &mut self.commands
    }

    // Setter:
    pub fn set_system(
        &self,
        name: &str,
        work_directory: &Path,
        artifact_directory: &Path,
        busybox_binary: &Path,
        myself: &Path,
    ) -> anyhow::Result<SystemContext> {
        let artifact_directory = util::resolve_directory(artifact_directory)
            .context("Failed to resolve work directory")?;
        let work_directory =
            util::resolve_directory(work_directory).context("Failed to resolve work directory")?;

        let scratch_directory = work_directory.join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir(&scratch_directory)
            .context("Failed to create scratch directory in work directory")?;

        let root_directory = scratch_directory.join("root_fs");
        std::fs::create_dir(&root_directory)
            .context("Failed to create root directory in scratch directory")?;

        let agent_script_directory = scratch_directory.join("agent_scripts");
        std::fs::create_dir(&agent_script_directory)
            .context("Failed to create agent script directory in scratch directory")?;

        let busybox_binary = busybox_binary
            .canonicalize()
            .context("Failed to canonicalize busybox binary")?;
        if !util::is_executable_file(&busybox_binary) {
            return Err(anyhow!("{busybox_binary:?} is no file or not executable"));
        }

        let myself = myself
            .canonicalize()
            .context("Failed to canonicalize my own binary path")?;
        if !util::is_executable_file(&myself) {
            return Err(anyhow!("{myself:?} is no file or not executable"));
        }

        let mut ctx = SystemContext {
            commands: self.commands.build(),
            printer: self.printer.clone(),
            variables: self.variables.clone(),
        };

        ctx.set_raw(BUSYBOX_BINARY, busybox_binary.as_os_str(), true, true)
            .unwrap();
        ctx.set_raw(MY_BINARY, myself.as_os_str(), true, true)
            .unwrap();
        ctx.set_raw(
            AGENT_SCRIPT_DIR,
            agent_script_directory.as_os_str(),
            true,
            true,
        )
        .unwrap();
        ctx.set_raw(ARTIFACT_DIR, artifact_directory.as_os_str(), true, true)
            .unwrap();
        ctx.set_raw(ROOT_DIR, root_directory.as_os_str(), true, true)
            .unwrap();
        ctx.set_raw(SCRATCH_DIR, scratch_directory.as_os_str(), true, true)
            .unwrap();
        ctx.set(SYSTEM_NAME, name, true, false).unwrap();
        ctx.set_raw(WORK_DIR, work_directory.as_os_str(), true, true)
            .unwrap();

        Ok(ctx)
    }

    // Getters:
    pub fn timestamp(&self) -> String {
        self.variables.get(TIMESTAMP).unwrap()
    }

    pub fn version(&self) -> String {
        self.variables.get(VERSION).unwrap()
    }

    // Generic functions:
    pub fn get(&self, name: &str) -> Option<String> {
        self.variables.get(name)
    }

    pub fn set(
        &mut self,
        name: &str,
        value: &str,
        is_read_only: bool,
        is_internal: bool,
    ) -> anyhow::Result<()> {
        self.variables.set(name, value, is_read_only, is_internal)
    }
}

impl std::fmt::Display for SystemContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SystemContext {{")?;
        if self.variables.is_empty() {
            writeln!(f, "  variables = {{}},")?;
        } else {
            writeln!(f, "  variables = {{{}}},", self.variables)?;
        }
        if self.commands.is_empty() {
            writeln!(f, "  commands = {{}},")?;
        } else {
            writeln!(f, "  commands = {{{}}},", self.commands)?;
        }
        writeln!(f, "}}")
    }
}

impl SystemContext {
    // Getters:
    pub fn printer(&self) -> Rc<crate::printer::Printer> {
        self.printer.clone()
    }

    pub fn agent_script_directory(&self) -> Option<PathBuf> {
        self.get_raw(AGENT_SCRIPT_DIR).map(PathBuf::from)
    }

    pub fn artifact_directory(&self) -> Option<PathBuf> {
        self.get_raw(ARTIFACT_DIR).map(PathBuf::from)
    }

    pub fn busybox_binary(&self) -> Option<PathBuf> {
        self.get_raw(BUSYBOX_BINARY).map(PathBuf::from)
    }

    pub fn my_binary(&self) -> Option<PathBuf> {
        self.get_raw(MY_BINARY).map(PathBuf::from)
    }

    pub fn root_directory(&self) -> Option<PathBuf> {
        self.get_raw(ROOT_DIR).map(PathBuf::from)
    }

    pub fn scratch_directory(&self) -> Option<PathBuf> {
        self.get_raw(SCRATCH_DIR).map(PathBuf::from)
    }

    pub fn system_name(&self) -> Option<String> {
        self.get(SYSTEM_NAME)
    }

    pub fn timestamp(&self) -> String {
        self.get(TIMESTAMP).unwrap()
    }

    pub fn version(&self) -> String {
        self.get(VERSION).unwrap()
    }

    pub fn work_directory(&self) -> Option<PathBuf> {
        self.get_raw(WORK_DIR).map(PathBuf::from)
    }

    // Generic functions:
    pub fn get(&self, name: &str) -> Option<String> {
        self.variables.get(name)
    }

    pub fn get_raw(&self, name: &str) -> Option<OsString> {
        self.variables.get_raw(name)
    }

    pub fn set(
        &mut self,
        name: &str,
        value: &str,
        is_read_only: bool,
        is_internal: bool,
    ) -> anyhow::Result<()> {
        self.variables.set(name, value, is_read_only, is_internal)
    }

    pub fn set_raw(
        &mut self,
        name: &str,
        value: &OsStr,
        is_read_only: bool,
        is_internal: bool,
    ) -> anyhow::Result<()> {
        self.variables
            .set_raw(name, value, is_read_only, is_internal)
    }

    pub fn iter(&self) -> impl Iterator<Item = ContextEntry> + '_ {
        self.variables.iter().map(|(k, cd)| ContextEntry {
            name: k.to_string_lossy().to_string(),
            value: cd.value.to_string_lossy().to_string(),
            is_read_only: cd.is_read_only,
            is_internal: cd.is_internal,
        })
    }

    pub fn command_manager_mut(&mut self) -> &mut crate::commands::CommandManager {
        &mut self.commands
    }

    pub fn command_manager(&self) -> &crate::commands::CommandManager {
        &self.commands
    }
}
