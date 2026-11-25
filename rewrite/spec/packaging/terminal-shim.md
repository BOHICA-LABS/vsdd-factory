---
kind: packaging
id: terminal-shim
package: T1
sources: [packaging.md#2.10, BLUEPRINT.md#4.1]
---

# The terminal shim

`<plugin>/bin/` is on `PATH` only for the Bash tool, and the plugin's cache path changes with every version.
`vsdd doctor --install-shim` ([[command:doctor]]), which `setup` offers with the person's confirmation, writes
`~/.local/bin/vsdd`: a few lines of `sh` that find the plugin directory to use and run its `bin/vsdd`.

It does not use a recorded path, which changes with every update, or the newest directory in the plugin cache, which
may be another project's version: projects can pin different tags, and during the cutover the rc.25 directories have
no `bin/vsdd` at all. It reads `~/.claude/plugins/installed_plugins.json` and takes the `vsdd-factory` entry whose
project path is the current directory or an ancestor of it, and failing that the user-scope entry. It then checks
that the install path has `bin/vsdd`; if it does not, it prints the command that reinstalls or updates the plugin for
this project and exits 2.

So the person-only commands (`vsdd decide`, `vsdd import`, `vsdd init --fresh`, `vsdd config set`,
`vsdd lock release --force`) always run the code of the version this project uses. With it, `vsdd decide`,
`vsdd lock release`, `vsdd config set`, `vsdd explain` and `vsdd doctor` work in a terminal, which is where the
designs send a person when a session cannot help.

Every message to a person names the in-session form first and the terminal form second
([[doc:conventions]]), because the in-session form needs no shim.

Built by package T1 with the rest of `vsdd doctor` ([[package:T1]]).
