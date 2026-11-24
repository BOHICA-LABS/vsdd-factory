"""The event log writer and reader (hooks.md 2.12)."""

import json
import threading
from pathlib import Path

import pytest

from vsdd.events import (
    EVENT_SCHEMA,
    EventLog,
    EventError,
    cut_command,
    events_dir_for,
    redact,
)


@pytest.fixture
def log(tmp_path: Path) -> EventLog:
    return EventLog(tmp_path / "events", "sess-1")


class TestWriteRead:
    def test_line_carries_the_required_fields(self, log, tmp_path):
        log.log("rule.block", severity="block", rule="tests-first", code="no_red_evidence")
        assert len(list((tmp_path / "events").glob("*.jsonl"))) == 1
        event = log.read()[0]
        assert event["v"] == 1
        assert event["ts"].endswith("Z") and "T" in event["ts"]
        assert event["type"] == "rule.block"
        assert event["severity"] == "block"
        assert event["session_id"] == "sess-1"
        assert event["rule"] == "tests-first"

    def test_daily_file_name_is_the_utc_date(self, log, tmp_path):
        import datetime as dt

        log.log("session.start", source="test")
        today = dt.datetime.now(dt.timezone.utc).date()
        assert (tmp_path / "events" / f"{today.isoformat()}.jsonl").is_file()

    def test_reader_skips_torn_lines(self, log, tmp_path):
        log.log("session.start")
        path = next((tmp_path / "events").glob("*.jsonl"))
        with open(path, "a", encoding="utf-8") as fh:
            fh.write('{"v":1,"broken":\n')
        assert len(log.read()) == 1

    def test_read_on_a_fresh_directory_is_empty(self, log):
        assert log.read() == []

    def test_concurrent_appends_do_not_interleave(self, tmp_path):
        log = EventLog(tmp_path / "events", "s")
        n_threads, per_thread = 8, 25

        def worker(tag: int) -> None:
            for i in range(per_thread):
                log.log("step.recorded", tag=tag, index=i)

        threads = [threading.Thread(target=worker, args=(t,)) for t in range(n_threads)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        assert len(log.read()) == n_threads * per_thread


class TestValidation:
    def test_type_must_be_dotted(self, log):
        with pytest.raises(EventError):
            log.build("undotted", "info")

    def test_severity_must_be_in_the_list(self, log):
        with pytest.raises(EventError):
            log.build("rule.block", "fatal")

    def test_session_id_must_not_be_empty(self, tmp_path):
        with pytest.raises(EventError):
            EventLog(tmp_path, "   ").build("session.start")

    def test_schema_constant_declares_the_contract(self):
        assert EVENT_SCHEMA["$id"] == "event.schema.json"
        assert EVENT_SCHEMA["required"] == ["v", "ts", "type", "severity", "session_id"]


class TestHygiene:
    def test_redact_masks_secret_shaped_pairs(self):
        assert redact("AWSECRET_KEY=hunter2 and the rest") == "AWSECRET_KEY=<redacted> and the rest"

    def test_redact_masks_common_tokens(self):
        text = "token ghp_" + "a" * 36 + " done"
        assert redact(text) == "token <redacted> done"

    def test_command_cut_to_two_hundred_characters(self):
        assert cut_command("x" * 500) == "x" * 199 + "…"

    def test_logged_command_is_cut_and_redacted(self, log, tmp_path):
        secret = "GH_TOKEN=" + "z" * 300
        log.log("runner.slow", command=secret)
        event = log.read()[0]
        # Redaction runs before the cut: no secret material survives at all,
        # and the line stays inside the command budget.
        assert len(event["command"]) <= 200
        assert "z" * 100 not in event["command"]
        assert "<redacted>" in event["command"]

    def test_none_fields_are_dropped(self, log, tmp_path):
        log.log("session.start", source=None, reason="test")
        event = log.read()[0]
        assert "source" not in event
        assert event["reason"] == "test"


class TestCaps:
    def test_four_kb_cap_keeps_required_fields(self, tmp_path):
        log = EventLog(tmp_path / "events", "s")
        huge = "y" * 9000
        log.log("runner.slow", detail=huge, extra=huge, more=huge, notes=huge)
        raw = (tmp_path / "events").glob("*.jsonl")
        line = next(iter(raw)).read_text(encoding="utf-8").strip()
        assert len(line.encode("utf-8")) <= 4096
        event = json.loads(line)
        for key in ("v", "ts", "type", "severity", "session_id"):
            assert key in event

    def test_twenty_mb_day_cap_silences_info_but_not_blocks(self, tmp_path):
        directory = tmp_path / "events"
        log = EventLog(directory, "s")
        log.log("session.start")  # creates today's file
        # Pad the day file past the cap directly: the 4 KB line cap would
        # shrink padded events, so the size pressure comes from real lines.
        day_file = next(iter(directory.glob("*.jsonl")))
        with open(day_file, "a", encoding="utf-8") as fh:
            pad = json.dumps({"v": 1, "ts": "t", "type": "x.y", "severity": "info", "session_id": "s", "blob": "z" * 1024}, separators=(",", ":"))
            for _ in range(21 * 1024):
                fh.write(pad + "\n")
        assert log.log("session.start", source="again") is False  # silenced
        assert log.log("rule.block", severity="block", rule="r") is True
        events = [e for e in log.read() if e["type"] != "x.y"]  # skip the padding
        assert [("session.start", None), ("rule.block", None)] == [
            (e["type"], e.get("source")) for e in events
        ]


class TestPrune:
    def test_old_files_removed_recent_kept(self, log, tmp_path):
        import datetime as dt

        log.log("session.start")
        day = dt.datetime.now(dt.timezone.utc).date()
        old = tmp_path / "events" / f"{(day - dt.timedelta(days=40)).isoformat()}.jsonl"
        old.write_text('{"v":1}\n', encoding="utf-8")
        not_daily = tmp_path / "events" / "handmade.jsonl"
        not_daily.write_text("{}\n", encoding="utf-8")
        removed = log.prune(retention_days=30)
        assert removed == [old.name]
        assert old.exists() is False
        assert not_daily.exists() is True
        assert len(log.read()) == 1


class TestOverride:
    def test_env_override_directory(self, tmp_path, monkeypatch):
        monkeypatch.setenv("VSDD_EVENTS_DIR", str(tmp_path / "elsewhere"))
        assert events_dir_for(tmp_path) == tmp_path / "elsewhere"

    def test_no_env_uses_the_state_store(self, tmp_path, monkeypatch):
        monkeypatch.delenv("VSDD_EVENTS_DIR", raising=False)
        assert events_dir_for(tmp_path) == tmp_path / ".factory" / "local" / "events"


class TestWriterValidatesAgainstTheSchema:
    """build() validates every event against EVENT_SCHEMA itself, so the
    documented schema cannot drift from what the writer accepts (hooks.md
    2.12) — and the schema carries real machine-readable values."""

    def test_non_integer_ms_is_refused(self, log):
        with pytest.raises(EventError, match="ms"):
            log.build("rule.block", ms="not-an-int")

    def test_non_string_agent_type_is_refused(self, log):
        with pytest.raises(EventError, match="agent_type"):
            log.build("rule.block", agent_type=42)

    def test_written_line_passes_event_schema(self, log, tmp_path):
        from vsdd.schema import validate

        log.log("rule.block", severity="block", rule="tests-first", ms=12, command="cargo test")
        (line,) = log.read()
        assert validate(line, EVENT_SCHEMA) == []

    def test_ts_pattern_compiles_and_matches_a_written_ts(self, log, tmp_path):
        import re

        log.log("session.start")
        (line,) = log.read()
        pattern = re.compile(EVENT_SCHEMA["properties"]["ts"]["pattern"])
        assert pattern.fullmatch(line["ts"]), line["ts"]

    def test_v_is_enumerated(self):
        assert EVENT_SCHEMA["properties"]["v"]["enum"] == [1]


class TestRedaction:
    def test_quoted_value_with_spaces_is_masked_whole(self):
        assert redact('PASSWORD="correct horse battery staple"') == "PASSWORD=<redacted>"
        assert redact("API_KEY='abc def ghi'") == "API_KEY=<redacted>"

    def test_flag_values_are_masked(self):
        assert redact("--password hunter2") == "--password <redacted>"
        assert redact("--password=hunter2") == "--password=<redacted>"
        assert redact("--api-token=abc123") == "--api-token=<redacted>"

    def test_basic_auth_user_is_masked(self):
        assert redact("curl -u admin:s3cr3tpass") == "curl -u admin:<redacted>"

    def test_secret_header_lines_are_masked(self):
        assert redact("X-Api-Key: supersecret") == "X-Api-Key: <redacted>"

    def test_detail_leaves_are_redacted_recursively(self, log):
        event = log.build(
            "rule.block",
            detail={"command": "GH_TOKEN=supersecretvalue123", "notes": ["COOKIE=stealth-mode"],
                    "count": 2},
        )
        assert event["detail"]["command"] == "GH_TOKEN=<redacted>"
        assert event["detail"]["notes"] == ["COOKIE=<redacted>"]
        assert event["detail"]["count"] == 2
