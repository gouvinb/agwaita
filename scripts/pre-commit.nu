#!/usr/bin/env nu
#-*- nushell-ts -*-


def prepare_hook [stash_name: string] {
    print "--- Prepare Pre-commit hook ---"

    git stash push --keep-index --include-untracked --message $stash_name out> /dev/null
}

def run_hook [] {
    print "--- Running Pre-commit hook ---"

    ./make.nu check
    ./make.nu build

    print "--- End Pre-commit hook ---"
}

def end_hook [stash_name: string] {
    if (git stash list | lines | any { |it| $it =~ $stash_name }) {
        git stash pop --quiet
    }
}


def main [] {
    let stash_name = $"pre-commit-(date now | format date %s)"

    prepare_hook $stash_name

    try {
        run_hook
    } finally {
        end_hook $stash_name
    }
}