---
kind: topic
id: workflow-arguments
sources: [pipeline.md#2.3, BLUEPRINT.md#4.5]
---

# Workflow arguments

How a workflow takes its arguments as an object or as typed text, and how free text reaches the tool
percent-encoded. Every workflow's own argument names are in its file; the twelve sets are summed up in
the registry and in [[workflow:story]] and the others.

A workflow accepts its arguments as an object or as the text a person typed after the command. A
person types text after the command, and whether the platform hands a script that text as a string or
as an object is not yet proven. Every script therefore passes `args` through one prelude function,
`parseArgs()` ([[topic:prelude]]), which accepts an object unchanged and parses a string by a small
grammar: a `key=value` word sets a named argument only when `key` is one of that workflow's own
argument names, and every other word is part of the main argument. A request such as "make
timeout=30 configurable" therefore keeps `timeout=30`. So
`/vsdd-factory:feature CSV export drops the last row intent=bug-fix` sets `request` and `intent`;
`/vsdd-factory:spike what is the cold start of uv` sets `question`;
`/vsdd-factory:spike open uv-startup -- measure cold and warm start` opens a named spike; and
`/vsdd-factory:spike close uv-startup` closes it. The harness runs every workflow with its documented
argument text.

Free text never goes into a command line raw. A script cannot write a file and a clerk's command takes
no standard input, so the prelude function `enc()` percent-encodes the text and the tool decodes it.
`encodeURIComponent` alone is not enough: it leaves `!`, `'`, `(`, `)` and `*` raw, and each of those
breaks or globs a shell command, so `enc()` encodes those five as well. The options are `vsdd survey feature --request-uri <encoded>`, `--guidance-uri`, `--question-uri`.
A lint test checks that every free-text value in a `relay()` command went through `enc()`. `safe()` is
for identifiers only.

Argument names in `parseArgs()` are conformance fixtures. Every workflow also accepts `repo`, the name
of a repository in a multi-repository project ([[topic:multi-repo]]).
