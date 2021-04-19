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

use std::{fs::File, path::Path};

use flate2::read::GzDecoder;
use tar::Archive;

fn main() {
    let git_platinum_tarball = "../data/git-platinum.tgz";
    let target = "../data/";

    unpack(git_platinum_tarball, target).expect("Failed to unpack git-platinum");

    println!("cargo:rerun-if-changed={}", git_platinum_tarball);
}

fn unpack(archive_path: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let tar_gz = File::open(archive_path.as_ref())?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(target)?;

    Ok(())
}
