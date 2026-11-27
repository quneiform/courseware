"""Verify the public entry artifacts against the local adapter and reader release."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tomllib
from google.protobuf import descriptor_pb2, descriptor_pool, message_factory


COURSES = Path(__file__).resolve().parent
ENTRIES = (
    (COURSES, "ch06-02-match", "The `match` Control Flow Construct"),
    (COURSES / "rust-book", "ch01-02-hello-world", "Hello, World!"),
)


def message(descriptor, name):
    pool = descriptor_pool.DescriptorPool()
    for file in descriptor_pb2.FileDescriptorSet.FromString(descriptor.read_bytes()).file:
        pool.Add(file)
    return message_factory.GetMessageClass(pool.FindMessageTypeByName(name))


def verify(args):
    skene = message(args.compose_schema, "qune.compose.Skene")
    posit = message(COURSES.parent/"contracts/quneic.learning/0.5.0/learning.fds", "qune.core.Posit")
    release = skene.FromString((args.reader_release/"skene.pb").read_bytes())
    method_name = ("qn.tablet.reader.located.v05.Reader/Render" if args.located else
                   "qn.tablet.reader.attributed.v05.Reader/Render")
    method, = [p.schema for p in release.provides if p.schema.full_name == method_name]
    method_descriptor = args.reader_release / ("located.fds" if args.located else "attributed.fds")
    assert method.descriptor.value == hashlib.sha256(method_descriptor.read_bytes()).digest(), method_name
    schemas = {"qn.tablet.v05.Chapter": "tablet.fds", "qn.tablet.reader.v05.Actions": "reader.fds"}
    if args.located:
        schemas.update({"qn.tablet.assets.v05.ChapterAssets": "assets.fds", "qn.tablet.location.v05.Location": "location.fds"})
    courses = args.courses or COURSES
    for original, chapter, title in ENTRIES:
        if args.located and original == COURSES:
            continue
        checkout = courses / original.relative_to(COURSES)
        with (checkout/"Qrate.toml").open("rb") as stream:
            manifest = tomllib.load(stream)
        assert manifest["requires"]["qn.tablet.reader"] == ("=0.5.2" if args.located else "=0.5.1")
        item, = [a for a in manifest["artifacts"] if a["role"]=="qune.compose.Skene"]
        raw = (checkout/item["path"]).read_bytes()
        assert len(raw)==item["byte_size"] and hashlib.sha256(raw).hexdigest()==item["sha256"]
        result = subprocess.check_output([args.protoc, "--descriptor_set_in="+str(args.compose_schema),
                                          "--encode=qune.compose.Skene","compose.proto"],
                                         input=(checkout/"entry.skene.textproto").read_bytes())
        assert result==raw, f"{checkout}: checked entry bytes differ from readable source"
        entry = skene.FromString(raw)
        req, = entry.requires
        assert req.schema==method
        published = posit.FromString(subprocess.check_output(
            [args.adapter, courses/"connected-lessons/root.json",courses,courses/"connected-lessons/learning-map.toml",
             "--chapter-posit","rust-book",chapter]))
        things = {e.target.thing.SerializeToString(deterministic=True) for e in published.diff.edits}
        assert len(things)==1 and any(e.value.string_value==title for e in published.diff.edits)
        assert {p.provider.SerializeToString(deterministic=True) for p in entry.provides}==things
        assert len(entry.provides) == len(schemas), "one provision per reader operand is required"
        pins = {p.schema.full_name:p.schema.descriptor.value for p in entry.provides}
        assert pins == {name:hashlib.sha256((args.reader_release/file).read_bytes()).digest()
                        for name,file in schemas.items()}
        print(manifest["module"]["name"], title, item["sha256"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("compose-schema","reader-release","adapter","protoc"):
        parser.add_argument("--"+key,type=Path,required=True)
    parser.add_argument("--located", action="store_true", help="check the opt-in four-operand entry")
    parser.add_argument("--courses", type=Path, help="separate checkout containing the selected editions")
    verify(parser.parse_args())
