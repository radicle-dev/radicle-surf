#!/usr/bin/env bash
set -euo pipefail

# Verify that the script is run from project root.
BASE=$(basename $(pwd))

if [ "${BASE}" != "radicle-surf" ]
then
   echo "ERROR: this script should be run from the root of radicle-surf"
   exit 1
fi

TARBALL_PATH=data/git-platinum.tgz
WORKDIR=.workdir

# Create the workdir if needed.
mkdir -p $WORKDIR

# This is here in case the last script run failed and it never cleaned up.
rm -rf $WORKDIR/git-platinum

# Clone an up-to-date version of git-platinum.
git clone https://github.com/radicle-dev/git-platinum.git $WORKDIR/git-platinum
git -C $WORKDIR/git-platinum/ checkout dev

# Add the necessary refs.
input="./data/mock-branches.txt"
while IFS= read -r line
do
    IFS=, read -a pair <<< $line
    echo "Creating branch ${pair[0]}"
    git -C $WORKDIR/git-platinum/ update-ref ${pair[0]} ${pair[1]}
done < "$input"

# Update the archive.
tar -czf $WORKDIR/git-platinum.tgz $WORKDIR/git-platinum
mv $WORKDIR/git-platinum.tgz $TARBALL_PATH

# Clean up.
rm -rf $WORKDIR/git-platinum
