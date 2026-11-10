#!/usr/bin/env nu
#-*- nushell-ts -*-

const TAG_RE = '^v?\d+\.\d+\.\d+(-[A-Za-z0-9.]+)?$'

def cargo_version [commit: string] {
    let pkg = (git show $"($commit):agw-cli/Cargo.toml" | from toml | get package)

    if ($pkg.version | describe | str starts-with "record") {
        git show $"($commit):Cargo.toml" | from toml | get workspace.package.version
    } else {
        $pkg.version
    }
}

def check_tag [tag: string, sha: string] {
    if not ($tag =~ $TAG_RE) {
        return $"tag '($tag)' doesn't match <Major>.<Minor>.<Patch>-<state>"
    }

    let commit = (git rev-parse $"($sha)^{commit}" | str trim)
    let cargo = (do -i { cargo_version $commit })

    if $cargo == null {
        return $"Cannot read agw-cli/Cargo.toml version at commit ($commit)"
    }

    let tag_version = ($tag | str replace -r '^v' '')
    if $cargo != $tag_version {
        return $"tag '($tag)' != version agw-cli/Cargo.toml '($cargo)'"
    }

    ""
}

def main [remote: string = "", url: string = ""] {
    let refs = (^cat | lines | where { |it| $it != "" })
    mut errors = []

    for line in $refs {
        let parts = ($line | split row " ")
        let local_ref = $parts.0
        let local_sha = $parts.1

        if not ($local_ref | str starts-with "refs/tags/") { continue }
        if ($local_sha =~ '^0+$') { continue }

        let tag = ($local_ref | str replace "refs/tags/" "")
        let err = (check_tag $tag $local_sha)

        if $err != "" {
            $errors = ($errors | append $err)
        }
    }

    if ($errors | is-not-empty) {
        print "--- Pre-push hook: tag version check ---"
        $errors | each { |e| print $"  - ($e)" } | ignore
        print "Fix agw-cli/Cargo.toml or the git tag, then push it."
        exit 1
    }
}
