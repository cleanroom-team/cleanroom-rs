// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

static INITIALIZE_FILES: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/initialize_files");

fn run_git(
    git: &Option<PathBuf>,
    directory: &Path,
    args: &[&str],
    context: &str,
) -> anyhow::Result<()> {
    if let Some(git) = git {
        std::process::Command::new(git)
            .args(args)
            .current_dir(directory)
            .output()
            .context("Failed to run git: {context}")
            .map(|_| ())
    } else {
        println!("Note: Git not found. {context}");
        Ok(())
    }
}

pub fn initialize(distribution: &crate::Distributions, directory: &Path) -> anyhow::Result<()> {
    let directory = directory
        .canonicalize()
        .context("Failed to canonicalize {directory:?}")?;

    if distribution == &crate::Distributions::Unknown {
        println!("Warning: You are using an unsupported distribution: You are on your own!");
    }

    if directory.exists() && !directory.is_dir() {
        return Err(anyhow::anyhow!(format!(
            "{directory:?} exists but is not a directory"
        )));
    }
    if directory.read_dir()?.next().is_some() {
        return Err(anyhow::anyhow!(format!(
            "{directory:?} exists but is non-empty directory"
        )));
    }

    if !directory.exists() {
        std::fs::create_dir_all(&directory).context("Failed to create {directory:?}")?;
    }

    let git = util::find_in_path("git").context("Failed to look for git")?;
    run_git(
        &git,
        &directory,
        &["init", "."],
        "Did not put {directory:?} under version control",
    )?;

    let env_file = ".env";
    let busybox = util::find_in_path("busybox").context("Failed to find busybox")?;
    let busybox_str = if let Some(busybox) = &busybox {
        busybox.to_string_lossy().to_string()
    } else {
        println!(
            "Warning: busybox not found: Please install it and edit CLRM_BUSYBOX in {:?}",
            directory.join(env_file)
        );
        String::from("<unknown>")
    };

    let mut env_out =
        std::fs::File::create(&directory.join(env_file)).context("Failed to open {env_file}")?;

    let comment = if busybox.is_some() { "# " } else { "" };
    let artifacts_str = directory.join("artifacts").to_string_lossy().to_string();
    writeln!(
        env_out,
        r#"export CLRM_ARTIFACT_DIR="{artifacts_str}"
# export CLRM_BOOTSTRAP_DIR="<unknown>"
# export CLRM_BOOTSTRAP_IMAGE="<unknown>"
{comment}export CLRM_BUSYBOX="{busybox_str}"
export CLRM_WORK_DIR=/var/tmp
"#
    )
    .context("Failed to write {env_file}")?;
    drop(env_out);

    let distribution_str = format!("{distribution:?}_").to_lowercase();

    for f in INITIALIZE_FILES.files() {
        let name = f.path().file_name().unwrap().to_string_lossy().to_string();
        if let Some(include) = name.strip_prefix(&distribution_str) {
            let mut out = std::fs::File::create(&directory.join(include))
                .context("Failed to open {include}")?;
            out.write_all(f.contents())
                .context("Failed to write {include}")?;
        }
    }

    std::fs::create_dir(directory.join("artifacts"))
        .context("Failed to create the artifacts directory")?;
    let mut keeper = std::fs::File::create(directory.join("artifacts/.keep_me"))
        .context("Failed to create directory keeper for git")?;
    writeln!(keeper, "# Make git keep the directory").context("Failed to write keeper file")?;
    drop(keeper);

    let mut gitignore = std::fs::File::create(directory.join(".gitignore"))
        .context("Failed to create .gitignore file")?;
    writeln!(gitignore, "artifacts/").context("Failed to write into .gitignore file")?;
    drop(gitignore);

    run_git(&git, &directory, &["add", "."], "Failed to add files")?;

    println!("You are ready to roll now... check the README.md for instructions");

    Ok(())
}
