// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use clap::ValueEnum;

#[derive(Debug, Clone, Eq, PartialEq, clap::ValueEnum)]
#[clap(rename_all = "snake_case")]
pub enum Phases {
    Prepare,
    Install,
    Polish,
    Test,
    BuildArtifacts,
    TestArtifacts,
}

impl Phases {
    pub fn iter() -> impl Iterator<Item = &'static Phases> {
        Phases::value_variants().iter()
    }
}

impl std::fmt::Display for Phases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.to_possible_value()
                .map(|pv| pv.get_name().to_string())
                .unwrap_or_else(|| "<unknown>".to_string())
        )
    }
}

pub mod agent;
pub mod agent_runner;
pub mod commands;
pub mod context;
pub mod printer;
pub mod scripts;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_phase_names() {
        let mut known_names = HashSet::new();
        for p in Phases::iter() {
            let pn = p.to_string();

            assert!(known_names.insert(pn.clone()));
            assert!(pn.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
        assert_eq!(known_names.len(), 6);
    }
}
