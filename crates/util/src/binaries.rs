// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Tobias Hunger <tobias.hunger@gmail.com>

use std::{
    ffi::{OsStr, OsString},
    os::unix::{fs::MetadataExt, prelude::OsStrExt},
    path::{Path, PathBuf},
};

/// Check whether `path` refers to an executable file
pub fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let Some(mode) = path.metadata().ok().map(|f| f.mode()) else {
        return false;
    };
    (mode & 0o111) != 0
}

fn home_directory() -> crate::Result<PathBuf> {
    let value = std::env::var_os("HOME").ok_or(crate::Error::InvalidEnv(OsString::from("HOME")))?;
    let path = PathBuf::from(value);
    path.canonicalize()
        .map_err(|_| crate::Error::InvalidDirectory {
            reason: "Could not resolve HOME directory".to_string(),
            directory: path.as_os_str().to_os_string(),
        })
}

pub fn require_binary(binary_name: &str) -> crate::Result<PathBuf> {
    if let Some(executable) = find_in_path(binary_name)? {
        Ok(executable)
    } else {
        Err(crate::Error::ExecutableNotFound(binary_name.to_owned()))
    }
}

/// Find a binary in a unix-like PATH environment
pub fn find_in_path(binary_name: &str) -> crate::Result<Option<PathBuf>> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let home_dir = home_directory()?;
    for p in PathIterator::new(&path) {
        let p = resolve_directory_impl(&PathBuf::from(p), &home_dir, None)?;
        let Ok(p) = p.canonicalize() else {
            continue;
        };

        // Just skip over non-existing directories.
        if !p.exists() {
            continue;
        }

        if let Some(executable) = resolve_file_in_directory(&p, binary_name) {
            return Ok(Some(executable));
        }
    }

    Ok(None)
}

struct PathIterator<'a> {
    path: &'a OsStr,
    start_pos: usize,
}

impl<'a> Iterator for PathIterator<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        let sp = self.start_pos;
        if sp > self.path.as_bytes().len() {
            None
        } else {
            let end_pos = if let Some((end_offset, _)) = self.path.as_bytes()[sp..]
                .iter()
                .enumerate()
                .find(|(_, i)| **i == b':')
            {
                sp + end_offset
            } else {
                self.path.len()
            };
            self.start_pos = end_pos + 1;
            Some(OsStr::from_bytes(&self.path.as_bytes()[sp..end_pos]))
        }
    }
}

impl<'a> PathIterator<'a> {
    pub fn new(path: &'a OsStr) -> Self {
        Self { path, start_pos: 0 }
    }
}

fn resolve_file_in_directory(dir: &Path, binary_name: &str) -> Option<PathBuf> {
    let mut executable = dir.to_path_buf();
    executable.push(binary_name);

    is_executable_file(&executable).then_some(executable)
}

fn extend_dir(dir: &Path, base_dir: &Path, skip: usize) -> PathBuf {
    let mut result = base_dir.to_path_buf();
    dir.iter().skip(skip).for_each(|c| result.push(c));
    result
}

pub fn resolve_directory(dir: &Path) -> crate::Result<PathBuf> {
    let home_dir = home_directory()?;
    let current_dir = std::env::current_dir().ok();

    resolve_directory_impl(dir, &home_dir, current_dir)?
        .canonicalize()
        .map_err(|_| crate::Error::InvalidDirectory {
            reason: "Failed to resolve directory".to_string(),
            directory: dir.as_os_str().to_os_string(),
        })
}

fn resolve_directory_impl(
    dir: &Path,
    home_dir: &Path,
    current_dir: Option<PathBuf>,
) -> crate::Result<PathBuf> {
    if dir.starts_with("~") {
        if dir.as_os_str().len() == 1 {
            return Ok(home_dir.to_path_buf());
        } else if dir.starts_with("~/") {
            return Ok(extend_dir(dir, home_dir, 1));
        } else {
            return Err(crate::Error::InvalidDirectory {
                reason: "Other users HOME is not supported".to_string(),
                directory: dir.as_os_str().to_os_string(),
            });
        }
    }
    if dir.is_absolute() {
        return Ok(dir.to_path_buf());
    }
    if let Some(current_dir) = current_dir {
        if dir.starts_with("./") {
            return Ok(extend_dir(dir, &current_dir, 1));
        } else if dir.starts_with("../") {
            return Ok(extend_dir(dir, &current_dir, 0));
        }
    }
    return Err(crate::Error::InvalidDirectory {
        reason: "Could not resolve".to_string(),
        directory: dir.as_os_str().to_os_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path_abs_path() {
        assert_eq!(
            resolve_directory_impl(
                &PathBuf::from("/usr/bin"),
                &PathBuf::from("/home/foo"),
                None
            )
            .unwrap(),
            PathBuf::from("/usr/bin")
        );
    }

    #[test]
    fn test_resolve_path_is_home() {
        assert_eq!(
            resolve_directory_impl(&PathBuf::from("~"), &PathBuf::from("/home/foo"), None).unwrap(),
            PathBuf::from("/home/foo")
        );
    }

    #[test]
    fn test_resolve_path_in_home() {
        assert_eq!(
            resolve_directory_impl(
                &PathBuf::from("~/.local/bin"),
                &PathBuf::from("/home/foo"),
                None
            )
            .unwrap(),
            PathBuf::from("/home/foo/.local/bin")
        );
    }

    #[test]
    fn test_resolve_path_is_other_home() {
        assert!(
            resolve_directory_impl(&PathBuf::from("~foo"), &PathBuf::from("/home/foo"), None)
                .is_err()
        );
    }

    #[test]
    fn test_resolve_path_in_other_home() {
        assert!(resolve_directory_impl(
            &PathBuf::from("~foo/.local/bin"),
            &PathBuf::from("/home/foo"),
            None
        )
        .is_err(),);
    }

    #[test]
    fn test_resolve_path_invalid_dot_path() {
        assert!(resolve_directory_impl(
            &PathBuf::from(".local/bin"),
            &PathBuf::from("/home/foo"),
            None
        )
        .is_err());
    }

    #[test]
    fn test_resolve_path_rel_path() {
        assert!(resolve_directory_impl(
            &PathBuf::from("./local/bin"),
            &PathBuf::from("/home/foo"),
            None
        )
        .is_err());
    }

    #[test]
    fn test_resolve_path_rel_path_up() {
        assert!(resolve_directory_impl(
            &PathBuf::from("../local/bin"),
            &PathBuf::from("/home/foo"),
            None
        )
        .is_err());
    }

    #[test]
    fn test_resolve_path_rel_path_with_current() {
        assert_eq!(
            resolve_directory_impl(
                &PathBuf::from("./local/bin"),
                &PathBuf::from("/home/foo"),
                Some(PathBuf::from("/zzz")),
            )
            .unwrap(),
            PathBuf::from("/zzz/local/bin")
        );
    }

    #[test]
    fn test_resolve_path_rel_path_up_with_current() {
        assert_eq!(
            resolve_directory_impl(
                &PathBuf::from("../local/bin"),
                &PathBuf::from("/home/foo"),
                Some(PathBuf::from("/zzz")),
            )
            .unwrap(),
            PathBuf::from("/zzz/../local/bin")
        );
    }

    #[test]
    fn test_path_iterator() {
        let path = OsString::from(":/foo::~/bar:");
        let mut iterator = PathIterator::new(&path);

        assert_eq!(iterator.next().unwrap(), OsString::from(""));
        assert_eq!(iterator.next().unwrap(), OsString::from("/foo"));
        assert_eq!(iterator.next().unwrap(), OsString::from(""));
        assert_eq!(iterator.next().unwrap(), OsString::from("~/bar"));
        assert_eq!(iterator.next().unwrap(), OsString::from(""));
        assert!(iterator.next().is_none());
        assert!(iterator.next().is_none());
    }
}
