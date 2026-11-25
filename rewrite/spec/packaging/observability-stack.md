---
kind: packaging
id: observability-stack
package: T3
sources: [packaging.md#the-observability-stack]
---

# The observability stack

It ports whole, to `<plugin>/observability/`, and stays opt-in.

- `docker-compose.yml` (Loki, the readiness helper, the OpenTelemetry collector, Prometheus, Grafana and its image
  renderer) — Ported. Image versions are re-pinned at build time. `vsdd obs` always passes `-p observability`, the
  project name the old stack got by accident from its folder name, so the user's existing data volumes carry over.
- `otel-collector-config.yaml` — Ported, with its file receiver pointed at the 1.0 event log's location and field
  names ([[topic:event-log]]).
- `loki-config.yaml`, `prometheus-config.yaml`, three Grafana provisioning files — Ported unchanged.
- `claude-cost.json`, `claude-overview.json` — Ported unchanged. They read Claude Code's own telemetry, not factory
  events.
- `factory-overview.json`, `factory-today.json`, `factory-prs.json`, `factory-subagents.json`, `factory-roi.json` —
  Ported, with queries re-pointed at the 1.0 event names.
- `factory-obs` (482 lines of bash) — Ported to Python as `vsdd obs` ([[command:obs]]).
- The registry, `~/.config/vsdd-factory/watched-factories` — Kept, same path and format. `vsdd obs up` inside an
  initialised project registers it automatically, which removes a manual step.
- The generated compose override — Moved from inside the plugin directory, which is replaced on every update, to
  `~/.config/vsdd-factory/obs/`.
- Sending events elsewhere (the old Datadog, Honeycomb, HTTP and gRPC sink crates) — Those crates were linked into
  the dispatcher and never switched on, so no user capability is lost. In 1.0 the hook runner writes one local file
  and never opens a connection. Anyone who wants events elsewhere adds an exporter to the collector, which is what a
  collector is for. `docs/observability.md` shows how ([[packaging:documentation]]).
- Turning on Claude Code's own telemetry for a project — New: `vsdd obs telemetry` writes the five environment
  settings into the project's `.claude/settings.local.json`. The old guide had people do it by hand, and only
  `prism` has it.

Lint L-OBS checks the compose file, the configurations and the dashboards ([[table:lint-checks]]). Delivered by
package T3 with `vsdd events` ([[package:T3]]).
