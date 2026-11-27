use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};
fn main() {
    let tablet = PathBuf::from("../contracts/qn.tablet.reader/0.5.0/tablet.fds");
    for (descriptor, file, include) in [
        (tablet.clone(), "tablet/tablet.proto", "tablet.rs"),
        (
            PathBuf::from("schema/courseware.fds"),
            "courseware.proto",
            "courseware_types.rs",
        ),
    ] {
        println!("cargo:rerun-if-changed={}", descriptor.display());
        buffa_build::Config::new()
            .descriptor_set(&descriptor)
            .files(&[file])
            .include_file(include)
            .generate_views(false)
            .preserve_unknown_fields(false)
            .compile()
            .expect("generate private reading bindings");
    }
    let archive = "schema/archive.fds";
    println!("cargo:rerun-if-changed={archive}");
    buffa_build::Config::new()
        .descriptor_set(archive)
        .files(&["courseware.proto", "archive.proto"])
        .preserve_unknown_fields(false)
        .generate_views(false)
        .include_file("archive_wire.rs")
        .compile()
        .expect("generate archive session contract");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("tablet-digest"),
        Sha256::digest(fs::read(tablet).unwrap()),
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=QUNE_LEARNING_SCHEMA");
    println!("cargo:rerun-if-changed=contract.toml");
    let owner = env::var_os("QUNE_LEARNING_SCHEMA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../contracts/quneic.learning/0.5.0"));
    let release_path = owner.join("release.toml");
    let descriptor = owner.join("learning.fds");
    println!("cargo:rerun-if-changed={}", release_path.display());
    println!("cargo:rerun-if-changed={}", descriptor.display());
    let pin: toml::Table = toml::from_str(include_str!("contract.toml")).unwrap();
    let release: toml::Table =
        toml::from_str(&fs::read_to_string(&release_path).expect("read Learning schema release"))
            .unwrap();
    for (index, key) in ["name", "version", "sha256"].into_iter().enumerate() {
        assert_eq!(
            pin.get(key),
            release.get(key),
            "contract.toml:{}:1: incompatible Learning schema {key}",
            index + 1
        );
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&descriptor).unwrap())),
        pin["sha256"].as_str().unwrap(),
        "contract.toml:3:1: incompatible Learning schema digest"
    );
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("schema-digest"),
        Sha256::digest(fs::read(&descriptor).unwrap()),
    )
    .unwrap();
    buffa_build::Config::new()
        .descriptor_set(&descriptor)
        .files(&[
            "core.proto",
            "cel/expr/checked.proto",
            "cel/expr/syntax.proto",
            "cel/expr/value.proto",
            "buf/validate/validate.proto",
            "quneic/learning/v1/learning.proto",
            "quneic/learning/v1/courseware.proto",
        ])
        .include_file("schema.rs")
        .generate_views(false)
        .generate_json(true)
        .preserve_unknown_fields(false)
        .map_type_in(
            buffa_build::MapRepr::BTreeMap,
            &[".quneic.learning.v1.LearningRoot"],
        )
        .extern_path(
            ".google.protobuf.FieldDescriptorProto",
            "::buffa_descriptor::generated::descriptor::FieldDescriptorProto",
        )
        .compile()
        .expect("Learning descriptor bindings");
}
