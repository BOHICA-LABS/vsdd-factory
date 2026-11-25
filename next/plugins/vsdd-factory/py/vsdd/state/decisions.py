"""The decision store (topics/decisions.md, records/decision.md,
rules/decisions.md; plan records.md section 3).

The grammar: ``<id> <choice> [key=value ...] [-- note]`` — every value named,
a key may repeat, free text after ``--``; no positional values, no quoted
strings. The nine kinds and their choices are :data:`KINDS`, the shipped form
of spec ``tables/decision-kinds.toml``. Settling writes the decision record
and nothing else: it runs no git and promotes no file, which is what keeps it
inside the hook's four-second deadline. ``digest`` and ``spec_approved`` are
the one meaning of "the specification is approved".
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from .store import DecisionRefused, StoreRefused, read_record, root_of, safe_segment, write_record

__all__ = ["settle_decision"]  # the facade lists only state-api names

RECORD_CAP = 4_000
PACKET_CAP = 12_000

_KEY_RE = re.compile(r"^[a-z][a-z0-9_-]*$")

_REQUIRED, _OPTIONAL, _REPEAT = "required", "optional", "repeat"


@dataclass(frozen=True)
class Choice:
    """One choice a kind accepts: its verb, the status it sets, the keys its
    line shows and no other, the values those keys may take, whether a note
    is required, the reasons the record may carry for the choice to be
    available, and whether the slice builds it."""

    verb: str
    status: str
    keys: dict[str, str] = field(default_factory=dict)
    values: dict[str, tuple[str, ...]] = field(default_factory=dict)
    note_required: bool = False
    any_of: frozenset[str] = frozenset()
    reasons: frozenset[str] = frozenset()
    built: bool = True


def _c(line: str, *, status: str, keys: dict[str, str] | None = None,
       values: dict[str, tuple] | None = None, note: bool = False, any_of=(),
       reasons=(), built: bool = True) -> Choice:
    verb, _, _rest = line.partition(" ")
    return Choice(
        verb,
        status,
        keys=dict(keys or {}),
        values={key: tuple(allowed) for key, allowed in (values or {}).items()},
        note_required=note,
        any_of=frozenset(any_of),
        reasons=frozenset(reasons),
        built=built,
    )


def _group(choices: list[Choice]) -> tuple[Choice, ...]:
    return tuple(choices)


#: tables/decision-kinds.toml as data. The verb maps to the status of the list
# it is in (topic decisions: approve -> approved, revise and hold -> revise,
# drop, reject and descope -> rejected). ``amend`` and ``open-spike`` refuse
# ``not_built`` until their request files come with the build that needs them
# (decision D10).
KINDS: dict[str, dict[str, tuple[Choice, ...]]] = {
    "brief": {
        "approved": _group([_c("approve", status="approved")]),
        "revise": _group([_c("revise -- <guidance>", status="revise", note=True)]),
        "rejected": _group([_c("drop", status="rejected")]),
    },
    "spec": {
        "approved": _group([
            _c("approve [variant=<id>] [topology=single or split]", status="approved",
               keys={"variant": _OPTIONAL, "topology": _OPTIONAL},
               values={"topology": ("single", "split")}),
        ]),
        "revise": _group([_c("revise [stage=<name>] -- <guidance>", status="revise",
                             keys={"stage": _OPTIONAL}, note=True)]),
        "rejected": _group([_c("drop", status="rejected")]),
    },
    "release": {
        "approved": _group([_c("approve", status="approved")]),
        "revise": _group([_c("hold -- <guidance>", status="revise", note=True)]),
        "rejected": _group([]),
    },
    "ideas": {
        "approved": _group([
            _c("select pursue=<ids> research=<ids> defer=<ids> reject=<ids>", status="approved",
               keys={"pursue": _OPTIONAL, "research": _OPTIONAL, "defer": _OPTIONAL,
                     "reject": _OPTIONAL},
               any_of=("pursue", "research", "defer", "reject")),
        ]),
        "revise": _group([]),
        "rejected": _group([]),
    },
    "merge": {
        "approved": _group([_c("approve", status="approved")]),
        "revise": _group([]),
        "rejected": _group([_c("reject", status="rejected")]),
    },
    "plan": {
        "approved": _group([_c("approve", status="approved")]),
        "revise": _group([_c("revise -- <guidance>", status="revise", note=True)]),
        "rejected": _group([]),
    },
    "finding": {
        "approved": _group([
            _c("waive key=<key> [key=<key> ...] -- <reason>", status="approved",
               keys={"key": _REPEAT}, note=True),
            _c("waive-coverage item=<item> [item=<item> ...] -- <reason>", status="approved",
               keys={"item": _REPEAT}, note=True),
        ]),
        "revise": _group([_c("direct -- <guidance>", status="revise", note=True)]),
        "rejected": _group([_c("descope", status="rejected")]),
    },
    "spec-gap": {
        "approved": _group([_c("amend", status="approved", built=False)]),
        "revise": _group([_c("not-a-gap -- <reason>", status="revise", note=True)]),
        "rejected": _group([_c("descope", status="rejected")]),
    },
    "blocked": {
        "approved": _group([
            _c("retry", status="approved"),
            _c("open-spike name=<name> (for spike_limit)", status="approved",
               keys={"name": _REQUIRED}, reasons=("spike_limit",), built=False),
            _c("skip (for an optional environment check)", status="approved",
               reasons=("environment",)),
        ]),
        "revise": _group([_c("direct -- <guidance>", status="revise", note=True)]),
        "rejected": _group([_c("descope", status="rejected")]),
    },
}

#: The four reasons a blocked decision carries (tables/decision-kinds.toml,
#: field reason in the decision-record schema).
BLOCKED_REASONS = ("spike_limit", "environment", "stuck", "reviewer")


def _choices_of(kind: str) -> list[Choice]:
    return [choice for status in KINDS[kind].values() for choice in status]


def _cycle_names(project) -> list[str]:
    from .cycles import cycles_dir

    directory = cycles_dir(project)
    names: list[str] = []
    if directory.is_dir():
        names = sorted(
            entry.name for entry in directory.iterdir() if (entry / "cycle.json").is_file()
        )
    if "_standing" not in names:
        names.append("_standing")
    return names


def _decision_path(project, cycle: str, bare: str):
    from .cycles import cycle_dir

    checked_cycle, checked_bare = safe_segment(cycle), safe_segment(bare)
    return cycle_dir(project, checked_cycle) / "decisions" / f"{checked_bare}.json"


def _decisions_dir(project, cycle: str):
    from .cycles import cycle_dir

    return cycle_dir(project, safe_segment(cycle)) / "decisions"


def parse_decide(tokens: list[str]) -> tuple[str, str, list[tuple[str, str]], str]:
    """The one grammar (topic decisions): id, choice, ``key=value`` pairs as
    typed, and the note — the tokens after a lone ``--``, joined by one
    space. Keys match ``[a-z][a-z0-9_-]*``; a value with no key, an empty
    value or a malformed key refuses ``bad_grammar``."""
    if len(tokens) < 2 or not tokens[0] or not tokens[1]:
        raise StoreRefused("bad_grammar", "usage: decide <id> <choice> [key=value ...] [-- note]")
    ident, choice = tokens[0], tokens[1]
    pairs: list[tuple[str, str]] = []
    note_parts: list[str] = []
    in_note = False
    for token in tokens[2:]:
        if in_note:
            note_parts.append(token)
            continue
        if token == "--":
            in_note = True
            continue
        key, sep, value = token.partition("=")
        if not sep or not _KEY_RE.fullmatch(key) or not value:
            raise StoreRefused(
                "bad_grammar", f"every value is named and a key may repeat: {token!r}"
            )
        pairs.append((key, value))
    return ident, choice, pairs, " ".join(note_parts)


def resolve_decision(project, id: str) -> tuple[str, str]:
    """The cycle and bare id of ``id``: ``id@cycle`` as written; a bare id is
    sought in the open cycles and ``_standing`` (decision D9) — the one
    record of that id whose status is ``open``, else the one record; more
    than one at either level is ``ambiguous``, with the qualified ids in
    ``.choices``. Closed cycles are never searched, so a settled decision in
    a closed cycle cannot make a bare read ambiguous."""
    from .cycles import cycles_open

    if "@" in id:
        bare, _, cycle = id.partition("@")  # <id>@<cycle>, the id may hold dots
        record = read_record(_decision_path(project, cycle, bare))
        if record is None:
            raise StoreRefused("unknown", f"no decision {id!r}")
        return cycle, bare
    candidates = [record["id"] for record in cycles_open(project)]
    if "_standing" not in candidates:
        candidates.append("_standing")
    for prefer_open in (True, False):
        holders: list[str] = []
        for cycle in candidates:
            record = read_record(_decision_path(project, cycle, id))
            if record is None:
                continue
            if prefer_open and record.get("status") != "open":
                continue
            holders.append(cycle)
        if len(holders) == 1:
            return holders[0], id
        if len(holders) > 1:
            raise DecisionRefused(
                "ambiguous",
                f"{id!r} names a decision in more than one cycle; qualify it as "
                + " or ".join(f"{id}@{cycle}" for cycle in sorted(holders)),
                choices=sorted(f"{id}@{cycle}" for cycle in holders),
            )
    raise StoreRefused("unknown", f"no decision {id!r}")


def decision_read(project, id: str) -> dict:
    """The decision record, resolving a bare id like
    :func:`resolve_decision`; ``unknown`` when there is none."""
    cycle, bare = resolve_decision(project, id)
    record = read_record(_decision_path(project, cycle, bare))
    if record is None:
        raise StoreRefused("unknown", f"no decision {id!r}")
    return record


def decisions_list(project, *, cycle=None, kind=None) -> list[tuple[str, dict]]:
    """Every decision record as ``(qualified id, record)``, by cycle and id."""
    found: list[tuple[str, dict]] = []
    names = [cycle] if cycle is not None else _cycle_names(project)
    for name in names:
        directory = _decisions_dir(project, name)
        if not directory.is_dir():
            continue
        for path in sorted(directory.glob("*.json")):
            record = read_record(path, "decision-record")
            if record is None:
                continue
            if kind is not None and record.get("kind") != kind:
                continue
            found.append((f"{record['id']}@{name}", record))
    return found


def latest_decision(project, kind: str) -> tuple[str, dict] | None:
    """The newest decision of the kind, by ``opened``."""
    found = decisions_list(project, kind=kind)
    if not found:
        return None
    found.sort(key=lambda pair: (pair[1].get("opened", ""), pair[0]))
    return found[-1]


def _stale(project, record: dict) -> bool:
    """Whether what the decision approves has changed since it was opened.

    A ``spec`` decision covers the current specification file set, not the
    list it stored: a contract added after the approval stales it, and a
    covered file deleted since reads as stale too, never as a missing-file
    refusal. Every other kind digests exactly the files it stored.
    """
    if record.get("kind") == "spec":
        from .digests import spec_digest

        try:
            current, files = spec_digest(project)
        except StoreRefused:
            return True  # a file of the set is gone: stale, not an error
        stored = record.get("digest", "")
        if not stored and not files:
            return False  # nothing was bound and the set is empty
        return current != stored
    if not record.get("files"):
        return False
    from .digests import digest

    try:
        current = digest(record["files"], root=root_of(project))
    except StoreRefused:
        return True
    return current != record.get("digest", "")


def decision_status(project, id: str) -> str:
    """The stored status of the decision — or ``stale`` when that is ``open``
    or ``approved`` and the kind's digest no longer matches (topic digests)."""
    record = decision_read(project, id)
    status = record.get("status", "")
    if status in ("open", "approved") and _stale(project, record):
        return "stale"
    return status


def spec_approved(project) -> tuple[bool, str]:
    """The one meaning of "the specification is approved": the latest ``spec``
    decision reads ``approved`` and its digest still matches."""
    latest = latest_decision(project, "spec")
    if latest is None:
        return False, "no spec decision"
    qualified, _record = latest
    status = decision_status(project, qualified)
    if status != "approved":
        return False, status
    return True, ""


def decision_open(
    project,
    *,
    cycle: str,
    kind: str,
    packet: str,
    subject: str = "",
    digest: str = "",
    files=(),
    reason: str = "",
    dependencies=(),
    now: str | None = None,
) -> dict:
    """Write the ``open`` record and the packet, replacing that id's record.
    Refuses ``bad_kind``, ``bad_reason`` (a blocked decision carries one of
    the four reasons) and ``over_cap``. The packet goes first, through the
    temporary directory and a replace; the record is the commit point, so a
    kill between the two writes leaves no open decision without a packet."""
    from .store import now as now_stamp, write_via_tmp

    if kind not in KINDS:
        raise StoreRefused("bad_kind", f"{kind!r} is not one of the nine kinds")
    if kind == "blocked":
        if not reason:
            raise StoreRefused("bad_reason", "a blocked decision carries its reason")
        if reason not in BLOCKED_REASONS:
            raise StoreRefused(
                "bad_reason",
                f"a blocked decision's reason is one of "
                + ", ".join(BLOCKED_REASONS),
            )
    ident = kind if not subject else f"{kind}.{subject}"
    safe_segment(cycle)
    safe_segment(ident)  # id and subject are path segments: traversal refuses here
    record = {
        "id": ident,
        "kind": kind,
        "status": "open",
        "choice": "",
        "values": {},
        "note": "",
        "digest": digest,
        "opened": now or now_stamp(),
    }
    if reason:
        record["reason"] = reason
    if files:
        record["files"] = list(files)
    if dependencies:
        record["dependencies"] = list(dependencies)
    directory = _decision_path(project, cycle, ident).parent
    packet_path = directory / f"{ident}.md"
    data = packet.encode("utf-8")
    if len(data) > PACKET_CAP:
        raise StoreRefused("over_cap", f"packet {packet_path.name} is {len(data)} bytes")
    directory.mkdir(parents=True, exist_ok=True)
    write_via_tmp(packet_path, data)
    write_record(
        _decision_path(project, cycle, ident), record, cap=RECORD_CAP, schema="decision-record"
    )
    return record


def _normalise(values) -> dict[str, str]:
    """The pair list or mapping as the record stores it; a repeated key is
    stored as its values joined by one space (decision D9)."""
    if isinstance(values, dict):
        return {str(key): str(value) for key, value in values.items()}
    joined: dict[str, str] = {}
    for key, value in values:
        key, value = str(key), str(value)
        joined[key] = f"{joined[key]} {value}" if key in joined else value
    return joined


def settle_decision(
    project,
    id: str,
    choice: str,
    values,
    note: str,
    session_id: str,
    *,
    settled_by: str | None = None,
    now: str | None = None,
) -> dict:
    """Settle the decision: write the record and nothing else.

    Refuses in this order: ``unknown``, ``ambiguous``, ``not_open``,
    ``stale``, ``bad_choice`` (listing the kind's choices, and raised too
    when the record's reason does not allow the choice), ``bad_key``,
    ``note_required``, ``not_built``. The read-modify-write holds
    ``flush.lock`` (decision D1). ``settled_by`` defaults to ``session``
    with a session id, else ``terminal``.
    """
    from .store import locked, now as now_stamp

    cycle, bare = resolve_decision(project, id)
    path = _decision_path(project, cycle, bare)
    with locked(project, "flush.lock"):
        record = read_record(path, "decision-record")
        if record is None:
            raise StoreRefused("unknown", f"no decision {id!r}")
        if record.get("status", "") != "open":
            raise StoreRefused("not_open", f"{id!r} is {record.get('status')}, not open")
        if _stale(project, record):
            raise StoreRefused("stale", f"what {id!r} approves has changed since it was opened")
        if record.get("kind") not in KINDS:
            raise StoreRefused("bad_kind", f"{record.get('kind')!r} is not one of the nine kinds")
        kind_choices = _choices_of(record["kind"])
        by_verb = {choice.verb: choice for choice in kind_choices}
        picked = by_verb.get(choice)
        if picked is None:
            raise DecisionRefused(
                "bad_choice",
                f"{choice!r} is not a choice of {record['kind']}",
                choices=[c.verb for c in kind_choices],
            )
        if picked.reasons and record.get("reason", "") not in picked.reasons:
            raise DecisionRefused(
                "bad_choice",
                f"{choice} is available only to a decision opened with the reason "
                + " or ".join(sorted(picked.reasons))
                + f"; this one carries {record.get('reason', '')!r}",
                choices=[c.verb for c in kind_choices],
            )
        stored_values = _normalise(values)
        if not isinstance(values, dict):
            seen_counts: dict[str, int] = {}
            for key, _value in values:
                seen_counts[str(key)] = seen_counts.get(str(key), 0) + 1
            for key, count in seen_counts.items():
                if count > 1 and picked.keys.get(key) != _REPEAT:
                    raise StoreRefused("bad_key", f"{key!r} may not repeat on {choice}")
        for key in stored_values:
            if key not in picked.keys:
                raise StoreRefused(
                    "bad_key", f"{key!r} is not a key of {choice}; the line shows {picked.keys}"
                )
        for key, allowed in picked.values.items():
            if key in stored_values and stored_values[key] not in allowed:
                raise StoreRefused(
                    "bad_key",
                    f"{key} must be one of " + ", ".join(allowed)
                    + f"; got {stored_values[key]!r}",
                )
        for key, mode in picked.keys.items():
            if mode in (_REQUIRED, _REPEAT) and key not in stored_values:
                raise StoreRefused("bad_key", f"{choice} needs {key}=<value>")
        if picked.any_of and not any(key in stored_values for key in picked.any_of):
            raise StoreRefused(
                "bad_key",
                f"{choice} needs at least one of " + ", ".join(sorted(picked.any_of)),
            )
        if picked.note_required and not note:
            raise StoreRefused("note_required", f"{choice} carries a note: {choice} -- <text>")
        if not picked.built:
            raise StoreRefused(
                "not_built", f"{choice} is not built yet; the slice leaves it out (decision D10)"
            )
        record["status"] = picked.status
        record["choice"] = choice
        record["values"] = stored_values
        record["note"] = note
        record["settled"] = now or now_stamp()
        record["settled_by"] = settled_by or ("session" if session_id else "terminal")
        record["settled_by_session"] = session_id or ""
        write_record(path, record, cap=RECORD_CAP, schema="decision-record")
        return record
