#!/usr/bin/env sh
# Pre-push hook: check that pushed tags match the agw-cli version.
# Git passes: <local ref> <local sha> <remote ref> <remote sha> on stdin.

set -eu

TAG_RE='^v?[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$'
ZERO_SHA='0000000000000000000000000000000000000000'
CRATE_MANIFEST='agw-cli/Cargo.toml'

# Print the version declared for agw-cli at a given commit.
# Handles `version.workspace = true` by reading [workspace.package] from the root manifest.
cargo_version() {
    commit=$1
    manifest=$(git show "$commit:$CRATE_MANIFEST" 2>/dev/null) || return 1

    if printf '%s\n' "$manifest" | grep -Eq '^[[:space:]]*version\.workspace[[:space:]]*=[[:space:]]*true'; then
        root=$(git show "$commit:Cargo.toml" 2>/dev/null) || return 1
        version=$(printf '%s\n' "$root" | awk '
            /^\[workspace\.package\]/ { in_section = 1; next }
            /^\[/ { in_section = 0 }
            in_section && /^[[:space:]]*version[[:space:]]*=/ {
                gsub(/.*=[[:space:]]*"|"[[:space:]]*$/, ""); print; exit
            }')
    else
        version=$(printf '%s\n' "$manifest" | awk '
            /^\[package\]/ { in_section = 1; next }
            /^\[/ { in_section = 0 }
            in_section && /^[[:space:]]*version[[:space:]]*=/ {
                gsub(/.*=[[:space:]]*"|"[[:space:]]*$/, ""); print; exit
            }')
    fi

    [ -n "$version" ] || return 1
    printf '%s\n' "$version"
}

# Print an error message for a tag, or nothing if it is valid.
check_tag() {
    tag=$1
    sha=$2

    if ! printf '%s\n' "$tag" | grep -Eq "$TAG_RE"; then
        echo "tag '$tag' doesn't match <Major>.<Minor>.<Patch>-<state>"
        return 0
    fi

    commit=$(git rev-parse "$sha^{commit}") || {
        echo "cannot resolve commit for tag '$tag'"
        return 0
    }

    if ! cargo=$(cargo_version "$commit"); then
        echo "cannot read $CRATE_MANIFEST version at commit $commit"
        return 0
    fi

    tag_version="${tag#v}"
    if [ "$cargo" != "$tag_version" ]; then
        echo "tag '$tag' != $CRATE_MANIFEST version '$cargo'"
    fi
}

errors=""

while read -r local_ref local_sha remote_ref remote_sha; do
    [ -n "${local_ref:-}" ] || continue
    case "$local_ref" in
        refs/tags/*) ;;
        *) continue ;;
    esac
    # Deleting a remote tag: local sha is all zeros.
    [ "$local_sha" = "$ZERO_SHA" ] && continue

    tag=${local_ref#refs/tags/}
    err=$(check_tag "$tag" "$local_sha")

    if [ -n "$err" ]; then
        errors="${errors}  - ${err}
"
    fi
done

if [ -n "$errors" ]; then
    echo "--- Pre-push hook: tag version check ---"
    printf '%s' "$errors"
    echo "Fix $CRATE_MANIFEST or the git tag, then push it."
    exit 1
fi
