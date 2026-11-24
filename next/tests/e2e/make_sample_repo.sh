#!/bin/sh
# Create a fresh git repository from the sample project in a temporary
# directory, so each end-to-end test can start from a clean copy.
#
# Usage: make_sample_repo.sh [-n] [DEST]
#   -n, --no-factory  leave .factory/ out of the copy (uninitialised project)
#   DEST              target directory; default a new mktemp directory
#
# Prints the repository path, then `commits=N` and `branch=B`, on success.
# The repository has exactly one commit on the branch the sample's
# factory.toml names as the integration branch (main).

set -eu

usage() {
    sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//' >&2
}

no_factory=0
dest=""
while [ $# -gt 0 ]; do
    case $1 in
        -n | --no-factory) no_factory=1 ;;
        -h | --help) usage; exit 0 ;;
        -*) usage; exit 2 ;;
        *) if [ -n "$dest" ]; then usage; exit 2; fi; dest=$1 ;;
    esac
    shift
done

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sample="$here/sample-project"
if [ ! -f "$sample/.factory/factory.toml" ]; then
    echo "make_sample_repo.sh: sample project not found at $sample" >&2
    exit 2
fi

if [ -z "$dest" ]; then
    dest=$(mktemp -d "${TMPDIR:-/tmp}/tally-sample.XXXXXX")
elif [ -e "$dest" ] && [ -n "$(ls -A "$dest" 2>/dev/null)" ]; then
    echo "make_sample_repo.sh: destination is not empty: $dest" >&2
    exit 2
else
    mkdir -p "$dest"
fi

# Copy everything; .gitignore keeps build output and .factory out of the
# product commit, mirroring what a real checkout tracks.
tar -C "$sample" -cf - \
    --exclude='.git' \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --exclude='build' \
    --exclude='.pytest_cache' \
    --exclude='.DS_Store' \
    . | tar -C "$dest" -xf -

if [ "$no_factory" -eq 1 ] && [ -d "$dest/.factory" ]; then
    rm -rf "$dest/.factory"
fi

git -C "$dest" init -q -b main
git -C "$dest" config user.name "vsdd-factory e2e"
git -C "$dest" config user.email "e2e@example.invalid"
git -C "$dest" config commit.gpgsign false
git -C "$dest" add -A
git -C "$dest" commit -q -m "Initial tally sample"

commits=$(git -C "$dest" rev-list --count HEAD)
if [ "$commits" != "1" ]; then
    echo "make_sample_repo.sh: expected 1 commit, got $commits" >&2
    exit 1
fi

printf '%s\n' "$dest"
printf 'commits=%s\n' "$commits"
printf 'branch=%s\n' "$(git -C "$dest" rev-parse --abbrev-ref HEAD)"
