// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `Context` to run in

pub struct Context {
    timestamp: String,
    system: Option<SystemContext>,
}

impl Context {
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    // SystemContext:

    pub fn set_system(&mut self, system_ctx: SystemContext) {
        assert!(self.system.is_none());
        self.system = Some(system_ctx);
    }
}

impl Default for Context {
    fn default() -> Self {
        let timestamp = format!("{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));

        Self {
            timestamp,
            system: None,
        }
    }
}

#[derive(Debug)]
pub struct SystemContext {
    pub name: String,
}
