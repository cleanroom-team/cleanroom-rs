// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `Context` to run in

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct Context(HashMap<OsString, OsString>);

const TIMESTAMP: &str = "TIMESTAMP";
const SYSTEM_NAME: &str = "SYSTEM_NAME";
const ROOT_DIR: &str = "ROOT_DIR";

impl Context {
    pub fn timestamp(&self) -> &OsStr {
        self.0.get(OsStr::new(TIMESTAMP)).unwrap()
    }

    pub fn set_system(&mut self, name: &str, root_directory: &Path) {
        self.set(SYSTEM_NAME, name).unwrap();
        self.set_raw(ROOT_DIR, root_directory.as_os_str()).unwrap();
    }

    pub fn root_directory(&self) -> Option<PathBuf> {
        self.get_raw(ROOT_DIR).map(PathBuf::from)
    }

    pub fn system_name(&self) -> Option<String> {
        self.get(SYSTEM_NAME)
    }

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
        let timestamp = OsString::from(format!("{}", chrono::Utc::now().format("%Y%m%d%H%M%S")));

        Self(HashMap::from([(OsString::from(TIMESTAMP), timestamp)]))
    }
}
