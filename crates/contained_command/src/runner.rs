// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

// cSpell: ignore chdir resolv

use std::{
    ffi::{OsStr, OsString},
    os::unix::{ffi::OsStrExt, fs::MetadataExt, process::ExitStatusExt},
    path::{Path, PathBuf},
};

use tokio::io::AsyncBufReadExt;

// ----------------------------------------------------------------------
// - Runtime:
// ----------------------------------------------------------------------

/// A trait that all the different run-times need to implement
pub trait Runtime {
    /// Run a `Command`
    ///
    /// # Errors
    ///
    /// Things can go wrong!
    fn run(
        &self,
        container_data: &ContainerData,
        command: &crate::Command,
    ) -> crate::Result<(tokio::process::Child, PathBuf, Vec<OsString>)>;
}

/// Helper struct to containerize into `systemd-nspawn`
#[derive(Clone, Debug)]
pub struct Nspawn {
    sudo_binary: Option<PathBuf>,
    nspawn_binary: PathBuf,
    root_directory: PathBuf,
}

impl Nspawn {
    /// Create a `systemd-nspawn` container, providing all binaries needed to
    /// run a `Command`
    ///
    /// # Errors
    ///
    /// Things can go wrong!
    pub fn custom_binary(
        nspawn_binary: PathBuf,
        sudo_binary: Option<PathBuf>,
        root_directory: &Path,
    ) -> crate::Result<Runner<Nspawn>> {
        let root_directory = util::resolve_directory(root_directory)?;
        let root_directory = root_directory.canonicalize()?;
        Ok(Runner::new(Nspawn {
            sudo_binary,
            nspawn_binary,
            root_directory,
        }))
    }

    /// Create a `systemd-nspawn` container with default binary paths
    ///
    /// # Errors
    ///
    /// Things can go wrong!
    pub fn default_runner(root_directory: &Path) -> crate::Result<Runner<Nspawn>> {
        let nspawn_binary = util::require_binary("systemd-nspawn")?;
        let sudo_binary = if util::is_effective_root() {
            None
        } else {
            Some(util::require_binary("sudo")?)
        };
        Self::custom_binary(nspawn_binary, sudo_binary, root_directory)
    }

    /// Validate values stored in `self`
    fn validate(&self) -> crate::Result<()> {
        if !util::is_executable_file(&self.nspawn_binary) {
            return Err(crate::Error::CommandNotExecutable(
                self.nspawn_binary.clone(),
            ));
        }
        if let Some(sudo) = &self.sudo_binary {
            if !util::is_executable_file(sudo) {
                return Err(crate::Error::CommandNotExecutable(sudo.clone()));
            }
        } else if !util::is_effective_root() {
            return Err(crate::Error::RootNeeded(
                "Not run as root and no sudo command provided".to_string(),
            ));
        }
        if !self
            .root_directory
            .metadata()
            .ok()
            .is_some_and(|md| md.is_dir())
        {
            return Err(crate::Error::ContainmentFailure(
                "\"{root_directory}\" is not a directory".to_string(),
            ));
        }
        Ok(())
    }

    /// Prepare `root_directory` for nspawn
    fn prepare(&self) -> crate::Result<()> {
        let usr_dir = self.root_directory.join("usr");
        if !usr_dir.exists() {
            std::fs::create_dir(usr_dir)?;
        }

        Ok(())
    }

    fn environment_arguments(
        container_data: &ContainerData,
        command: &crate::Command,
    ) -> Vec<OsString> {
        container_data
            .environment
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(
                command
                    .environment
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone())),
            )
            .map(|(k, v)| {
                let mut result = OsString::from("--setenv=");
                result.push(k);
                result.push("=");
                result.push(v);
                result
            })
            .collect()
    }

    fn binding_arguments(
        container_data: &ContainerData,
        command: &crate::Command,
    ) -> Vec<OsString> {
        container_data
            .bindings
            .iter()
            .chain(command.bindings.iter())
            .map(|b| match b {
                crate::Binding::TmpFS(target) => {
                    let mut result = OsString::from("--tmpfs=");
                    result.push(target.as_os_str());
                    result
                }
                crate::Binding::RW(mapping) => {
                    let mut result = OsString::from("--bind=");
                    result.push(mapping.source.as_os_str());
                    result.push(OsString::from(":"));
                    result.push(mapping.target.as_os_str());
                    result
                }
                crate::Binding::RO(mapping) => {
                    let mut result = OsString::from("--bind-ro=");
                    result.push(mapping.source.as_os_str());
                    result.push(OsString::from(":"));
                    result.push(mapping.target.as_os_str());
                    result
                }
                crate::Binding::Inaccessible(target) => {
                    let mut result = OsString::from("--inaccessible=");
                    result.push(target.as_os_str());
                    result
                }
                crate::Binding::Overlay(mapping) => {
                    let mut result = OsString::from("--overlay=");
                    for s in &mapping.sources {
                        result.push(s.as_os_str());
                        result.push(OsString::from(":"));
                    }
                    result.push(mapping.target.as_os_str());
                    result
                }
                crate::Binding::OverlayRO(mapping) => {
                    let mut result = OsString::from("--overlay-ro=");
                    for s in &mapping.sources {
                        result.push(s.as_os_str());
                        result.push(OsString::from(":"));
                    }
                    result.push(mapping.target.as_os_str());
                    result
                }
            })
            .collect()
    }
}

impl Runtime for Nspawn {
    fn run(
        &self,
        container_data: &ContainerData,
        command: &crate::Command,
    ) -> crate::Result<(tokio::process::Child, PathBuf, Vec<OsString>)> {
        self.validate()?;
        self.prepare()?;

        let (executable, mut args) = if let Some(sudo) = &self.sudo_binary {
            (
                sudo.as_os_str().to_os_string(),
                vec![self.nspawn_binary.as_os_str().to_os_string()],
            )
        } else {
            (self.nspawn_binary.as_os_str().to_os_string(), vec![])
        };

        args.extend_from_slice(&[
            OsString::from("--quiet"),
            OsString::from("--volatile=yes"),
            OsString::from("--settings=off"),
            OsString::from("--register=off"),
            OsString::from("--resolv-conf=off"),
            OsString::from("--timezone=off"),
            OsString::from("--link-journal=no"),
            OsString::from("--console=pipe"),
        ]);

        if let Some(machine_id) = container_data.machine_id {
            let mut tmp = OsString::from("--uuid=");
            tmp.push(OsStr::from_bytes(&machine_id[..]));
            args.push(tmp);
        }

        if !container_data.enable_network {
            args.push(OsString::from("--private-network"));
        }
        args.extend_from_slice(&Self::environment_arguments(container_data, command));
        args.extend_from_slice(&Self::binding_arguments(container_data, command));

        if container_data.enable_private_users {
            let effective_uid = std::fs::metadata("/proc/self")
                .map(|m| m.uid())
                .expect("/proc/self should be accessible to this process!");
            args.push(OsString::from(
                format!("--private-users={effective_uid}:1",),
            ));
        }

        if let Some(current_directory) = &command.current_directory {
            let mut dir_arg = OsString::from("--chdir=");
            dir_arg.push(current_directory.as_os_str());
            args.push(dir_arg);
        }

        let mut dir_arg = OsString::from("--directory=");
        dir_arg.push(self.root_directory.as_os_str());
        args.push(dir_arg);

        // Actual Command:
        args.push(command.command.as_os_str().to_os_string());
        args.append(&mut command.arguments.clone());

        Ok((
            tokio::process::Command::new(executable.clone())
                .args(args.clone())
                .env_clear()
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?,
            PathBuf::from(executable),
            args,
        ))
    }
}

// ----------------------------------------------------------------------
// - Runner:
// ----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ContainerData {
    machine_id: Option<[u8; 32]>,
    bindings: Vec<crate::Binding>,
    environment: Vec<(OsString, OsString)>,
    enable_network: bool,
    enable_private_users: bool,
    current_directory: PathBuf,
}

/// The `Runner` that will run a `Command` in a container
#[derive(Clone, Debug)]
pub struct Runner<RT: Clone + std::fmt::Debug + Runtime> {
    runtime: RT,
    container_data: ContainerData,
}

impl<RT: Clone + std::fmt::Debug + Runtime> Runner<RT> {
    /// Create a `Runner` that will create a container in `root_directory`
    #[must_use]
    pub fn new(runtime: RT) -> Self {
        Runner {
            runtime,
            container_data: ContainerData {
                machine_id: None,
                environment: Vec::new(),
                bindings: Vec::new(),
                enable_network: false,
                enable_private_users: true,
                current_directory: PathBuf::from("/"),
            },
        }
    }

    /// Set the `machine_id` of the container
    #[must_use]
    pub fn machine_id(mut self, id: [u8; 32]) -> Self {
        self.container_data.machine_id = Some(id);
        self
    }

    /// Set current work directory for the `Command`
    #[must_use]
    pub fn current_dir<P: AsRef<OsStr>>(mut self, dir: P) -> Self {
        self.container_data.current_directory = PathBuf::from(&dir);
        self
    }

    /// Set bindings
    #[must_use]
    pub fn set_bindings(mut self, bind: &[crate::Binding]) -> Self {
        self.container_data.bindings = bind.to_vec();
        self
    }

    /// Add one binding
    #[must_use]
    pub fn binding(mut self, bind: crate::Binding) -> Self {
        self.container_data.bindings.push(bind);
        self
    }

    /// Set environment
    ///
    /// The environment starts out _empty_!
    #[must_use]
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.container_data
            .environment
            .push((OsString::from(&key), OsString::from(&value)));
        self
    }

    /// Enable networking
    #[must_use]
    pub fn with_network(mut self) -> Self {
        self.container_data.enable_network = true;
        self
    }

    /// Share users with the container
    #[must_use]
    pub fn share_users(mut self) -> Self {
        self.container_data.enable_private_users = false;
        self
    }

    /// Run a `Command`
    ///
    /// # Errors
    ///
    /// Things can go wrong!
    pub fn run_raw(
        &self,
        command: &crate::Command,
    ) -> crate::Result<(tokio::process::Child, PathBuf, Vec<OsString>)> {
        self.runtime.run(&self.container_data, command)
    }

    /// Run a `Command`, process output and report result
    ///
    /// # Errors
    ///
    /// Things can go wrong!
    ///
    /// # Panics
    ///
    /// Sometimes
    pub async fn run(
        &self,
        command: &crate::Command,
        trace: &dyn Fn(&str),
        error: &dyn Fn(&str),
        stdout: &mut dyn FnMut(&'_ str),
        stderr: &mut dyn FnMut(&'_ str),
    ) -> crate::Result<()> {
        let (mut child, executable, args) = self.run_raw(command)?;
        trace(&format!(
            "Running {executable:?} {} ...",
            args.join(&OsString::from(" ")).to_string_lossy()
        ));

        let stdin = child.stdin.take().unwrap();
        drop(stdin);

        let mut stdout_reader = tokio::io::BufReader::new(child.stdout.take().unwrap()).lines();
        let mut stderr_reader = tokio::io::BufReader::new(child.stderr.take().unwrap()).lines();

        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    if let Ok(Some(line)) = result { stdout(&line) }
                }
                result = stderr_reader.next_line() => {
                    if let Ok(Some(line)) = result { stderr(&line) }
                }
                result = child.wait() => {
                    match result {
                        Ok(exit_status) => {
                            match exit_status.code() {
                                Some(exit_code) if exit_code == command. expected_exit_code => {
                                    trace(&format!("Child process finished with expected exit code {}", exit_code));
                                    return Ok(());
                                },
                                Some(exit_code) => {
                                    let message = "Command finished with unexpected exit code".to_string();
                                    error(&message);
                                    return Err(crate::Error::CommandFailed { command: executable.clone(), args, message, status: Some(exit_code) });
                                },
                                None => {
                                    let message = format!("Command was interrupted by signal {}", exit_status.signal().unwrap());
                                    error(&message);
                                    return Err(crate::Error::CommandFailed { command: executable.clone(), args, message, status: None });
                                },
                            }
                        },
                        Err(e) => {
                            let message = format!("Command failed: {e}");
                            error(&message);
                            return Err(crate::Error::CommandFailed { command: command.command.clone(), args, message, status: None});
                        },
                    }
                }
            };
        }
    }
}
