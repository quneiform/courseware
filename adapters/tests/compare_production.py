"""Compare two published adapter releases against one reviewed source and admission receipt."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from google.protobuf import descriptor_pb2, descriptor_pool, message_factory


def verify(old, new, courses, receipt, schema):
    pool = descriptor_pool.DescriptorPool()
    for descriptor in descriptor_pb2.FileDescriptorSet.FromString(schema.read_bytes()).file:
        pool.Add(descriptor)

    def decode(kind, wire):
        return message_factory.GetMessageClass(pool.FindMessageTypeByName(kind)).FromString(wire)

    overlay = courses / "connected-lessons"
    def export(executable, *flags):
        return subprocess.check_output([executable, overlay/"root.json", courses,
                                        overlay/"learning-map.toml", *flags])

    proof = json.loads(receipt.read_bytes())
    for alias, row in {"catalog":proof["catalog"], "map":proof["map"], **proof["editions"]}.items():
        flags = {"catalog":["--root-posit"], "map":["--posit-wire"]}.get(alias, ["--course-posit",alias])
        before, after = [export(exe, *flags) for exe in (old,new)]
        assert hashlib.sha256(before).hexdigest() == row["posit_sha256"], alias
        if alias in ("catalog","map"):
            assert before == after, alias
        else:
            a,b = (decode("qune.core.Posit", wire) for wire in (before,after))
            assert a.id != b.id and len(a.diff.edits) == len(b.diff.edits)
            a.ClearField("id")
            b.ClearField("id")
            i, = [i for i, pair in enumerate(zip(a.diff.edits,b.diff.edits)) if pair[0] != pair[1]]
            x,y = a.diff.edits[i],b.diff.edits[i]
            assert x.path == y.path and x.target == y.target
            assert x.path.schema.full_name == "quneic.learning.v1.AdaptedCourse"
            assert x.path.path.elements[0].field_number == 1

            p,q = [decode("quneic.learning.v1.AdaptedCourse", export(exe,"--adapted-course",alias))
                   for exe in (old,new)]
            assert p.basis.adapter != q.basis.adapter
            p.basis.ClearField("adapter")
            q.basis.ClearField("adapter")
            assert p == q
            del a.diff.edits[i]
            del b.diff.edits[i]
            assert a == b
        print(alias, row["posit_sha256"], hashlib.sha256(after).hexdigest())


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    for arg in ("old","new","courses","receipt","schema"):
        parser.add_argument(arg,type=lambda p: Path(p).resolve(strict=True))
    verify(**vars(parser.parse_args()))
