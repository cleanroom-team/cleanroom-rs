// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Phases {
    Prepare,
    Install,
    Polish,
    Test,
    GenerateArtifacts,
    TestArtifacts,
}

impl std::fmt::Display for Phases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let debug = format!("{:?}", self).to_lowercase();
        write!(f, "{debug}")
    }
}

impl Phases {
    fn iter() -> PhasesIter {
        PhasesIter(None)
    }
}

pub struct PhasesIter(Option<Phases>);

impl Iterator for PhasesIter {
    type Item = Phases;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.0 {
            None => Some(Phases::Prepare),
            Some(Phases::Prepare) => Some(Phases::Install),
            Some(Phases::Install) => Some(Phases::Polish),
            Some(Phases::Polish) => Some(Phases::Test),
            Some(Phases::Test) => Some(Phases::GenerateArtifacts),
            Some(Phases::GenerateArtifacts) => Some(Phases::TestArtifacts),
            Some(Phases::TestArtifacts) => None,
        };
        if next.is_some() {
            self.0 = next.clone();
        }
        next
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SubPhases {
    Pre,
    Main,
    Post,
}

impl std::fmt::Display for SubPhases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubPhases::Pre => write!(f, "pre"),
            SubPhases::Main => write!(f, "main"),
            SubPhases::Post => write!(f, "post"),
        }
    }
}

impl SubPhases {
    pub fn iter() -> SubPhasesIter {
        SubPhasesIter(None)
    }
}

pub struct SubPhasesIter(Option<SubPhases>);

impl Iterator for SubPhasesIter {
    type Item = SubPhases;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.0 {
            None => Some(SubPhases::Pre),
            Some(SubPhases::Pre) => Some(SubPhases::Main),
            Some(SubPhases::Main) => Some(SubPhases::Post),
            Some(SubPhases::Post) => None,
        };
        if next.is_some() {
            self.0 = next.clone();
        }
        next
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
            assert!(pn.chars().all(|c| c.is_ascii_lowercase()));
        }
        assert_eq!(known_names.len(), 6);
    }

    #[test]
    fn test_sub_phase_names() {
        let mut known_names = HashSet::new();
        for sp in SubPhases::iter() {
            let spn = sp.to_string();

            assert!(known_names.insert(spn.clone()));
            assert!(spn.chars().all(|c| c.is_ascii_lowercase()))
        }
        assert_eq!(known_names.len(), 3);
    }
}
