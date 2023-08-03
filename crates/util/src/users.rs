// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Tobias Hunger <tobias.hunger@gmail.com>

pub fn is_effective_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}
