// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

pub fn run(greeting: &str) -> anyhow::Result<()> {
    println!("Hello {greeting}.");

    Ok(())
}
