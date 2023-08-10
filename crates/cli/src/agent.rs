// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// cSpell: ignore logname

use std::{ffi::OsStr, os::unix::prelude::OsStrExt};

fn blacklisted_var(name: &OsStr) -> bool {
    let k = name.as_bytes();
    k == b"PATH"
        || (k.len() >= 9 && k[0..9] == b"container"[..])
        || k == b"HOME"
        || k == b"USER"
        || k == b"LOGNAME"
        || k == b"NOTIFY_SOCKET"
        || k == b"LANG"
}

pub fn run(command_prefix: &str) -> anyhow::Result<()> {
    let original_env = std::env::vars_os()
        .filter(|(k, _)| !blacklisted_var(k.as_os_str()))
        .collect::<std::collections::HashMap<_, _>>();

    // Update environment here...
    std::env::set_var("FOOBAR_zzz", "baz");
    std::env::set_var("FOOBAR", "baz");
    std::env::set_var("FOOBAR", "bar");
    std::env::set_var("TIMESTAMP", "42");

    std::env::vars_os()
        .filter(|(k, v)| !blacklisted_var(k.as_os_str()) && original_env.get(k) != Some(v))
        .for_each(|(k, v)| {
            println!("{command_prefix}: SET {k:?}={v:?}");
        });

    println!("{command_prefix}: BAR something");
    println!("{command_prefix}: SET something");

    Ok(())
}
