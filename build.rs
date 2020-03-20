// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{env, path::Path, process::Command};

fn main() {
    // Path set up for the project directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let curr_dir = Path::new(&manifest_dir);

    // Run `git submodule update --init`
    Command::new("git")
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .current_dir(&curr_dir)
        .status()
        .expect("Failed to execute `git submodule update --init`");

    // Tell the build script that we should re-run this if git-platinum changes.
    println!("cargo:rerun-if-changed=data/git-platinum");
}
