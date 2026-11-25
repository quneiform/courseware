"""Check the pinned, dependency-free Logic exercise with its course compiler."""
from pathlib import Path
import subprocess
import sys
import tomllib

root = Path(__file__).resolve().parents[1]
exercise = tomllib.loads((root / "exercise.toml").read_text())
revision = tomllib.loads((root / "course.toml").read_text())["revision"]

def source(path):
    return subprocess.check_output(["git", "-C", root / "upstream", "show", f"{revision}:{path}"], text=True)

compiler = exercise["toolchain"]
assert source("lean-toolchain").strip() == compiler
lines = source(exercise["path"]).splitlines()
statement, unfinished = lines[exercise["first_line"] - 1:exercise["last_line"]]
assert statement.endswith(":= by") and unfinished.strip() == "sorry"
for proof, expected in [("exact Or.inl h.left", 0), ("exact h", 1), ("sorry", 1)]:
    # These are fixed, documented fixtures. This verifier executes no learner code.
    result = subprocess.run(["elan", "run", compiler, "lean", "-DwarningAsError=true", "--stdin"],
                            input=f"{statement}\n  {proof}\n", text=True, capture_output=True)
    if result.returncode != expected:
        sys.exit(f"Unexpected result for {proof}: {result.stdout}{result.stderr}")
    if expected and ("type mismatch" if proof == "exact h" else "uses 'sorry'") not in result.stdout:
        sys.exit(f"Unexpected diagnostic for {proof}: {result.stdout}")
    print(f"{exercise['id']}: {proof}: {'accepted' if not expected else 'rejected'}")
