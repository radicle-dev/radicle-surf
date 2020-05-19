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

use std::{
    env,
    path::Path,
    process::{Command, ExitStatus},
};

fn checkout(curr_dir: &Path, branch: &str) -> ExitStatus {
    let mut git_checkout = "git checkout ".to_string();
    git_checkout.push_str(branch);

    Command::new("git")
        .arg("submodule")
        .arg("foreach")
        .arg(&git_checkout)
        .current_dir(&curr_dir)
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute `git submodule update {}`", git_checkout))
}

fn main() {
    // Path set up for the project directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let curr_dir = Path::new(&manifest_dir);

    // Run `git submodule update --init`
    let init = Command::new("git")
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .current_dir(&curr_dir)
        .status()
        .expect("Failed to execute `git submodule update --init`");
    assert!(init.success(), "init of submodule failed");

    let dev_status = checkout(curr_dir, "dev");
    assert!(dev_status.success(), "failed to checkout dev");

    let master_status = checkout(curr_dir, "master");
    assert!(master_status.success(), "failed to checkout master");

    // Tell the build script that we should re-run this if git-platinum changes.
    println!("cargo:rerun-if-changed=data/git-platinum");
}
