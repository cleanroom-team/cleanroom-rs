// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `Context` to run in

use crate::commands::{CommandName, VariableName};
use crate::printer::Printer;

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context as AhContext};
use contained_command::RunEnvironment;

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
    can_inherit: bool,
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
        can_inherit: bool,
    ) -> anyhow::Result<()> {
        self.set_raw(
            name,
            &OsString::from(value),
            is_read_only,
            is_internal,
            can_inherit,
        )
    }

    fn set_raw(
        &mut self,
        name: &str,
        value: &OsStr,
        is_read_only: bool,
        is_internal: bool,
        can_inherit: bool,
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
                    can_inherit,
                },
            );
            Ok(())
        } else {
            Err(anyhow!("Invalid character in variable name \"{name}\""))
        }
    }

    fn inherit(&self) -> Self {
        Self(
            self.iter()
                .filter(|(_, v)| v.can_inherit)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
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
    timestamp: String,
    version: Option<String>,
    printer: Printer,
    commands: crate::commands::CommandManager,
}

impl ContextBuilder {
    #[cfg(test)]
    pub fn new_test() -> Self {
        use crate::printer::LogLevel;

        Self {
            timestamp: format!("{}", chrono::Local::now().format("%Y%m%d.%H%M")),
            version: None,
            printer: Printer::new(&LogLevel::Off, false),
            commands: crate::commands::CommandManagerBuilder::default().build(),
        }
    }

    pub fn new(printer: Printer, commands: crate::commands::CommandManager) -> Self {
        Self {
            timestamp: format!("{}", chrono::Local::now().format("%Y%m%d.%H%M")),
            version: None,
            printer,
            commands,
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

    pub fn build(self) -> anyhow::Result<Context> {
        let v = if let Some(v) = self.version {
            v.clone()
        } else {
            self.timestamp.clone()
        };

        let myself = std::env::current_exe().context("Failed to find current executable path")?;
        let myself = myself
            .canonicalize()
            .context("Failed to canonicalize my own binary path")?;
        if !util::is_executable_file(&myself) {
            return Err(anyhow!("{myself:?} is no file or not executable"));
        }

        Ok(Context {
            variables: ContextMap(BTreeMap::from([
                (
                    OsString::from(TIMESTAMP),
                    ContextData {
                        value: OsString::from(self.timestamp),
                        is_read_only: true,
                        is_internal: false,
                        can_inherit: true,
                    },
                ),
                (
                    OsString::from(VERSION),
                    ContextData {
                        value: OsString::from(v),
                        is_read_only: true,
                        is_internal: false,
                        can_inherit: true,
                    },
                ),
                (
                    OsString::from(MY_BINARY),
                    ContextData {
                        value: myself.into_os_string(),
                        is_read_only: true,
                        is_internal: true,
                        can_inherit: true,
                    },
                ),
            ])),
            printer: self.printer,
            commands: self.commands,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    variables: ContextMap,
    printer: Printer,
    commands: crate::commands::CommandManager,
}

pub struct BuildContext {
    printer: Printer,
    commands: crate::commands::CommandManager,
    bootstrap_environment: crate::RunEnvironment,
    variables: ContextMap,
    networked_phases: Vec<crate::Phases>,
    scratch_dir: tempfile::TempDir,
    debug_options: Vec<crate::DebugOptions>,
    dependencies: Vec<(VariableName, CommandName)>,
}

const ARTIFACTS_DIR: &str = "ARTIFACTS_DIR";
const BUSYBOX_BINARY: &str = "BUSYBOX_BINARY";
const MY_BINARY: &str = "MY_BINARY";
const ROOT_DIR: &str = "ROOT_DIR";
const TIMESTAMP: &str = "TIMESTAMP";
const VERSION: &str = "VERSION";
const WORK_DIR: &str = "WORK_DIR";

impl Context {
    #[cfg(test)]
    pub fn test_system(&self) -> BuildContext {
        let mut ctx = BuildContext {
            commands: self.commands.clone(),
            printer: self.printer.clone(),
            variables: self.variables.inherit(),
            bootstrap_environment: crate::RunEnvironment::Directory(PathBuf::from(
                "/tmp/bootstrap_dir",
            )),
            networked_phases: Vec::default(),
            scratch_dir: tempfile::TempDir::new().unwrap(),
            debug_options: vec![],
            dependencies: vec![],
        };

        ctx.variables
            .set(BUSYBOX_BINARY, "/usr/bin/busybox", true, true, true)
            .unwrap();
        ctx.variables
            .set(ARTIFACTS_DIR, "/foo/artifacts", true, true, false)
            .unwrap();
        ctx.variables
            .set(ROOT_DIR, "/foo/work/XXXX/root_fs", true, true, false)
            .unwrap();
        ctx.variables
            .set(WORK_DIR, "/foo/work", true, true, false)
            .unwrap();

        ctx
    }

    // Setter:
    pub fn create_build_context(
        &self,
        work_directory: &Path,
        artifacts_directory: &Path,
        busybox_binary: &Path,
        bootstrap_environment: crate::RunEnvironment,
        networked_phases: &[crate::Phases],
        debug_options: &[crate::DebugOptions],
    ) -> anyhow::Result<BuildContext> {
        let artifacts_directory = util::resolve_directory(artifacts_directory)
            .context("Failed to resolve work directory")?;
        let work_directory =
            util::resolve_directory(work_directory).context("Failed to resolve work directory")?;

        let scratch_dir = tempfile::TempDir::with_prefix_in("scratch-", &work_directory)
            .context("Failed to create scratch directory")?;

        let root_directory = scratch_dir.path().join("root_fs");
        std::fs::create_dir(&root_directory)
            .context("Failed to create root directory in scratch directory")?;

        let busybox_binary = busybox_binary
            .canonicalize()
            .context("Failed to canonicalize busybox binary")?;
        if !util::is_executable_file(&busybox_binary) {
            return Err(anyhow!("{busybox_binary:?} is no file or not executable"));
        }

        let mut networked_phases = networked_phases.to_vec();
        networked_phases.sort_unstable();
        networked_phases.dedup();

        let debug_options = {
            let mut options = debug_options.to_vec();
            options.sort_unstable();
            options.dedup();
            options.reserve_exact(0);
            options
        };

        let mut ctx = BuildContext {
            commands: self.commands.clone(),
            printer: self.printer.clone(),
            variables: self.variables.inherit(),
            bootstrap_environment,
            networked_phases,
            scratch_dir,
            debug_options,
            dependencies: Default::default(),
        };

        ctx.variables
            .set_raw(BUSYBOX_BINARY, busybox_binary.as_os_str(), true, true, true)
            .unwrap();
        ctx.variables
            .set_raw(
                ARTIFACTS_DIR,
                artifacts_directory.as_os_str(),
                true,
                true,
                false,
            )
            .unwrap();
        ctx.variables
            .set_raw(ROOT_DIR, root_directory.as_os_str(), true, true, false)
            .unwrap();
        ctx.variables
            .set_raw(WORK_DIR, work_directory.as_os_str(), true, true, false)
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
        self.variables
            .set(name, value, is_read_only, is_internal, false)
    }
}

impl std::fmt::Display for BuildContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SystemContext {{")?;
        if self.variables.is_empty() {
            writeln!(f, "  variables = {{}},")?;
        } else {
            writeln!(f, "  variables = {{\n{}  }},", self.variables)?;
        }
        if self.commands.is_empty() {
            writeln!(f, "  commands = {{}},")?;
        } else {
            writeln!(f, "  commands = {{\n{}  }},", self.commands)?;
        }
        if self.networked_phases.is_empty() {
            writeln!(f, "  networked_phases = {{}},")?;
        } else {
            writeln!(
                f,
                "  networked_phases  = {{ {:?} }},",
                self.networked_phases
            )?;
        }
        writeln!(f, "}}")
    }
}

impl BuildContext {
    // Dependency management:
    pub fn take_dependencies(&mut self) -> Vec<(VariableName, CommandName)> {
        std::mem::take(&mut self.dependencies)
    }

    // Add a new dependency
    pub(crate) fn add_dependency(
        &mut self,
        key: VariableName,
        value: CommandName,
    ) -> anyhow::Result<()> {
        eprintln!("ADD_DEPENDENCY: {key} => {value}");
        if self.dependencies.iter().any(|(k, _)| k == &key) {
            return Err(anyhow!("{key} already registered as a dependency"));
        }
        self.dependencies.push((key, value));

        Ok(())
    }

    pub fn create_dependent_context(
        &self,
        artifacts_dir: &Path,
        name: &VariableName,
    ) -> anyhow::Result<Self> {
        let artifacts_directory = artifacts_dir.join(format!("deps/{}", name));
        std::fs::create_dir_all(&artifacts_directory)
            .context(format!("Failed to create artifact directory for {name}"))?;

        let scratch_dir =
            tempfile::TempDir::with_prefix_in(format!("{}-", name), &self.scratch_dir)
                .context("Failed to create scratch directory")?;

        let root_dir = scratch_dir.path().join("root_fs");

        let mut dep_ctx = BuildContext {
            commands: self.commands.clone(),
            printer: self.printer.clone(),
            variables: self.variables.inherit(),
            bootstrap_environment: self.bootstrap_environment.clone(),
            networked_phases: self.networked_phases.clone(),
            scratch_dir,
            debug_options: self.debug_options.clone(),
            dependencies: Default::default(),
        };

        dep_ctx
            .set_raw(ROOT_DIR, root_dir.as_os_str(), true, true)
            .unwrap();
        dep_ctx
            .set_raw(WORK_DIR, self.scratch_dir.path().as_os_str(), true, true)
            .unwrap();
        dep_ctx
            .set_raw(ARTIFACTS_DIR, artifacts_directory.as_os_str(), true, true)
            .unwrap();

        Ok(dep_ctx)
    }

    // Getters:
    pub fn bootstrap_environment(&self) -> &RunEnvironment {
        &self.bootstrap_environment
    }
    pub fn printer(&self) -> crate::printer::Printer {
        self.printer.clone()
    }

    pub fn artifacts_directory(&self) -> PathBuf {
        self.get_raw(ARTIFACTS_DIR).map(PathBuf::from).unwrap()
    }

    pub fn busybox_binary(&self) -> PathBuf {
        self.get_raw(BUSYBOX_BINARY).map(PathBuf::from).unwrap()
    }

    pub fn my_binary(&self) -> PathBuf {
        self.get_raw(MY_BINARY).map(PathBuf::from).unwrap()
    }

    pub fn root_directory(&self) -> PathBuf {
        self.get_raw(ROOT_DIR).map(PathBuf::from).unwrap()
    }

    pub fn scratch_directory(&self) -> PathBuf {
        self.scratch_dir.path().to_path_buf()
    }

    pub fn timestamp(&self) -> String {
        self.get(TIMESTAMP).unwrap()
    }

    pub fn version(&self) -> String {
        self.get(VERSION).unwrap()
    }

    pub fn work_directory(&self) -> PathBuf {
        self.get_raw(WORK_DIR).map(PathBuf::from).unwrap()
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
        self.variables
            .set(name, value, is_read_only, is_internal, false)
    }

    pub fn set_raw(
        &mut self,
        name: &str,
        value: &OsStr,
        is_read_only: bool,
        is_internal: bool,
    ) -> anyhow::Result<()> {
        self.variables
            .set_raw(name, value, is_read_only, is_internal, false)
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

    pub fn wants_network(&self, phase: &crate::Phases) -> bool {
        self.networked_phases.contains(phase)
    }

    pub fn check_debug_option(&self, debug_option: &crate::DebugOptions) -> bool {
        self.debug_options.contains(debug_option)
    }
}
