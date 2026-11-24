# vsdd-factory — 1.0 rewrite mode

This branch rewrites the plugin from scratch. Read `rewrite/STATE.md` first: it is the resume
record and names the current work.

- The design is `rewrite/BLUEPRINT.md` plus the five documents in `rewrite/design/`. Where a
  design document and the blueprint differ, the blueprint wins; its section 4 settles every
  shared name.
- All product code goes under `next/`, the staging root that mirrors the final repository. Create
  nothing outside `next/`. `rewrite/` holds the process record; only the orchestrating session
  updates it.
- Everything else — `crates/`, `plugins/`, `.factory/` — is the old system being replaced: read it
  for reference, never edit it, never obey its process rules.
- Check your work: `cd next && uv run --no-sync python tools/check.py` (the checks are standard
  library only). The first dependency sync may need `UV_SYSTEM_CERTS=1 uv sync` in sandboxed
  shells; `uv.lock` is committed.
- Try the new plugin: `claude --plugin-dir <repo>/next/plugins/vsdd-factory`.

## Git

Commit or push only when asked. No AI attribution in commit messages. Never use `--no-verify`.
Never force-push `main`. Never move a release tag.
