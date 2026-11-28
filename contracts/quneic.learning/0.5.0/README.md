# Learning 0.5.0 descriptor release

The Learning Qrate owns the Edition 2023 schemas `quneic/learning/v1/{learning,courseware}.proto`. This data-only release contains the transitive FileDescriptorSet for both files, including Qune core, CEL, Buf validation and protobuf imports. The public Qrate identity is `quneic.learning.contract@0.5.0`. An independent consumer can resolve the release directory, verify its exact SHA-256 from `release.toml`, and generate its own private binding. The owner implementation and private checkout are unnecessary.

To reproduce the release, supply the owner proto directory, the pinned Qune 0.5 spec directory and the protoc 3.3.0 vendored package’s compiler and include directory. Run from this release directory.

```sh
"$PROTOC" -I"$LEARNING_PROTO" -I"$QUNE_SPEC" -I"$QUNE_SPEC/vendor" -I"$PROTOC_INCLUDE" \
  --include_imports --descriptor_set_out=/tmp/learning.fds \
  quneic/learning/v1/learning.proto quneic/learning/v1/courseware.proto
cmp /tmp/learning.fds learning.fds
```

The adapter grants read access to the release manifest and descriptor as declared foreign file inputs. Its consumer pin is `adapters/contract.toml`. A plain Cargo checkout finds the release in this repository. A caller can supply a separately obtained read-only release directory through `QUNE_LEARNING_SCHEMA`. The build verifies owner, version and exact digest before generating bindings in Cargo's output directory.

From the public checkout root the importer reads the connected lesson through the same pinned source catalog used by the validator.

```sh
root=courses/connected-lessons/root.json
map=courses/connected-lessons/learning-map.toml
cargo run --locked -q --bin validate-learning-map -- "$root" courses "$map" --map-wire > /tmp/map.pb
```

Use `--root-wire` for LearningRoot, `--adapted-course ALIAS` for AdaptedCourse and `--posit-wire` for the identified core Posit carrying the placements field edit. Binary stdout is unframed. Existing `--wire` emits protobuf JSON. Importing prepares the Posit. The runtime is responsible for admission and the resulting cut. AdaptedCourse currently identifies passages and exercise destinations; source-level origins are optional and unpopulated in this importer.
