#!/usr/bin/env bash
set -euo pipefail

input="./data/mock-branches.txt"
while IFS= read -r line
do
  IFS=, read -a pair <<< $line
  echo "Removing branch ${pair[0]}"
  git submodule foreach "git update-ref -d ${pair[0]}"
done < "$input"
