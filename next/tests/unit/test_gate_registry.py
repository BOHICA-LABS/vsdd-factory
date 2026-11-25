"""The gate registry (plan gates.md 3.4, D2; ORDER.md section 3 rule 2)."""

import re
import subprocess
import sys
from pathlib import Path

from tests.unit import gate_project
from tests.unit.gate_project import STORY
from vsdd.state.gate_evidence import registered_gate

NEXT = Path(__file__).resolve().parents[2]
PY = NEXT / "plugins" / "vsdd-factory" / "py"

_GATE_LINE = re.compile(r"^\s*(from|import)\s+vsdd\.gates", re.M)


def test_gates_register_by_file_name():
    """A fresh interpreter, no gate import: the lookup imports the
    ``gate_<name>.py`` modules by file name and finds the functions."""
    program = (
        "import sys; sys.path.insert(0, {!r}); "
        "from vsdd.state.gate_evidence import registered_gate; "
        "print(all(callable(registered_gate(n)) for n in ('red', 'green', 'demo')))"
    ).format(str(PY))
    done = subprocess.run([sys.executable, "-B", "-c", program], capture_output=True, text=True)
    assert done.returncode == 0, done.stderr
    assert done.stdout.strip().endswith("True")


def test_registered_function_attaches_and_can_be_pending(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env_key = gate_project.write_caller(root, gate_project.IMPLEMENTER)
    import io

    from vsdd.cli import dispatch

    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "survey", "story", "--subject", STORY, "--json"],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ={"HOME": str(Path(tmp_path).parent), "VSDD_CALLER": env_key},
    )
    _ = code
    fn = registered_gate("red")
    assert callable(fn)
    pending = fn(root, STORY, run, wait=0)
    assert pending == {
        "ok": True,
        "state": "pending",
        "retry": f"vsdd gate red --story {STORY} --run {run}",
    }
    # the same measurement, awaited: one run of the suite serves the result
    result = fn(root, STORY, run, wait=120)
    assert result["ok"] is True and result["passed"] is True, result.get("failures")
    assert result["metrics"]["new"] == 3
    assert result["metrics"]["seconds"] >= 0


def test_demo_is_direct(tmp_path):
    """Demo is direct: the registered function takes the registry contract's
    wait and retry, answers at once and is never pending."""
    from tests.unit.test_gate_demo import demo_ready
    from vsdd.state.gate_evidence import registered_gate

    root, run, _env, _green = demo_ready(tmp_path)
    fn = registered_gate("demo")
    result = fn(root, STORY, run, wait=0, retry="x")
    assert result["ok"] is True and result["passed"] is True, result["failures"]
    assert "state" not in result


def test_state_imports_no_gate_module():
    """State code imports no gate module (the registry imports by file name
    on the first lookup); a static import in a state module is a defect."""
    state_dir = PY / "vsdd" / "state"
    offenders = []
    for path in sorted(state_dir.glob("*.py")):
        text = path.read_text()
        if _GATE_LINE.search(text):
            offenders.append(path.name)
    assert offenders == [], offenders
