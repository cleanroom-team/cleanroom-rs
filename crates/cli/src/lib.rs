// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Phases {
    Invalid,
    Prepare,
    Install,
    Polish,
    Test,
    GenerateArtifacts,
    TestArtifacts,
}

impl std::fmt::Display for Phases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phases::Invalid => write!(f, "<invalid>"),
            _ => {
                let debug = format!("{:?}", self).to_lowercase();
                write!(f, "{debug}")
            }
        }
    }
}

impl Iterator for Phases {
    type Item = Phases;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Phases::Invalid => Some(Phases::Prepare),
            Phases::Prepare => Some(Phases::Install),
            Phases::Install => Some(Phases::Polish),
            Phases::Polish => Some(Phases::Test),
            Phases::Test => Some(Phases::GenerateArtifacts),
            Phases::GenerateArtifacts => Some(Phases::TestArtifacts),
            Phases::TestArtifacts => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SubPhases {
    Invalid,
    Pre,
    Main,
    Post,
}

impl std::fmt::Display for SubPhases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubPhases::Invalid => write!(f, "<invalid>"),
            SubPhases::Pre => write!(f, "pre"),
            SubPhases::Main => write!(f, "main"),
            SubPhases::Post => write!(f, "post"),
        }
    }
}

impl Iterator for SubPhases {
    type Item = SubPhases;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SubPhases::Invalid => Some(SubPhases::Pre),
            SubPhases::Pre => Some(SubPhases::Main),
            SubPhases::Main => Some(SubPhases::Post),
            SubPhases::Post => None,
        }
    }
}

pub mod agent;
pub mod agent_runner;
pub mod commands;
pub mod context;
pub mod printer;
pub mod scripts;
