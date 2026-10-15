#!/usr/bin/env python3
"""S-25.08 demo driver (evidence-only; lives under docs/demo-evidence/S-25.08/).

Drives the REAL `factory-dispatcher` binary built from this worktree against
FIXTURE projects created in fresh temp dirs. It never touches the repo's own
.factory/. Each subcommand prints a short, readable transcript:

    verdicts       AC-001/002/013/014/028       E-MAINTENANCE-001 blocks + admitted paths
    causes         AC-018/028/030               E-MAINTENANCE-002 <cause> tokens
    lifecycle      AC-003/004/006/011                      reserve on PreToolUse, release on completion/block
    release-order  AC-011/031                              release-leg evaluation order (process boundary)
    b2             AC-007/009/015/020                      B2 (BC-INDEX) coordinator conformance
    registry       AC-027                                  registry error -> exit code mapping

Usage: python3 demo-driver.py <subcommand>      (env FD=<path to factory-dispatcher>)
"""
import fcntl
import json
import os
import re
import subprocess
import sys
import tempfile

ROOT = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
FD = os.environ.get("FD") or os.path.join(ROOT, "target/debug/factory-dispatcher")
CYCLES = ".factory/cycles/c1/burst-log.md"
BCPATH = ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md"


class Proj:
    def __init__(self, registry="schema_version = 2\n", ms=True):
        self.tmp = tempfile.TemporaryDirectory(prefix="s2508-demo-")
        self.plug = tempfile.TemporaryDirectory(prefix="s2508-plug-")
        self.logs = tempfile.TemporaryDirectory(prefix="s2508-logs-")
        self.root = self.tmp.name
        if registry is not None:
            with open(os.path.join(self.plug.name, "hooks-registry.toml"), "w") as f:
                f.write(registry)
        if ms:
            os.makedirs(self.ms + "/reservations")
            open(self.ms + "/exclusive.lock", "w").close()
            self.gate("OPEN")
        self._lockfd = None

    @property
    def ms(self):
        return self.root + "/.factory/migration-state"

    def gate(self, state):
        with open(self.ms + "/gate-state.json", "w") as f:
            f.write('"%s"' % state)

    def txn(self, state, generation_id="gen-1", migration_id=None):
        v = {"txn_id": "txn-demo", "activation_id": "act-demo", "fencing_generation": 1,
             "state": state, "generation_id": generation_id, "source_sha256": None,
             "source_body_row_sha256": None, "intent_log_path": None,
             "pending_canonical_moves": [], "created_at": "2026-10-06T00:00:00Z",
             "updated_at": "2026-10-06T00:00:00Z"}
        if migration_id:
            v["migration_id"] = migration_id
        with open(self.ms + "/txn-act-demo.json", "w") as f:
            json.dump(v, f)

    def hold_lock(self):
        self._lockfd = open(self.ms + "/exclusive.lock", "w")
        fcntl.flock(self._lockfd, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def target(self, rel):
        p = os.path.join(self.root, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        return p

    def reservations(self):
        d = self.ms + "/reservations"
        return sorted(os.listdir(d)) if os.path.isdir(d) else []


def envelope(event, tool, tuid, path=None, tool_input=None):
    v = {"hook_event_name": event, "tool_name": tool, "session_id": "sess-demo",
         "tool_input": tool_input if tool_input is not None else
         {"file_path": path, "old_string": "a", "new_string": "b"}}
    if tuid is not None:
        v["tool_use_id"] = tuid
    return json.dumps(v)


def run(p, payload, plug=True):
    env = dict(os.environ)
    env["CLAUDE_PROJECT_DIR"] = p.root
    env["VSDD_LOG_DIR"] = p.logs.name
    env.pop("VSDD_TEST_ADMISSION_SEAM_DIR", None)
    if plug:
        env["CLAUDE_PLUGIN_ROOT"] = p.plug.name
    r = subprocess.run([FD], input=payload, capture_output=True, text=True, env=env, timeout=30)
    return r.returncode, r.stderr.strip()


def show(label, code, err, extra=""):
    print("  %-50s exit=%s" % (label, code))
    for line in err.splitlines():
        m = re.search(r'block_reason="([^"]*)"', line)
        if m:
            print("      " + m.group(1))
        elif line.startswith("{"):
            try:
                print("      " + json.loads(line)["message"].splitlines()[0][:120])
            except Exception:
                pass
        elif line.startswith("factory-dispatcher: E-"):
            print("      " + line[:120])
    if extra:
        print("      " + extra)


def verdicts():
    print("== Protected-path admission verdicts (real dispatcher, temp fixtures) ==")
    p = Proj(); p.gate("LOCKED"); p.txn("STAGING", migration_id="backfill-append-logs"); p.hold_lock()
    c, e = run(p, envelope("PreToolUse", "Edit", "T1", p.target(CYCLES)))
    show("cycles path, live STAGING txn (lock held)", c, e, "reservations left: %s" % p.reservations())
    p = Proj(); p.gate("LOCKED"); p.txn("COMMITTING", migration_id="migrate-bc-index"); p.hold_lock()
    c, e = run(p, envelope("PreToolUse", "Write", "T2", p.target(BCPATH)))
    show("BC-INDEX path, live COMMITTING txn", c, e)
    p = Proj(); p.gate("LOCKED"); p.txn("STAGING", migration_id="backfill-append-logs"); p.hold_lock()
    c, e = run(p, envelope("PreToolUse", "Edit", "T3", p.target(".factory/STATE.md")))
    show("OUT-of-scope .factory/STATE.md, live txn", c, e, "-> admitted (not a protected path)")
    p = Proj()
    c, e = run(p, envelope("PreToolUse", "Edit", "T4", p.target(CYCLES)))
    show("cycles path, gate OPEN, no txn", c, e, "-> admitted")


def causes():
    print("== E-MAINTENANCE-002 <cause> tokens: fail closed, exit 2 ==")
    p = Proj()
    c, e = run(p, envelope("PreToolUse", "Edit", "../x", p.target(CYCLES)))
    show("in-scope + tool_use_id '../x'", c, e)
    p = Proj(); open(p.ms + "/gate-state.json", "w").write('"BOGUS"')
    c, e = run(p, envelope("PreToolUse", "Edit", "T5", p.target(CYCLES)))
    show("gate-state.json = \"BOGUS\"", c, e)
    p = Proj(ms=False); os.symlink(".factory", p.root + "/.factory")
    c, e = run(p, envelope("PreToolUse", "Edit", "T6", p.root + "/.factory/cycles/c1/b.md"))
    show("unstatable .factory (self-loop symlink, ELOOP)", c, e)
    p = Proj(ms=False); os.symlink(".factory", p.root + "/.factory")
    c, e = run(p, envelope("PreToolUse", "Edit", "../x", p.root + "/.factory/cycles/c1/b.md"))
    show("ELOOP .factory + bad id (stat precedes id check)", c, e, "-> cause is io, NOT invalid_tool_use_id (AC-030 g)")
    p = Proj(ms=False); os.makedirs(p.root + "/.factory")
    c, e = run(p, envelope("PreToolUse", "Edit", "../x", p.root + "/docs/readme.md"))
    show("out-of-scope target + bad id", c, e, "-> admitted: id never examined (AC-030 h)")
    p = Proj(ms=False); open(p.root + "/.factory", "w").close()
    c, e = run(p, envelope("PreToolUse", "Edit", "T7", p.root + "/.factory/cycles/c1/b.md"))
    show(".factory is a regular file (Absent set)", c, e, "-> admitted, out of scope (AC-028)")


def lifecycle():
    print("== Reserve on PreToolUse / release on completion (real dispatcher) ==")
    p = Proj(); t = p.target(CYCLES)
    c, e = run(p, envelope("PreToolUse", "Edit", "TU1", t))
    show("PreToolUse Edit (admitted)", c, e, "reservations: %s" % p.reservations())
    c, e = run(p, envelope("PostToolUse", "Edit", "TU1", t))
    show("PostToolUse (completion)", c, e, "reservations: %s" % p.reservations())
    c, e = run(p, envelope("PreToolUse", "Write", "TU2", t))
    show("PreToolUse Write (admitted)", c, e, "reservations: %s" % p.reservations())
    c, e = run(p, envelope("PostToolUseFailure", "Write", "TU2", t))
    show("PostToolUseFailure (tool failed)", c, e, "reservations: %s" % p.reservations())
    print("-- Bash is left unprocessed (AC-004) --")
    c, e = run(p, envelope("PreToolUse", "Bash", "TU3", tool_input={"command": "echo hi > %s" % t}))
    show("PreToolUse Bash", c, e, "reservations: %s" % p.reservations())


def release_order():
    print("== Release-leg evaluation order (process boundary: exit + filesystem only) ==")
    print("   migration-state is a self-looping symlink => any stat of it is ELOOP.")
    for label, tuid in [("tool_use_id ABSENT", None), ("tool_use_id '../x'", "../x"),
                        ("tool_use_id '' (empty)", ""), ("tool_use_id VALID", "toolu_ok")]:
        p = Proj(ms=False); os.makedirs(p.root + "/.factory")
        os.symlink("migration-state", p.root + "/.factory/migration-state")
        c, e = run(p, envelope("PostToolUse", "Edit", tuid, p.root + "/" + CYCLES))
        show("PostToolUse, ELOOP migration-state, " + label, c, e)
    print("   Process boundary is identical for all four (exit 0, no verdict, nothing deleted).")
    print("   The warn-vs-silent split is a tracing event (discarded by the production")
    print("   dispatcher), so it is shown by the captured-subscriber tests in the tape that follows.")


def b2():
    print("== B2 (BC-INDEX / migrate-bc-index) coordinator conformance ==")
    p = Proj(); p.gate("LOCKED"); p.txn("COMMITTING", migration_id="migrate-bc-index")
    c, e = run(p, envelope("PreToolUse", "Edit", "B1", p.target(BCPATH)))
    show("COMMITTING, lock free, no terminal record", c, e)
    p = Proj(); p.gate("DRAINING")
    c, e = run(p, envelope("PreToolUse", "Edit", "B2", p.target(BCPATH)))
    gate = open(p.ms + "/gate-state.json").read()
    show("Branch A: DRAINING, no txn, lock free", c, e, "gate now: %s (self-healed)" % gate)
    p = Proj(); p.gate("DRAINING"); p.txn("STAGING", generation_id=None, migration_id="migrate-bc-index")
    c, e = run(p, envelope("PreToolUse", "Edit", "B3", p.target(BCPATH)))
    t = json.load(open(p.ms + "/txn-act-demo.json"))
    show("Branch B: STAGING, generation_id null, lock free", c, e,
         "txn state=%s abort_reason=%s gate=%s" % (t["state"], t.get("abort_reason"), open(p.ms + "/gate-state.json").read()))
    p = Proj(); p.gate("LOCKED"); p.txn("STAGING", migration_id="mystery-migration"); p.hold_lock()
    c, e = run(p, envelope("PreToolUse", "Edit", "B4", p.target(BCPATH)))
    show("Foreign migration_id txn, lock held (AC-015)", c, e)


BAD = {
    "missing registry (E-REG NotFound)": None,
    "malformed TOML (Toml)": "this is = = not toml [[[\n",
    "schema_version = 1 (mismatch)": "schema_version = 1\n",
    "schema_version = 99 (mismatch)": "schema_version = 99\n",
}


def registry():
    print("== Registry error -> exit code (BC-7.06.001 Inv 7 / BC-1.08.001 Inv 1,2) ==")
    for label, body in BAD.items():
        p = Proj(registry=body, ms=False); os.makedirs(p.root + "/.factory")
        c, e = run(p, envelope("PreToolUse", "Edit", "R1", p.root + "/docs/x.md"))
        show(label, c, e)
    p = Proj(registry="schema_version = 2\n", ms=False); os.makedirs(p.root + "/.factory")
    c, e = run(p, envelope("PreToolUse", "Edit", "R1", p.root + "/docs/x.md"))
    show("valid empty registry (control)", c, e)


if __name__ == "__main__":
    {"verdicts": verdicts, "causes": causes, "lifecycle": lifecycle, "release-order": release_order,
     "b2": b2, "registry": registry}[sys.argv[1]]()
