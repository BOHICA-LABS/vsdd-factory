#!/bin/sh
# find-root.sh — project discovery for the vsdd-factory launcher.
# Sourced by py/launch.sh for both the hook and cli entries, so there is one
# implementation of "which factory project is this?".
#
# Sets, in the caller's shell:
#   FACTORY_ROOT     the project root, when a factory project was found
#   FACTORY_LEGACY   1, when only pre-1.0 factory state (.factory/STATE.md) was found
#   neither          when no factory project or legacy state was found
#
# The two variables are assignments only (SC2034): they are read by the script
# that sources this file, never here.
#
# Method (rewrite/design/hooks.md section 2.8), in shell and without starting
# any process: start at CLAUDE_PROJECT_DIR (or the working directory), and walk
# upwards testing for .factory/factory.toml at each directory. A session inside
# a linked worktree is covered by following the .git file to the main checkout.
# The walk stops below the user's home directory and the filesystem root, so a
# stray .factory/ there can never turn every project into a factory project.

# shellcheck disable=SC2034  # read by the sourcing script, not here
find_root() {
  FACTORY_ROOT=
  FACTORY_LEGACY=

  rf_cur="${CLAUDE_PROJECT_DIR:-$PWD}"
  # Absolute starts only: a relative or empty start means "not a factory project".
  case "$rf_cur" in /*) ;; *) return 0 ;; esac
  # Normalize trailing slashes, but keep "/".
  while :; do
    case "$rf_cur" in */) rf_cur="${rf_cur%/}" ;; *) break ;; esac
  done
  [ -n "$rf_cur" ] || rf_cur=/

  # The stop at the home directory compares strings, so compare against a copy
  # with trailing slashes stripped: a $HOME spelled with a trailing slash would
  # otherwise never match and adopt $HOME/.factory as a project root.
  rf_home="${HOME:-}"
  while :; do
    case "$rf_home" in */) rf_home="${rf_home%/}" ;; *) break ;; esac
  done

  while :; do
    # Home and the filesystem root are walked up to, never used as roots.
    case "$rf_cur" in
      "$rf_home"|"/") return 0 ;;
    esac

    if [ -f "$rf_cur/.factory/factory.toml" ]; then
      FACTORY_ROOT="$rf_cur"
      return 0
    fi

    # A linked worktree carries .git as a file: "gitdir: <main>/.git/worktrees/<name>".
    # Since git 2.48 the line may be relative ("gitdir: ../main/.git/worktrees/x",
    # worktree.useRelativePaths), so make it absolute against the worktree before
    # stripping the suffix; a relative path here would export a relative
    # FACTORY_ROOT that breaks as soon as the caller's cwd differs.
    # Also test the main checkout, once, without leaving the shell.
    if [ -f "$rf_cur/.git" ]; then
      rf_gd=
      IFS= read -r rf_gd <"$rf_cur/.git" 2>/dev/null || rf_gd=
      case "$rf_gd" in
        "gitdir: "*".git/worktrees/"*)
          rf_main="${rf_gd#gitdir: }"
          case "$rf_main" in
            /*) ;;
            *) rf_main="$rf_cur/$rf_main" ;;
          esac
          rf_main="${rf_main%%/.git/worktrees/*}"
          if [ -f "$rf_main/.factory/factory.toml" ]; then
            FACTORY_ROOT="$rf_main"
            return 0
          fi
          if [ -f "$rf_main/.factory/STATE.md" ]; then
            FACTORY_LEGACY=1
          fi
          ;;
      esac
    fi

    # Pre-1.0 state: recognised, so the briefing can name the migration step.
    if [ -f "$rf_cur/.factory/STATE.md" ]; then
      FACTORY_LEGACY=1
    fi

    # Move to the parent; "" means we were at "/", and "/" was handled above.
    rf_cur="${rf_cur%/*}"
    [ -n "$rf_cur" ] || rf_cur=/
  done
}

find_root
