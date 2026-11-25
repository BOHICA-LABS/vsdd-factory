---
kind: skill
id: observe
package: K3b
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, packaging.md#the-observability-stack]
---

# observe

Starts, stops and checks the local observability stack, registers a project with it, and sets the telemetry
environment.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "<up or down or status or register or telemetry>"
```

## What it does

Instructs the main session to run the packaging design's observability command (`vsdd obs`,
[[command:obs]]). The stack is ported whole to `<plugin>/observability/` and stays opt-in: the compose file
(Loki, the readiness helper, the OpenTelemetry collector, Prometheus, Grafana and its image renderer), the
collector configuration pointed at the 1.0 event log, the Grafana provisioning files and dashboards, and the
registry at `~/.config/vsdd-factory/watched-factories`, which `vsdd obs up` inside an initialised project
joins automatically. The generated compose override lives in `~/.config/vsdd-factory/obs/`, not inside the
plugin directory, which is replaced on every update. `vsdd obs telemetry` writes the five environment settings
that turn on Claude Code's own telemetry into the project's `.claude/settings.local.json`. In 1.0 the hook
runner writes one local file and never opens a connection; anyone who wants events elsewhere adds an exporter
to the collector.
