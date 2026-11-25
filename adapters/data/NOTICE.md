# Rust Book permission examples

The permission examples in `permissions.toml` are adapted from the Cognitive Engineering Lab’s experimental edition of *The Rust Programming Language*. They include lines from Rust examples and permission transitions recorded by Aquascope. The extraction script converts those transitions into short descriptions and omits hidden source lines.

The pinned source and the deployment that produced these examples are [CEL source 88250e0](https://github.com/cognitive-engineering-lab/rust-book/tree/88250e0392cef0622f318e35469108d68694c1f7) and [published edition 967294b](https://github.com/cognitive-engineering-lab/rust-book/tree/967294b5d47e077a5208a5dece56bd3bb29d37db). CEL’s repository supplies its [copyright statement](licenses/COPYRIGHT) and the [MIT](licenses/LICENSE-MIT) and [Apache 2.0](licenses/LICENSE-APACHE) licenses. The copies here were taken from the pinned source. The MIT notice credits The Rust Project Developers.

To regenerate the file, pass a clone of the public CEL repository containing its deployment history to `python3 adapters/tools/freeze_permissions.py`. The script checks that the deployment identifies the pinned source revision and emits the TOML on standard output. Its SHA-256 is `96a6f5ab2e1bfabda7b8c75922035fe1df0831dc74d2ff3d93502551673a4d30`.
