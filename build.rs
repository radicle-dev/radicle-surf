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
    let git_checkout = format!("git checkout -f {}", branch);

    Command::new("git")
        .arg("submodule")
        .arg("foreach")
        .arg(&git_checkout)
        .current_dir(&curr_dir)
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute `git submodule update {}`", git_checkout))
}

fn update_ref(curr_dir: &Path, namespace: &str, target: &str) -> ExitStatus {
    let namespace = format!("git update-ref {} {}", namespace, target);
    Command::new("git")
        .arg("submodule")
        .arg("foreach")
        .arg(&namespace)
        .current_dir(&curr_dir)
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute `git submodule update {}`", namespace))
}

fn setup_fixtures() {
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

    let pinned = checkout(curr_dir, "223aaf87d6ea62eef0014857640fd7c8dd0f80b5");
    assert!(pinned.success(), "failed to pin to commit");

    for (new_rev, rev) in [
        (
            "refs/namespaces/golden/refs/heads/master",
            "refs/heads/master",
        ),
        ("refs/namespaces/golden/refs/heads/banana", "refs/heads/dev"),
        (
            "refs/namespaces/golden/refs/tags/v0.1.0",
            "refs/tags/v0.1.0",
        ),
        (
            "refs/namespaces/golden/refs/tags/v0.2.0",
            "refs/tags/v0.2.0",
        ),
        (
            "refs/namespaces/golden/refs/remotes/kickflip/heads/heelflip",
            "refs/heads/dev",
        ),
        (
            "refs/namespaces/golden/refs/namespaces/silver/refs/heads/master",
            "refs/heads/dev",
        ),
        (
            "refs/remotes/banana/pineapple",
            "refs/remotes/origin/master",
        ),
    ]
    .iter()
    {
        let update_rev = update_ref(curr_dir, new_rev, rev);
        assert!(
            update_rev.success(),
            "failed to set up '{} -> {}'",
            new_rev,
            rev
        );
    }
}

fn main() {
    if env::var("GIT_FIXTURES").is_ok() {
        setup_fixtures();
    }

    for entry in glob::glob("./.git/modules/data/git-platinum/refs/**/*")
        .expect("Failed to read glob pattern")
    {
        match entry {
            Ok(path) => println!("{:?}", path.display()),
            Err(e) => println!("{:?}", e),
        }
    }

    // Tell the build script that we should re-run if this script changes.
    println!("cargo:rerun-if-env-changed=GIT_FIXTURES");
}
