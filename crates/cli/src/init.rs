// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

static INITIALIZE_FILES: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/initialize_files");

fn write_out_example_files(
    src_dir: &include_dir::Dir<'static>,
    dest_dir: &Path,
    distribution: &crate::Distributions,
) -> anyhow::Result<()> {
    let distribution_str = format!("{distribution:?}_").to_lowercase();
    let all_str = String::from("all_");

    for e in src_dir.entries() {
        let name = e
            .path()
            .file_name()
            .context("Failed to read example config")?
            .to_string_lossy()
            .to_string();
        if let Some(included) = name
            .strip_prefix(&distribution_str)
            .or_else(|| name.strip_prefix(&all_str))
        {
            match &e {
                include_dir::DirEntry::Dir(d) => {
                    let new_dest_dir = dest_dir.join(included);
                    std::fs::create_dir(&new_dest_dir)
                        .context("Failed to create sub-directory {new_dest_dir:}")?;
                    write_out_example_files(d, &new_dest_dir, distribution)?;
                }
                include_dir::DirEntry::File(f) => {
                    let mut out = std::fs::File::create(&dest_dir.join(included))
                        .context("Failed to open {included}")?;
                    out.write_all(f.contents())
                        .context("Failed to write {included}")?;
                }
            }
        }
    }
    Ok(())
}

fn find_busybox(passed_in: &Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(bb) = passed_in {
        return bb.canonicalize().context(format!(
            "Failed to canonicalize {passed_in:?}: Does it exist?"
        ));
    }
    let searched_for =
        util::find_in_path("busybox").context("Failed to look for busybox in PATH")?;
    searched_for.ok_or_else(|| anyhow::anyhow!("busybox not found in PATH"))
}

pub fn initialize(
    busybox_binary: &Option<PathBuf>,
    distribution: &crate::Distributions,
    directory: &Path,
) -> anyhow::Result<()> {
    let myself = std::env::current_exe()
        .context("Failed to find current executable path")?
        .canonicalize()
        .context("Failed to get canonical path to my own binary")?;

    let directory = directory.canonicalize().context(format!(
        "Failed to canonicalize {directory:?}: Does it exist?"
    ))?;

    if distribution == &crate::Distributions::Unknown {
        println!("Warning: You are using an unsupported distribution: You are on your own!");
    }

    if directory.exists() {
        if !directory.is_dir() {
            return Err(anyhow::anyhow!(format!(
                "{directory:?} exists but is not a directory"
            )));
        } else if directory.read_dir()?.next().is_some() {
            return Err(anyhow::anyhow!(format!(
                "{directory:?} exists but is non-empty directory"
            )));
        }
    }
    let busybox = find_busybox(busybox_binary)?;

    write_out_example_files(&INITIALIZE_FILES, &directory, distribution)?;

    let mut shell = std::process::Command::new(&busybox)
        .arg("sh")
        .arg("-e")
        .arg(directory.join("setup.sh"))
        .arg(&directory)
        .arg(&busybox)
        .arg(&myself)
        .current_dir(directory)
        .spawn()
        .context("Failed to start busybox with setup script")?;

    let exit_status = shell.wait().context("Setup script failed to run")?;
    if exit_status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Setup script quit with an error"))
    }
}
