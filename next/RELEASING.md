# Releasing vsdd-factory

This file is finalized in the documentation package (D3), from the packaging design
(`rewrite/design/packaging.md`, section 2.8). Outline, to be replaced by the tested procedure:
a release pull request edits the version in `plugins/vsdd-factory/.claude-plugin/plugin.json`
(the only place it is written by hand), CI must be green, and the marketplace repository is
pinned to the resulting tag.
