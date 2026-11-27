"""Open a real edition through a hosted Git reader and query its public archive service."""
import argparse
from contextlib import contextmanager
from pathlib import Path
import struct
import subprocess

from google.protobuf import descriptor_pb2, descriptor_pool, message_factory

def messages():
    path = Path(__file__).resolve().parents[1] / "schema/archive.fds"
    pool = descriptor_pool.DescriptorPool()
    files = descriptor_pb2.FileDescriptorSet.FromString(path.read_bytes()).file
    released = descriptor_pb2.FileDescriptorSet.FromString(
        path.with_name("courseware.fds").read_bytes()
    ).file
    assert all(any(f == original for f in files) for original in released)
    for file in files:
        pool.Add(file)
    def cls(name):
        return message_factory.GetMessageClass(pool.FindMessageTypeByName("courseware.archive.v1." + name))
    return cls("Request"), cls("Response")

Request, Response = messages()

@contextmanager
def session(exe):
    with subprocess.Popen([exe, "--archive-session-v1"], stdin=subprocess.PIPE, stdout=subprocess.PIPE) as proc:
        def send(message):
            data = message.SerializeToString()
            proc.stdin.write(struct.pack(">I", len(data)) + data)
            proc.stdin.flush()
        def receive():
            header = proc.stdout.read(4)
            assert len(header) == 4
            size, = struct.unpack(">I", header)
            assert 0 < size <= 128 << 20
            return Response.FromString(proc.stdout.read(size))
        try:
            yield send, receive
        finally:
            proc.stdin.close()

def verify(exe, course):
    qid = "87c3b09e-84d3-49d7-8680-0b2d068a7fc5"
    with session(exe) as (send, receive):
        send(Request(open={"directory":bytes(course), "hosted_git":True}))
        req = receive()
        assert req.WhichOneof("body") == "git"
        git = req.git
        assert (Path(git.directory.decode()).resolve()).is_relative_to(course)
        assert len(bytes.fromhex(git.revision)) == 20
        data = subprocess.check_output(["git", "-C", git.directory, "archive", "--format=tar", git.revision, "--", *git.paths])
        send(Request(archive=data))
        opened = receive()
        assert opened.WhichOneof("body") == "opened", opened.failure
        book = opened.opened.courses[""]
        assert book.chapters and book.navigation.roots
        question = next(q for q in book.questions if q.id == qid)
        assert "function" in question.prompt
        assert "answer" not in question.DESCRIPTOR.fields_by_name
        def query(op, key, answers=(), alias=""):
            send(Request(query={"tablet":alias,"key":key,"operation":op,"answers":answers}))
            return receive()
        assert not query("CHECK", qid, ["start"]).answer.correct
        assert query("CHECK", qid, ["  MAIN "]).answer.correct
        assert query("REVEAL", qid).answer.explanation
        assert query("CHECK", qid, ["main"], "elsewhere").WhichOneof("body") == "failure"
        assert query("ASSET", "../outside").WhichOneof("body") == "failure"
        assert query("ASSET", "img/trpl15-01.svg").answer.blob.media_type == "image/svg+xml"
    with session(exe) as (send, receive):
        send(Request(open={"directory":bytes(course), "hosted_git":True}))
        assert receive().WhichOneof("body") == "git"
        # The embedding host refuses its archive. A native reread would leak authority.
        send(Request(archive=b"invalid tar"))
        assert receive().WhichOneof("body") == "failure"
    print("PASS hosted Git grant, pinned public reading, private grade/reveal, scoped assets and refusals")

if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("executable", type=Path)
    p.add_argument("course", type=Path)
    a = p.parse_args()
    verify(a.executable.resolve(), a.course.resolve())
