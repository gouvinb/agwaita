#!/usr/bin/env sh

set -e

echo "--- Prepare Pre-commit hook ---"

STASH_NAME="pre-commit-$(date +%s)"
git stash push --keep-index --include-untracked --message "$STASH_NAME" > /dev/null

cleanup() {
    if git stash list | grep -q "$STASH_NAME"; then
        git stash pop --quiet
    fi
}
trap cleanup EXIT

echo "--- Running Pre-commit hook ---"

./make.nu check
./make.nu build

echo "--- End Pre-commit hook ---"