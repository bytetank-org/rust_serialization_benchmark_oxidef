#![allow(unused_imports)]
use eyre::{eyre as err, Result};
use std::{
    env,
    path::{Path, PathBuf},
};

#[cfg(feature = "regenerate-capnp")]
fn capnpc_compile_dataset(name: &'static str) -> Result<()> {
    let mut command = capnpc::CompilerCommand::new();
    #[cfg(windows)]
    command.capnp_executable("prebuilt/capnp.exe");
    command.file(format!("src/datasets/{0}/{0}.capnp", name));
    command.output_path(".");
    command.default_parent_module(vec!["datasets".into(), name.into()]);
    command.run()?;
    Ok(())
}

#[cfg(feature = "regenerate-flatbuffers")]
fn flatc_compile_dataset(name: &'static str) -> Result<()> {
    #[cfg(windows)]
    let flatc = flatc_rust::Flatc::from_path("./prebuilt/flatc.exe");
    #[cfg(not(windows))]
    let flatc = flatc_rust::Flatc::from_env_path();

    flatc.run(flatc_rust::Args {
        lang: "rust",
        inputs: &[Path::new(&format!("./src/datasets/{0}/{0}.fbs", name))],
        out_dir: Path::new(&format!("./src/datasets/{}", name)),
        extra: &["--gen-onefile"],
        ..Default::default()
    })?;
    Ok(())
}

#[cfg(feature = "regenerate-buffa")]
fn buffa_compile_dataset(name: &'static str) -> Result<()> {
    if cfg!(windows) && env::var("PROTOC").is_err() {
        env::set_var("PROTOC", "./prebuilt/protoc.exe");
    }
    buffa_build::Config::new()
        .files(&[format!("./src/datasets/{name}/{name}.proto")])
        .includes(&[format!("./src/datasets/{name}/")])
        .out_dir(format!("./src/datasets/{name}/{name}_buffa"))
        .include_file("mod.rs")
        .compile()
        .map_err(|err| err!("{err}"))?;
    Ok(())
}

#[cfg(feature = "regenerate-prost")]
fn prost_compile_dataset(name: &'static str) -> Result<()> {
    if cfg!(windows) && env::var("PROTOC").is_err() {
        env::set_var("PROTOC", "./prebuilt/protoc.exe");
    }
    let mut prost_config = prost_build::Config::new();
    prost_config.protoc_arg("--experimental_allow_proto3_optional");
    prost_config.out_dir(format!("./src/datasets/{name}"));
    prost_config.compile_protos(
        &[format!("./src/datasets/{name}/{name}.proto").as_str()],
        &["src"],
    )?;
    Ok(())
}

#[cfg(feature = "regenerate-protobuf")]
fn protobuf_compile_dataset(name: &'static str) -> Result<()> {
    if cfg!(windows) && env::var("PROTOC").is_err() {
        env::set_var("PROTOC", "./prebuilt/protoc.exe");
    }
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_extra_arg("--experimental_allow_proto3_optional")
        .out_dir(format!("./src/datasets/{name}/{name}_protobuf"))
        .inputs(&[format!("./src/datasets/{name}/{name}.proto")])
        .include(format!("./src/datasets/{name}/"))
        .run()
        .map_err(|err| err!(err.into_boxed_dyn_error()))?;
    Ok(())
}

#[cfg(feature = "regenerate-protobuf4")]
fn protobuf4_compile_dataset(name: &'static str) -> Result<()> {
    if cfg!(windows) && env::var("PROTOC").is_err() {
        env::set_var("PROTOC", "./prebuilt/protoc.exe");
    }
    protobuf4_codegen::CodeGen::new()
        .include(format!("./src/datasets/{name}"))
        .inputs([format!("{name}.proto")])
        .output_dir(format!("./protobuf4-generated/src/{name}"))
        .generate_and_compile()
        .map_err(eyre::Report::msg)?;
    Ok(())
}

#[cfg(any(feature = "oxidef", feature = "oxidef_old"))]
fn oxidef_dataset_files(names: &[&str]) -> Vec<String> {
    // Each dataset has a schema using `final` types everywhere and an equivalent schema using
    // `extensible` types everywhere, so the overhead of extensibility can be compared.
    names
        .iter()
        .flat_map(|name| {
            [
                format!("src/datasets/{name}/{name}.oxidef"),
                format!("src/datasets/{name}/{name}_extensible.oxidef"),
            ]
        })
        .collect()
}

#[cfg(feature = "oxidef")]
fn oxidef_compile_datasets(names: &[&str]) -> Result<()> {
    oxidef::compile_files(oxidef_dataset_files(names))
        .compact1()
        .validation()
        .finish_rust("oxidef")
        .map_err(|err| err!("{err:?}"))?;
    Ok(())
}

#[cfg(feature = "oxidef_old")]
fn oxidef_old_compile_datasets(names: &[&str]) -> Result<()> {
    oxidef_old::compile_files(oxidef_dataset_files(names))
        .compact1()
        .validation()
        .finish_rust("oxidef_old")
        .map_err(|err| err!("{err:?}"))?;

    // The generated code refers to the runtime crates by their usual names, so point it at the
    // renamed `_old` copies instead.
    let out_dir = PathBuf::from(env::var("OUT_DIR")?).join("oxidef_old");
    for entry in std::fs::read_dir(&out_dir)? {
        let path = entry?.path();
        let code = std::fs::read_to_string(&path)?
            .replace("::oxidef_compact1::", "::oxidef_compact1_old::")
            .replace("::oxidef_validation::", "::oxidef_validation_old::");
        std::fs::write(&path, code)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    #[cfg(feature = "oxidef")]
    oxidef_compile_datasets(&["log", "mesh", "minecraft_savedata", "mk48"])?;
    #[cfg(feature = "oxidef_old")]
    oxidef_old_compile_datasets(&["log", "mesh", "minecraft_savedata", "mk48"])?;

    #[cfg(any(
        feature = "regenerate-buffa",
        feature = "regenerate-capnp",
        feature = "regenerate-flatbuffers",
        feature = "regenerate-prost",
        feature = "regenerate-protobuf",
        feature = "regenerate-protobuf4",
    ))]
    {
        const DATASETS: &[&str] = &["log", "mesh", "minecraft_savedata", "mk48"];
        for &name in DATASETS.iter() {
            #[cfg(feature = "regenerate-buffa")]
            buffa_compile_dataset(name)?;
            #[cfg(feature = "regenerate-capnp")]
            capnpc_compile_dataset(name)?;
            #[cfg(feature = "regenerate-flatbuffers")]
            flatc_compile_dataset(name)?;
            #[cfg(feature = "regenerate-prost")]
            prost_compile_dataset(name)?;
            #[cfg(feature = "regenerate-protobuf")]
            protobuf_compile_dataset(name)?;
            #[cfg(feature = "regenerate-protobuf4")]
            protobuf4_compile_dataset(name)?;
        }
    }
    Ok(())
}
