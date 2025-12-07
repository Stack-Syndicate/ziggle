use colored::Colorize;
use inquire::Text;
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use toml_edit::{Array, DocumentMut, value};

fn main() -> anyhow::Result<()> {
    let directory_str = Text::new("Project directory:").with_default(".").prompt()?;
    let directory: PathBuf = if directory_str == "." {
        std::env::current_dir()
            .expect("Failed to get current directory.")
            .clone()
    } else {
        Path::new(&*directory_str).to_path_buf()
    };
    let project_name = Text::new("Project name:")
        .with_default(
            directory
                .file_name()
                .expect("Invalid directory name.")
                .to_str()
                .unwrap(),
        )
        .prompt()?;
    if !directory.exists() {
        println!(
            "{} {}",
            "!".red(),
            format_args!(
                "Directory {} does not exist.",
                directory.to_string_lossy().red()
            )
        );
        print!(
            "{} {}",
            "$".yellow(),
            format_args!("Creating directory {}", "-> ".blue())
        );
        Command::new("mkdir").arg(&directory).output()?;
        println!("{}", "done.".bright_green());
    }
    print!(
        "{} {}",
        "$".yellow(),
        format_args!("Initializing zig project {}", "-> ".blue())
    );
    std::env::set_current_dir(&directory)?;
    Command::new("zig").arg("init").output()?;
    Command::new("mv").arg("src").arg("src-zig").output()?;
    let mut build_zig = Regex::new(r"//.*")
        .unwrap()
        .replace_all(&std::fs::read_to_string("build.zig").unwrap(), "")
        .to_string();
    build_zig = Regex::new(r"\n\s*\n+")
        .unwrap()
        .replace_all(&build_zig, "\n")
        .to_string();
    fs::write("build.zig", build_zig)?;
    println!("{}", "done.".bright_green());
    print!(
        "{} {}",
        "$".yellow(),
        format_args!("Initializing rust project {}", "-> ".blue())
    );
    Command::new("cargo").arg("init").arg("--lib").output()?;
    Command::new("cargo")
        .arg("add")
        .arg("cbindgen")
        .arg("--build")
        .output()?;
    println!("{}", "done.".bright_green());
    print!(
        "{} {}",
        "$".yellow(),
        format_args!("Creating cbindgen build.rs file{}", " -> ".blue())
    );
    let build_rs = format!(
        r#"
        use cbindgen::Language;
        use std::env;

        fn main() {{
            println!("cargo:rerun-if-changed=src/lib.rs");
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            cbindgen::Builder::new()
                .with_crate(manifest_dir)
                .with_language(Language::C)
                .generate()
                .expect("Unable to generate C bindings.")
                .write_to_file("target/headers/{}.h");
        }}"#,
        project_name
    );
    fs::write("build.rs", build_rs).expect("Unable to write build.rs");
    Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg("build.rs")
        .output()?;
    println!("{}", "done.".bright_green());
    print!(
        "{} {}",
        "$".yellow(),
        format_args!(
            "Setting rust crate-type to cdylib + staticlib{}",
            " -> ".blue()
        )
    );
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Unable to read from Cargo.toml");
    let mut cargo_toml_parsed = cargo_toml
        .parse::<DocumentMut>()
        .expect("Could not parse Cargo.toml");
    let mut cargo_toml_lib_array = Array::default();
    cargo_toml_lib_array.push("cdylib");
    cargo_toml_lib_array.push("rlib");
    cargo_toml_lib_array.push("staticlib");
    cargo_toml_parsed["lib"]["crate-type"] = value(cargo_toml_lib_array);
    fs::write("Cargo.toml", cargo_toml_parsed.to_string())?;
    println!("{}", "done.".bright_green());
    let build_zig_rust = format!(
        r#"
        const rust_build = b.addSystemCommand(&[_][]const u8{{ "cargo", "build", "--release" }});
        exe.linkLibC();
        exe.addLibraryPath(b.path("target/release/"));
        exe.addIncludePath(b.path("target/headers/"));
        exe.linkSystemLibrary("{}");
        exe.step.dependOn(&rust_build.step);
        b.installArtifact(exe);"#,
        project_name
    );
    let mut build_zig = fs::read_to_string("build.zig").expect("Unable to read from build.zig");
    build_zig = build_zig.replace("b.installArtifact(exe);", &build_zig_rust);
    build_zig = build_zig.replace("src/root.zig", "src-zig/root.zig");
    build_zig = build_zig.replace("src/main.zig", "src-zig/main.zig");
    fs::write("build.zig", build_zig).expect("Unable to write new build.zig");
    Command::new("zig")
        .arg("build")
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()?;
    println!("{}", "!! PROJECT INITIALISED !!".bright_magenta(),);
    Ok(())
}
