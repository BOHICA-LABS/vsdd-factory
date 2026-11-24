# vsdd-factory

Verified Spec-Driven Development for Claude Code: specifications, stories, test-first
delivery, adversarial review, holdout evaluation and release gating, run by workflows and
enforced by hooks.

This is the plugin directory as installed. To build, test or contribute to it, use the
repository checkout, which has the design documents, the tests and the build tooling:

https://github.com/bohica-labs/vsdd-factory

Install from the marketplace. In Claude Code, run:

    /plugin marketplace add bohica-labs/claude-mp
    /plugin install vsdd-factory@claude-mp

(The equivalent terminal form is `claude plugin install vsdd-factory@claude-mp`.)

Requirements: Claude Code 2.1.331 or later; Python 3.11 or later, which `uv` locates but
never downloads (if none is installed: `uv python install 3.12`); `uv`, `git` and `gh`. An
internet connection is needed for install; the shipped Python is standard-library only and
never touches the network. Privacy: the plugin ships no telemetry; hook events are written
to the project's own `.factory/local/events/` directory.

After installing, run `/vsdd-factory:setup` inside the project you want the pipeline to work
on. `/vsdd-factory:status` shows where things stand at any time.
