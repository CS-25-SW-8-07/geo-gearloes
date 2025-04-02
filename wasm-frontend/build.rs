use std::{
    env,
    io::{Write, stderr},
    os::unix::process::ExitStatusExt,
    path::Path,
    process::Command,
};

fn main() -> std::io::Result<()> {
    println!("{:?}", std::env::current_dir());
    println!("cargo::rerun-if-changed=build.rs");
    recursive_rerun_if_changed("websm")?;
    let out_dir = env::var("OUT_DIR").unwrap();
    copy_webywasm("./websm/webywasm/", format!("{out_dir}/webywasm"))?;
    build(out_dir.as_str());

    Ok(())
}

fn recursive_rerun_if_changed(src: impl AsRef<Path>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_str().unwrap();
        if name == "node_modules" || name == "target" {
            continue;
        };

        if entry.file_type()?.is_dir() {
            recursive_rerun_if_changed(entry.path())?;
        } else {
            println!("cargo::rerun-if-changed={}", entry.path().to_str().unwrap());
        }
    }

    Ok(())
}

// NOTE: https://stackoverflow.com/questions/26958489/how-to-copy-a-folder-recursively-in-rust
fn copy_webywasm(src: impl AsRef<Path>, dist: impl AsRef<Path>) -> std::io::Result<()> {
    println!("Copying webywasm");
    dbg!(&dist.as_ref());
    std::fs::create_dir_all(&dist)?;
    for entry in std::fs::read_dir(&src)? {
        let entry = entry?;
        if entry.file_name().to_str().unwrap() == "node_modules" {
            continue;
        };

        if entry.file_type()?.is_dir() {
            copy_webywasm(entry.path(), dist.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dist.as_ref().join(entry.file_name()))?;
            println!("Cake");
        }
    }

    Ok(())
}

fn build(out_dir: &str) {
    println!("Building");

    std::env::set_current_dir(format!("./websm/rustywasm/")).unwrap();

    let res = Command::new("wasm-pack")
        .env("CARGO_TARGET_DIR", format!("{out_dir}/target"))
        .args([
            "build",
            #[cfg(debug_assertions)]
            {
                "--dev"
            },
            "--out-dir",
            format!("{out_dir}/rustywasm/pkg").as_str(),
            "--out-name",
            "rustywasm",
        ])
        .output()
        .expect("Failed to run wasm-pack, is it installed?");

    if res.status != ExitStatusExt::from_raw(0) {
        let s = String::from_utf8(res.stderr).unwrap();
        panic!("{s}");
    }

    std::env::set_current_dir(format!("{out_dir}/webywasm")).unwrap();

    let res = Command::new("yarn")
        .output()
        .expect("Failed to install yarn dependencies");

    if res.status != ExitStatusExt::from_raw(0) {
        let s = String::from_utf8(res.stderr).unwrap();
        panic!("{s}");
    }

    let res = Command::new("yarn")
        .arg("build")
        .output()
        .expect("Failed to run yarn");

    if res.status != ExitStatusExt::from_raw(0) {
        let s = String::from_utf8(res.stderr).unwrap();
        panic!("{s}");
    }
}
