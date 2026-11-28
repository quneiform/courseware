"""Compile the standalone Edition 2023 schema and verify its example binary encoding."""
from pathlib import Path
import os
import subprocess
import tempfile

root = Path(__file__).resolve().parent.parent
protoc = os.environ.get("PROTOC", "protoc")
schema = [f"--proto_path={root / 'schema'}", "courseware.proto"]
message = "courseware.v05.CourseReading"

def run(flag, data):
    return subprocess.run([protoc, f"--{flag}={message}", *schema],
                          input=data, stdout=subprocess.PIPE, check=True).stdout

with tempfile.TemporaryDirectory() as directory:
    descriptor = Path(directory) / "courseware.fds"
    subprocess.run([protoc, f"--descriptor_set_out={descriptor}", *schema], check=True)
    assert descriptor.read_bytes() == (root / "schema/courseware.fds").read_bytes(), "stale question schema"

wire = run("encode", (root / "tests/fixtures/course-reading.textproto").read_bytes())
decoded = run("decode", wire)
assert b'starter: ""' in decoded and b'language: "rust"' in decoded
assert run("encode", decoded) == wire
print(f"{message} round-tripped {len(wire)} bytes")
