// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Clone)]
pub enum Phases {
    Prepare,
    Install,
    Polish,
    Test,
}

pub mod agent;
pub mod agent_runner;
pub mod commands;
pub mod context;
pub mod printer;
pub mod scripts;
