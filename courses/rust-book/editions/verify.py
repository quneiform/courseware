"""Verify the candidate's 120 SourceAnchors against the selected CEL Git revision."""
import argparse
import hashlib
import subprocess
from pathlib import Path

from google.protobuf import descriptor_pb2, descriptor_pool, message_factory


def verify(adapter, courses, descriptor):
    wire = subprocess.check_output([
        adapter, courses / "connected-lessons/root.json", courses,
        courses / "connected-lessons/learning-map.toml", "--adapted-course", "rust-book"
    ])
    pool = descriptor_pool.DescriptorPool()
    for file in descriptor_pb2.FileDescriptorSet.FromString(descriptor.read_bytes()).file:
        pool.Add(file)
    cls = message_factory.GetMessageClass(pool.FindMessageTypeByName("quneic.learning.v1.AdaptedCourse"))
    course = cls.FromString(wire)
    paths = set()
    for passage in course.passages:
        origin, = passage.origins
        path = origin.path
        assert path.startswith("src/") and path.endswith(".md") and path not in paths, path
        paths.add(path)
        original = subprocess.check_output(["git", "-C", courses / "rust-book/upstream", "show",
                                            course.basis.source.commit + ":" + path])
        assert origin.file_digest.value == hashlib.sha256(original).digest(), path
    assert len(paths) == 120
    print(f"{len(paths)} unique Markdown origins verified at {course.basis.source.commit}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("adapter", type=Path)
    parser.add_argument("courses", type=Path)
    parser.add_argument("learning_descriptor", type=Path)
    options = parser.parse_args()
    verify(options.adapter, options.courses, options.learning_descriptor)
