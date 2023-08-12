// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `Context` to run in

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context as AhContext};

#[derive(Debug, Clone)]
pub struct Context(HashMap<OsString, OsString>);

#[derive(Debug, Clone)]
pub struct SystemContext(HashMap<OsString, OsString>);

const AGENT_SCRIPT_DIR: &str = "_AGENT_SCRIPT_DIR";
const ARTIFACT_DIR: &str = "_ARTIFACT_DIR";
const BUSYBOX_BINARY: &str = "_BUSYBOX_BINARY";
const MY_BINARY: &str = "_MY_BINARY";
const ROOT_DIR: &str = "_ROOT_DIR";
const SCRATCH_DIR: &str = "_SCRATCH_DIR";
const SYSTEM_NAME: &str = "SYSTEM_NAME";
const TIMESTAMP: &str = "TIMESTAMP";
const VERSION: &str = "VERSION";
const WORK_DIR: &str = "_WORK_DIR";

impl Context {
    pub fn new(timestamp: &Option<String>, version: &Option<String>) -> Self {
        let ts = if let Some(ts) = timestamp {
            ts.clone()
        } else {
            format!("{}", chrono::Utc::now().format("%Y%m%d%H%M%S"))
        };

        let v = if let Some(v) = version {
            v.clone()
        } else {
            ts.clone()
        };

        Self(HashMap::from([
            (OsString::from(TIMESTAMP), OsString::from(ts)),
            (OsString::from(VERSION), OsString::from(v)),
        ]))
    }

    #[cfg(test)]
    pub fn test_system(&self) -> SystemContext {
        let mut ctx = SystemContext(self.0.clone());

        ctx.set(BUSYBOX_BINARY, "/usr/bin/busybox").unwrap();
        ctx.set(MY_BINARY, "/foo/agent").unwrap();
        ctx.set(AGENT_SCRIPT_DIR, "/foo/agent_scripts").unwrap();
        ctx.set(ARTIFACT_DIR, "/foo/artifacts").unwrap();
        ctx.set(ROOT_DIR, "/foo/work/XXXX/root_fs").unwrap();
        ctx.set(SCRATCH_DIR, "/foo/work/XXXX").unwrap();
        ctx.set(SYSTEM_NAME, "test_system").unwrap();
        ctx.set(WORK_DIR, "/foo/work").unwrap();

        ctx
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

        let mut ctx = SystemContext(self.0.clone());

        ctx.set_raw(BUSYBOX_BINARY, busybox_binary.as_os_str())
            .unwrap();
        ctx.set_raw(MY_BINARY, myself.as_os_str()).unwrap();
        ctx.set_raw(AGENT_SCRIPT_DIR, agent_script_directory.as_os_str())
            .unwrap();
        ctx.set_raw(ARTIFACT_DIR, artifact_directory.as_os_str())
            .unwrap();
        ctx.set_raw(ROOT_DIR, root_directory.as_os_str()).unwrap();
        ctx.set_raw(SCRATCH_DIR, scratch_directory.as_os_str())
            .unwrap();
        ctx.set(SYSTEM_NAME, name).unwrap();
        ctx.set_raw(WORK_DIR, work_directory.as_os_str()).unwrap();

        Ok(ctx)
    }

    // Getters:
    pub fn timestamp(&self) -> &OsStr {
        self.0.get(OsStr::new(TIMESTAMP)).unwrap()
    }

    pub fn version(&self) -> &OsStr {
        self.0.get(OsStr::new(VERSION)).unwrap()
    }

    // Generic functions:
    pub fn get(&self, name: &str) -> Option<String> {
        self.get_raw(name).map(|v| v.to_string_lossy().to_string())
    }

    pub fn get_raw(&self, name: &str) -> Option<OsString> {
        self.0.get(&OsString::from(name)).cloned()
    }

    pub fn set(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        self.set_raw(name, &OsString::from(value))
    }

    pub fn set_raw(&mut self, name: &str, value: &OsStr) -> anyhow::Result<()> {
        if name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            self.0.insert(OsString::from(name), value.to_os_string());
            Ok(())
        } else {
            Err(anyhow!("Invalid character in variable name \"{name}\""))
        }
    }
}

impl std::ops::Deref for Context {
    type Target = HashMap<OsString, OsString>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(&None, &None)
    }
}

impl SystemContext {
    // Getters:
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

    pub fn timestamp(&self) -> &OsStr {
        self.0.get(OsStr::new(TIMESTAMP)).unwrap()
    }

    pub fn version(&self) -> &OsStr {
        self.0.get(OsStr::new(VERSION)).unwrap()
    }

    pub fn work_directory(&self) -> Option<PathBuf> {
        self.get_raw(WORK_DIR).map(PathBuf::from)
    }

    // Generic functions:
    pub fn get(&self, name: &str) -> Option<String> {
        self.get_raw(name).map(|v| v.to_string_lossy().to_string())
    }

    pub fn get_raw(&self, name: &str) -> Option<OsString> {
        self.0.get(&OsString::from(name)).cloned()
    }

    pub fn set(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        self.set_raw(name, &OsString::from(value))
    }

    pub fn set_raw(&mut self, name: &str, value: &OsStr) -> anyhow::Result<()> {
        if name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            self.0.insert(OsString::from(name), value.to_os_string());
            Ok(())
        } else {
            Err(anyhow!("Invalid character in variable name \"{name}\""))
        }
    }
}

impl std::ops::Deref for SystemContext {
    type Target = HashMap<OsString, OsString>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
