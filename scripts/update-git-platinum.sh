#!/usr/bin/env bash
set -euo pipefail

## This is an update script for /data/git-platinum. It's meant to be executed
## from the root of the repo.

TARBALL_PATH=data/git-platinum.tgz

# This is here in case the last script run failed and it never cleaned up.
# Is there a better way to handle this?
rm -rf git-platinum

# Clone an up-to-date version of git-platinum.
git clone https://github.com/radicle-dev/git-platinum.git
git -C git-platinum/ checkout dev

# Add the necessary refs.
input="./data/mock-branches.txt"
while IFS= read -r line
do
    IFS=, read -a pair <<< $line
    echo "Creating branch ${pair[0]}"
    git -C git-platinum/ update-ref ${pair[0]} ${pair[1]}
done < "$input"

# Update the archive.
tar -czf git-platinum.tgz git-platinum
mv git-platinum.tgz $TARBALL_PATH

# Clean up.
rm -rf git-platinum

# Commit.
git reset
git add $TARBALL_PATH
git commit -m "Update git-platinum"
