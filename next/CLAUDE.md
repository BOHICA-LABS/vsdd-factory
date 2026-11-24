# vsdd-factory

This file is finalized in the documentation package (D3). Until then:

- The repository root CLAUDE.md holds the rewrite rules that bind right now.
- `rewrite/STATE.md` is the process record; `rewrite/BLUEPRINT.md` and `rewrite/design/`
  are the design.
- Check your work with `uv run --no-sync python tools/check.py` from `next/`.

What is true of this repository already: the plugin is `plugins/vsdd-factory/`; everything
else here builds, tests or checks it. It is not itself a factory project: there is no
pipeline state to update after a change.
