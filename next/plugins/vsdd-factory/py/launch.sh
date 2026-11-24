#!/bin/sh
# launch.sh — the one guarded launcher for vsdd-factory (rewrite/design/hooks.md 2.2).
#
#   sh py/launch.sh hook <gate|brief|notify>    stdin: hook JSON, passed through untouched
#   sh py/launch.sh cli <args...>               the vsdd command
#
# Neither entry execs uv: both must see uv's exit code. The hook entry turns a
# runner block (10) into a Claude Code block (2) and every other failure into
# the right message; the cli entry distinguishes "no Python at the floor" from
# vsdd's own usage exit (2). Facts this relies on were measured and are
# recorded in the design and rewrite/STATE.md.
#
# No entry downloads a Python. When uv cannot find one, the fix is a terminal
# command a person runs themselves: uv python install 3.12.
#
# Test seam: VSDD_FACTORY_SELFTEST_NO_UV=1 makes both entries behave as if uv
# is not installed, whatever the machine has. Only the test suite sets it.

entry="${1:-}"

case "$0" in
  */*) here="${0%/*}" ;;
  *)   here=. ;;
esac

# shellcheck source=SCRIPTDIR/find-root.sh
. "$here/find-root.sh"                      # sets FACTORY_ROOT, or FACTORY_LEGACY=1, or neither

msg_py_floor="uv found no Python 3.11 or newer. In your own terminal run: uv python install 3.12, then restart Claude Code. If the problem persists, check: uv --version, and: uv cache dir."
msg_uv_missing_hook="uv is not installed, so the factory gates cannot run. In your own terminal run: brew install uv (or: curl -LsSf https://astral.sh/uv/install.sh | sh), then restart Claude Code."
msg_uv_missing_cli="uv is not installed, so the vsdd tool cannot run. In your own terminal run: brew install uv (or: curl -LsSf https://astral.sh/uv/install.sh | sh), then restart Claude Code."
msg_launch_failed="In your own terminal check: uv --version, then reinstall the plugin, and restart Claude Code."

append_event() {                            # $1 = type, $2 = severity, $3 = code; best effort, never fails
  ev_dir="${VSDD_EVENTS_DIR:-$FACTORY_ROOT/.factory/local/events}"
  mkdir -p "$ev_dir" 2>/dev/null || return 0
  printf '{"v":1,"ts":"%s","type":"%s","severity":"%s","session_id":null,"code":"%s","mode":"%s"}\n' \
    "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$1" "$2" "$3" "${mode:-}" \
    >>"$ev_dir/$(date -u '+%Y-%m-%d').jsonl" 2>/dev/null || return 0
}

print_down_briefing() {                     # $1 = fixed reason (no double quotes inside)
  m="$1"
  case "$m" in
    *.) m="${m%.}" ;;                       # the briefing appends its own clause; no doubled full stop
  esac
  printf '{"continue":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"vsdd-factory: the factory gates are down: %s; Guarded tool calls are blocked until this is fixed."},"systemMessage":"vsdd-factory: the factory gates are down: %s"}\n' \
    "$m" "$1"
}

print_disabled_briefing() {
  printf '{"continue":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"vsdd-factory hooks are switched off in this session (VSDD_FACTORY_HOOKS=off). The gates will not run and nothing is guarded."},"systemMessage":"vsdd-factory: hooks are switched off (VSDD_FACTORY_HOOKS=off)"}\n'
}

print_migration_line() {
  printf 'vsdd-factory: this project has pre-1.0 factory state. To migrate it, run /vsdd-factory:setup in this session (a person can instead run the import in their own terminal; /vsdd-factory:status shows the state either way).\n'
}

fail() {                                    # $1 = code, $2 = fixed message (no double quotes inside)
  append_event "runner.launch_failed" "error" "$1"
  case "$mode" in
    gate)  printf 'vsdd-factory: %s Guarded tool calls are blocked until this is fixed. [%s]\n' "$2" "$1" >&2; exit 2 ;;
    brief) print_down_briefing "$2"; exit 0 ;;
    *)     exit 0 ;;
  esac
}

interpreter_ok() {                          # true when uv finds a Python >= 3.11 without a download;
                                            # failure path only, so the success path costs nothing.
                                            # The probe gets its own writable cache: when the session's
                                            # UV_CACHE_DIR is broken, the probe must still answer the
                                            # interpreter question, or a broken cache is misreported
                                            # as a missing Python.
  UV_CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/vsdd-factory/probe-cache" \
    "$uv" python find --no-python-downloads '>=3.11' >/dev/null 2>&1
}

find_uv() {                                 # sets uv, or leaves it empty
  uv=$(command -v uv 2>/dev/null)
  if [ -z "$uv" ]; then
    for d in "$HOME/.local/bin" "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin; do
      if [ -x "$d/uv" ]; then uv="$d/uv"; break; fi
    done
  fi
}

clean_env() {
  unset UV_PYTHON PYTHONPATH PYTHONHOME PYTHONSTARTUP
  PYTHONSAFEPATH=1
  PYTHONUTF8=1
  PYTHONPYCACHEPREFIX="${XDG_CACHE_HOME:-$HOME/.cache}/vsdd-factory/pycache"
  export PYTHONSAFEPATH PYTHONUTF8 PYTHONPYCACHEPREFIX
  if [ -n "$FACTORY_ROOT" ]; then
    VSDD_FACTORY_ROOT="$FACTORY_ROOT"
    export VSDD_FACTORY_ROOT
  else
    unset VSDD_FACTORY_ROOT || :            # a stale or forged root must not survive into the child
  fi
}

map_rc() {                                  # $1 = uv's exit code -> fail() with the message for
  rc=$1                                     # that code (hooks.md 2.9). Always exits. The cli entry
  case "$rc" in                             # has its own mapping below, because vsdd's own usage
    2)  # uv exits 2 both when no Python at the floor is installed and when uv
        # itself cannot run (an unwritable cache, an unknown flag on an old
        # uv). Discriminate, so each cause gets its own fix.
        if interpreter_ok; then
          fail uv_unusable "uv could not run (exit 2), although a Python 3.11 or newer is installed. In your own terminal check: uv --version, and: uv cache dir, then upgrade: uv self update (or: brew upgrade uv), and restart Claude Code."
        fi
        fail py_floor "$msg_py_floor" ;;
    13) fail py_floor "$msg_py_floor" ;;    # hook.py's own floor check, defence in depth
    *)  fail "launch_rc_$rc" "the hook runner could not run (exit $rc). $msg_launch_failed" ;;
  esac
}

case "$entry" in
  hook)
    mode="${2:-gate}"
    case "$mode" in
      gate|brief|notify) ;;
      *)  # An unknown mode would make every launch failure below silent: fail closed.
        printf "vsdd-factory: unknown hook mode '%s'. The mode must be gate, brief or notify; check the command in hooks.json.\n" "$mode" >&2
        exit 2
        ;;
    esac

    if [ -z "$FACTORY_ROOT" ]; then
      # Not a 1.0 factory project: completely inert. Legacy state is announced
      # on the session start only.
      if [ "$mode" = brief ] && [ -n "$FACTORY_LEGACY" ]; then
        print_migration_line
      fi
      exit 0
    fi

    # The one emergency switch. Only a person can set it: a tool call cannot
    # change the environment of the Claude Code process.
    if [ "${VSDD_FACTORY_HOOKS:-on}" = off ]; then
      [ "$mode" = brief ] && print_disabled_briefing
      append_event "runner.disabled" "info" "hooks_off"
      exit 0
    fi

    if [ -n "${VSDD_FACTORY_SELFTEST_NO_UV:-}" ]; then
      uv=                                   # test seam: behave as if uv is not installed
    else
      find_uv
    fi
    if [ -z "$uv" ]; then
      fail uv_missing "$msg_uv_missing_hook"
    fi

    clean_env
    "$uv" run --quiet --no-config --offline --no-python-downloads --python '>=3.11' --script "$here/hook.py" "$mode"
    rc=$?
    case "$rc" in
      0)  exit 0 ;;
      10) exit 2 ;;                         # a rule blocked; the launcher is silent: the reason is on stderr
      *)  map_rc "$rc" ;;
    esac
    ;;

  cli)
    shift
    if [ -n "${VSDD_FACTORY_SELFTEST_NO_UV:-}" ]; then
      uv=                                   # test seam: behave as if uv is not installed
    else
      find_uv
    fi
    if [ -z "$uv" ]; then
      printf 'vsdd-factory: %s\n' "$msg_uv_missing_cli" >&2
      exit 2
    fi
    clean_env
    "$uv" run --quiet --no-config --offline --no-python-downloads --python '>=3.11' --script "$here/cli.py" "$@"
    rc=$?
    case "$rc" in
      0)  exit 0 ;;
      2)  # vsdd itself exits 2 for a usage error, so blame uv only when the
          # interpreter check confirms it; otherwise pass the code through.
        interpreter_ok || printf 'vsdd-factory: %s\n' "$msg_py_floor" >&2
        exit 2 ;;
      *)  printf 'vsdd-factory: the vsdd tool could not run (exit %s). %s\n' "$rc" "$msg_launch_failed" >&2
        exit "$rc" ;;
    esac
    ;;

  *)
    printf 'usage: launch.sh hook <gate|brief|notify> | launch.sh cli <args...>\n' >&2
    exit 2
    ;;
esac
