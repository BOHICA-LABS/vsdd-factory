---
kind: command
id: repos-scan
package: W12
sources: [BLUEPRINT.md#4.9, packaging.md#2.10]
---

# `vsdd repos scan`

Operator tool. `vsdd repos scan [--list] [--count]` finds the repositories of a multi-repository project and prints
their dependency graph as JSON, reading `Cargo.toml`, `package.json`, `pyproject.toml` and `go.mod`.

## Who may call it

Run by the main session.

Built with the `multi` workflow in package W12 ([[package:W12]], [[workflow:multi]]), which
serves only multi-repository projects.
