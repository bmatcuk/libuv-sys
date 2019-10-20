use std::env;
use std::error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{create_dir_all, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use which::which;

static LIBUV_VERSION: &str = "v1.30";
static LIBUV_REPOSITORY: &str = "https://github.com/libuv/libuv.git";
static LIBUV_SOURCE_PATH: &str = "libuv.source";

static GYP_REPOSITORY: &str = "https://chromium.googlesource.com/external/gyp";

#[derive(Debug)]
enum Error {
    BindgenError,
    CommandError(String, io::Error),
    CommandFailure(String, Output),
    PathError(String, io::Error),
    VersionNotFound,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::CommandError(_, err) => Some(err),
            Error::PathError(_, err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BindgenError => write!(f, "Failed to generate ffi bindings"),
            Error::CommandError(cmd, source) => write!(f, "Failed to run `{}`: {}", cmd, source),
            Error::CommandFailure(cmd, out) => {
                let stdout = std::str::from_utf8(&out.stdout).unwrap();
                let stderr = std::str::from_utf8(&out.stderr).unwrap();
                write!(f, "`{}` did not exit successfully: {}", cmd, out.status)?;
                if !stdout.is_empty() {
                    write!(f, "\n--- stdout\n{}", stdout)?;
                }
                if !stderr.is_empty() {
                    write!(f, "\n--- stderr\n{}", stderr)?;
                }
                Ok(())
            }
            Error::PathError(dir, source) => write!(f, "Path error for `{}`: {}", dir, source),
            Error::VersionNotFound => write!(f, "Could not find libuv {}", LIBUV_VERSION),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

fn build_pkgconfig_max_version() -> String {
    let dotidx = LIBUV_VERSION.find('.').unwrap();
    let next_minor_version = LIBUV_VERSION[(dotidx + 1)..].parse::<usize>().unwrap() + 1;
    format!("{}.{}", &LIBUV_VERSION[1..dotidx], next_minor_version)
}

fn envify(name: &str) -> String {
    name.chars()
        .map(|c| match c.to_ascii_uppercase() {
            '-' => '_',
            c => c,
        })
        .collect()
}

fn find_executable(name: &str) -> Option<PathBuf> {
    env::var_os(format!("{}_PATH", envify(name)))
        .map(|p| p.into())
        .or_else(|| which(name).ok())
}

fn run(mut cmd: Command) -> Result<Vec<u8>> {
    println!("Running {:?}...", cmd);

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(output.stdout)
            } else {
                Err(Error::CommandFailure(format!("{:?}", cmd), output))
            }
        }
        Err(cause) => Err(Error::CommandError(format!("{:?}", cmd), cause)),
    }
}

fn build_command<S: AsRef<OsStr>, P: AsRef<Path>>(pwd: &P, cmd: &S, args: &[&str]) -> Command {
    let mut cmd = Command::new(cmd);
    cmd.args(args).current_dir(pwd);
    cmd
}

fn quick_run<S: AsRef<OsStr>, P: AsRef<Path>>(pwd: &P, cmd: &S, args: &[&str]) -> Result<Vec<u8>> {
    run(build_command(pwd, cmd, args))
}

fn mkdirp<P: AsRef<Path>>(path: &P) -> Result<()> {
    create_dir_all(path).map_err(|e| Error::PathError(path.as_ref().to_string_lossy().into(), e))
}

fn touch<P: AsRef<Path>>(path: &P) -> Result<()> {
    match OpenOptions::new()
        .create(true)
        .write(true)
        .open(path.as_ref())
    {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::PathError(path.as_ref().to_string_lossy().into(), e)),
    }
}

fn force_redownload() -> bool {
    env::var_os("LIBUV_REDOWNLOAD").is_some()
}

fn download_libuv<S: AsRef<OsStr>>(git: &S) -> Result<(PathBuf, bool)> {
    println!("Downloading libuv...");

    let outdir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let source_path = outdir.join(LIBUV_SOURCE_PATH);
    let gitdir = source_path.join(".git");
    if Path::new(&gitdir).exists() {
        if force_redownload() {
            // fetch latest and reset hard
            quick_run(&source_path, &git, &["fetch", "origin"])?;
            quick_run(&source_path, &git, &["reset", "--hard", "FETCH_HEAD"])?;
        } else {
            println!("- source already downloaded; skipping...");
            return Ok((source_path, false));
        }
    } else {
        // download
        quick_run(
            &outdir,
            &git,
            &["clone", LIBUV_REPOSITORY, LIBUV_SOURCE_PATH],
        )?;
    }

    // find the tag for the version
    let tags = quick_run(
        &source_path,
        &git,
        &[
            "tag",
            "-l",
            &format!("{}.*", LIBUV_VERSION),
            "--sort",
            "-version:refname",
        ],
    )?;
    let tag = match std::str::from_utf8(&tags).unwrap().lines().nth(0) {
        Some(tag) => tag.trim(),
        None => return Err(Error::VersionNotFound),
    };

    // checkout the version tag
    println!("- checking out {}...", tag);
    quick_run(&source_path, &git, &["checkout", tag])?;

    Ok((source_path, true))
}

fn force_rebuild() -> bool {
    env::var_os("LIBUV_REBUILD").is_some()
}

fn build_with_cmake<S: AsRef<OsStr>>(cmake: &S, force: bool) -> Result<bool> {
    println!("Building libuv with cmake...");

    // make build directory
    let force = force || force_rebuild();
    let outdir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let build_path = outdir.join(LIBUV_SOURCE_PATH).join("out").join("cmake");
    let library_path = if cfg!(windows) {
        build_path.join("Release")
    } else {
        build_path.clone()
    };
    println!(
        "cargo:rustc-link-search=native={}",
        library_path.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=static=uv_a");

    let library_output = if cfg!(windows) {
        library_path.join("uv_a.lib")
    } else {
        library_path.join("libuv_a.a")
    };
    if library_output.exists() && !force {
        println!("- build exists; skipping...");
        return Ok(false);
    }

    println!("- making output directory {:?}...", build_path);
    mkdirp(&build_path)?;

    // configure cmake
    let up2 = Path::new("..").join("..");
    println!("- configure...");
    quick_run(
        &build_path,
        &cmake,
        &[
            up2.to_string_lossy().as_ref(),
            "-DBUILD_TESTING=OFF",
            "-DCMAKE_BUILD_TYPE=Release",
        ],
    )?;

    // build
    println!("- build...");
    quick_run(
        &build_path,
        &cmake,
        &["--build", ".", "--config", "Release"],
    )?;

    Ok(true)
}

fn build_with_gyp<S: AsRef<OsStr>>(git: &S, force: bool) -> Result<bool> {
    println!("Building libuv with gyp...");

    let outdir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let source_path = outdir.join(LIBUV_SOURCE_PATH);
    let force = force || force_rebuild();

    // build, unfortunately, depends on the os =(
    if cfg!(windows) {
        let lib_path = source_path.join("Release").join("lib");
        println!(
            "cargo:rustc-link-search=native={}",
            lib_path.to_string_lossy()
        );
        println!("cargo:rustc-link-lib=static=uv");
        // TODO: what's the name of the static library?
        if lib_path.join("libuv.lib").exists() && !force {
            println!("- build exists; skipping...");
            return Ok(false);
        }

        println!("- running vcbuild...");
        quick_run(&source_path, &source_path.join("vcbuild.bat"), &[])?;

        return Ok(true);
    }

    // install gyp
    let gyp_path = source_path.join("build").join("gyp");
    if !gyp_path.exists() {
        println!("- install gyp...");
        quick_run(
            &outdir,
            &git,
            &["clone", GYP_REPOSITORY, gyp_path.to_string_lossy().as_ref()],
        )?;
    }

    // build... TODO: android?
    if cfg!(target_os = "macos") {
        let lib_path = source_path.join("build").join("Release");
        println!(
            "cargo:rustc-link-search=native={}",
            lib_path.to_string_lossy()
        );
        println!("cargo:rustc-link-lib=static=uv");
        if lib_path.join("libuv.a").exists() && !force {
            println!("- build exists; skipping...");
            return Ok(false);
        }

        let xcodebuild = find_executable("xcodebuild").expect("`xcodebuild` is required");
        let target = env::var("TARGET").unwrap();
        let arch_idx = target.find('-').unwrap();
        println!("- building xcodeproj...");
        quick_run(
            &source_path,
            &source_path.join("gyp_uv.py"),
            &["-f", "xcode"],
        )?;

        println!("- build...");
        quick_run(
            &source_path,
            &xcodebuild,
            &[
                &format!("-ARCHS={}", &target[..arch_idx]),
                "-project",
                "out/uv.xcodeproj",
                "-configuration",
                "Release",
                "-alltargets",
            ],
        )?;
    } else {
        let lib_path = source_path.join("out").join("Release");
        println!(
            "cargo:rustc-link-search=native={}",
            lib_path.to_string_lossy()
        );
        println!("cargo:rustc-link-lib=static=uv");
        if lib_path.join("libuv.a").exists() && !force {
            println!("- build exists; skipping...");
            return Ok(false);
        }

        println!("- building Makefile...");
        quick_run(
            &source_path,
            &source_path.join("gyp_uv.py"),
            &["-f", "make"],
        )?;

        let make = find_executable("make").expect("`make` is required");
        let mut build_cmd = build_command(&source_path, &make, &["-C", "out"]);
        build_cmd.env("BUILDTYPE", "Release");
        println!("- build...");
        run(build_cmd)?;
    }

    Ok(true)
}

fn build_with_autotools<S: AsRef<OsStr>>(sh: &S, force: bool) -> Result<bool> {
    println!("Building libuv with autotools...");

    let force = force || force_rebuild();
    let outdir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let source_path = outdir.join(LIBUV_SOURCE_PATH);
    let lib_path = source_path.join(".libs");
    println!(
        "cargo:rustc-link-search=native={}",
        lib_path.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=static=uv");
    if lib_path.join("libuv.a").exists() && !force {
        println!("- build exists; skipping...");
        return Ok(false);
    }

    // run autogen
    println!("- autogen...");
    quick_run(&source_path, &sh, &["autogen.sh"])?;

    // configure
    println!("- configure...");
    quick_run(&source_path, &source_path.join("configure"), &[])?;

    // make
    let make = find_executable("make").expect("`make` is required");
    println!("- build...");
    quick_run(&source_path, &make, &[])?;

    Ok(true)
}

fn force_regenerate_bindings() -> bool {
    env::var_os("LIBUV_REGENERATE_BINDINGS").is_some()
}

fn generate_bindings<P: AsRef<Path>>(header_path: &P, force: bool) -> Result<()> {
    println!("Generating bindings for libuv...");

    let force = force || force_regenerate_bindings();
    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    if outdir.exists() && !force {
        println!("- bindings exist; skipping...");
        return Ok(());
    }

    // bindgen needs the path as a String
    let include_path = header_path.as_ref().parent().unwrap().to_string_lossy();
    let header_path = header_path.as_ref().to_string_lossy();

    // generate ffi bindings
    let bindings = bindgen::Builder::default()
        .header(header_path)
        .clang_arg(format!("-I{}", include_path))
        .whitelist_type("uv_.+")
        .whitelist_function("uv_.+")
        .whitelist_var("uv_.+")
        .generate()
        .map_err(|_| Error::BindgenError)?;

    // write to file
    bindings
        .write_to_file(&outdir)
        .map_err(|e| Error::PathError(outdir.to_string_lossy().into(), e))?;

    Ok(())
}

fn main() {
    println!("cargo:rerun-if-env-changed=LIBUV_REDOWNLOAD");
    println!("cargo:rerun-if-env-changed=LIBUV_REBUILD");
    println!("cargo:rerun-if-env-changed=LIBUV_REGENERATE_BINDINGS");

    // If we find libuv with pkg-config, we just need bindings... if there are _any_ errors, just
    // move on to downloading. Either we don't have pkg-config, or we don't have libuv.
    let max_version = build_pkgconfig_max_version();
    let pkgconfig_result = pkg_config::Config::new()
        .range_version(&LIBUV_VERSION[1..]..max_version.as_ref())
        .env_metadata(true)
        .probe("libuv");
    if let Ok(libuv) = pkgconfig_result {
        println!("Resolving libuv with pkg-config");
        let bindings_sentinal = PathBuf::from(env::var("OUT_DIR").unwrap()).join(format!(
            "pkgconfig-{}",
            libuv.version.lines().nth(0).unwrap()
        ));
        for include_path in libuv.include_paths {
            let header_path = include_path.join("uv.h");
            if header_path.exists() {
                generate_bindings(&header_path, !bindings_sentinal.exists()).unwrap();
                touch(&bindings_sentinal).unwrap();
                return;
            }
        }
        panic!("Could not find `uv.h` from pkg-config");
    }

    // We need git to download libuv
    let git = find_executable("git").expect("`git` is required");
    let (source_path, downloaded) = download_libuv(&git).unwrap();

    // libuv has a couple build systems... cmake seems to be the most straightforward, so we'll try
    // that first... Next up would be gyp which will require python, followed by good ol'
    // autotools. Autotools, of courses, won't work on Windows.
    let cmake = find_executable("cmake");
    let python = find_executable("python");
    let sh = find_executable("sh");
    match (cmake, python, sh) {
        (Some(ref cmake), _, _) => build_with_cmake(&cmake, downloaded).unwrap(),
        (_, Some(_), _) => build_with_gyp(&git, downloaded).unwrap(),
        (_, _, Some(ref sh)) => build_with_autotools(&sh, downloaded).unwrap(),
        _ => panic!("Requires either cmake, python2 (for gyp) or sh and the autotools"),
    };

    let header_path = source_path.join("include").join("uv.h");
    generate_bindings(&header_path, downloaded).unwrap();
}
