#!/usr/bin/env bash
set -euo pipefail

# Ensure that we have heads/dev and we're on the master branch
git submodule foreach "git checkout dev"
git submodule foreach "git checkout master"

# Ensure that we have the mock branches set up in the submodule
input="./data/mock-branches.txt"
while IFS= read -r line
do
  IFS=, read -a pair <<< $line
  echo "Creating branch ${pair[0]}"
  git -C data/git-platinum/ update-ref ${pair[0]} ${pair[1]}
done < "$input"
