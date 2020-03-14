use std::env;
use std::error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

static LIBUV_VERSION: &str = "1.35.0";

#[derive(Debug)]
enum Error {
    BindgenError,
    PathError(String, io::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::PathError(_, err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BindgenError => write!(f, "Failed to generate ffi bindings"),
            Error::PathError(dir, source) => write!(f, "Path error for `{}`: {}", dir, source),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

fn build_pkgconfig_max_version() -> String {
    let dotidx = LIBUV_VERSION.find('.').unwrap();
    let dotidx2 = LIBUV_VERSION[(dotidx + 1)..].find('.').unwrap() + dotidx + 1;
    let next_minor_version = LIBUV_VERSION[(dotidx + 1)..dotidx2].parse::<usize>().unwrap() + 1;
    format!("{}.{}.0", &LIBUV_VERSION[..dotidx], next_minor_version)
}

fn try_pkgconfig() -> Option<Option<PathBuf>> {
    // can't use pkg-config for cross-compile
    if env::var("TARGET") != env::var("HOST") {
        return None;
    }

    // If we find libuv with pkg-config, we just need bindings... if there are _any_ errors, just
    // move on to building. Either we don't have pkg-config, or we don't have libuv.
    let max_version = build_pkgconfig_max_version();
    let pkgconfig_result = pkg_config::Config::new()
        .range_version(&LIBUV_VERSION[..]..max_version.as_ref())
        .env_metadata(true)
        .probe("libuv");
    if let Ok(libuv) = pkgconfig_result {
        println!("Resolving libuv with pkg-config");
        for include_path in libuv.include_paths {
            let header_path = include_path.join("uv.h");
            if header_path.exists() {
                return Some(Some(include_path));
            }
        }

        // we couldn't find uv.h, but we have a local copy anyway so it's not necessarily an
        // error...
        return Some(None);
    }
    return None
}

#[cfg(windows)]
fn windows_version_specific_settings(build: &mut cc::Build) {
    match nt_version::get() {
        (6, 0, _) => {
            // Vista
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=psapi");
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=user32");
            build.define("_WIN32_WINNT", "0x0600")
        },
        (6, 1, _) => build.define("_WIN32_WINNT", "0x0601"),    // Windows 7
        (6, 2, _) => build.define("_WIN32_WINNT", "0x0602"),    // Windows 8
        (6, 3, _) => build.define("_WIN32_WINNT", "0x0603"),    // Windows 8.1
        (10, _, _) => build.define("_WIN32_WINNT", "0x0A00"),   // Windows 10
        _ => panic!("This version of Windows is unsupported by libuv.")
    };
}

#[cfg(not(windows))]
fn windows_version_specific_settings(_build: &mut cc::Build) {}

fn build<P: AsRef<Path>>(source_path: &P) -> Result<()> {
    let src_path = source_path.as_ref().join("src");
    let unix_path = src_path.join("unix");

    let target = env::var("TARGET").unwrap();
    let android = target.ends_with("-android") || target.ends_with("-androideabi");
    let apple = target.contains("-apple-");
    let dragonfly = target.ends_with("-dragonfly");
    let freebsd = target.ends_with("-freebsd");
    let linux = target.contains("-linux-");
    let netbsd = target.ends_with("-netbsd");
    let openbsd = target.ends_with("-openbsd");
    let solaris = target.ends_with("-solaris");

    // based on libuv's CMakeLists.txt
    let mut build = cc::Build::new();
    let compiler = build.get_compiler();
    let clang = compiler.is_like_clang();
    let gnu = compiler.is_like_gnu();
    let msvc = compiler.is_like_msvc();
    build
        .include(source_path.as_ref().join("include"))
        .include(&src_path);

    if msvc {
        build.flag("/W4");
    } else if apple || clang || gnu {
        build
            .flag("-fvisibility=hidden")
            .flag("--std=gnu89")
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Wstrict-prototypes")
            .flag("-Wno-unused-parameter");
    }

    build
        .file(src_path.join("fs-poll.c"))
        .file(src_path.join("idna.c"))
        .file(src_path.join("inet.c"))
        .file(src_path.join("random.c"))
        .file(src_path.join("strscpy.c"))
        .file(src_path.join("threadpool.c"))
        .file(src_path.join("timer.c"))
        .file(src_path.join("uv-common.c"))
        .file(src_path.join("uv-data-getter-setters.c"))
        .file(src_path.join("version.c"));

    if cfg!(windows) {
        windows_version_specific_settings(&mut build);

        println!("cargo:rustc-link-lib=iphlpapi");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=userenv");
        println!("cargo:rustc-link-lib=ws2_32");

        let win_path = src_path.join("win");
        build
            .define("WIN32_LEAN_AND_MEAN", None)
            .file(win_path.join("async.c"))
            .file(win_path.join("core.c"))
            .file(win_path.join("detect-wakeup.c"))
            .file(win_path.join("dl.c"))
            .file(win_path.join("error.c"))
            .file(win_path.join("fs.c"))
            .file(win_path.join("fs-event.c"))
            .file(win_path.join("getaddrinfo.c"))
            .file(win_path.join("getnameinfo.c"))
            .file(win_path.join("handle.c"))
            .file(win_path.join("loop-watcher.c"))
            .file(win_path.join("pipe.c"))
            .file(win_path.join("thread.c"))
            .file(win_path.join("poll.c"))
            .file(win_path.join("process.c"))
            .file(win_path.join("process-stdio.c"))
            .file(win_path.join("signal.c"))
            .file(win_path.join("snprintf.c"))
            .file(win_path.join("stream.c"))
            .file(win_path.join("tcp.c"))
            .file(win_path.join("tty.c"))
            .file(win_path.join("udp.c"))
            .file(win_path.join("util.c"))
            .file(win_path.join("winapi.c"))
            .file(win_path.join("winsock.c"));
    } else {
        if !android {
            println!("cargo:rustc-link-lib=pthread");
        }

        build
            .define("_FILE_OFFSET_BITS", "64")
            .define("_LARGEFILE_SOURCE", None)
            .file(unix_path.join("async.c"))
            .file(unix_path.join("core.c"))
            .file(unix_path.join("dl.c"))
            .file(unix_path.join("fs.c"))
            .file(unix_path.join("getaddrinfo.c"))
            .file(unix_path.join("getnameinfo.c"))
            .file(unix_path.join("loop-watcher.c"))
            .file(unix_path.join("loop.c"))
            .file(unix_path.join("pipe.c"))
            .file(unix_path.join("poll.c"))
            .file(unix_path.join("process.c"))
            .file(unix_path.join("random-devurandom.c"))
            .file(unix_path.join("signal.c"))
            .file(unix_path.join("stream.c"))
            .file(unix_path.join("tcp.c"))
            .file(unix_path.join("thread.c"))
            .file(unix_path.join("tty.c"))
            .file(unix_path.join("udp.c"));
    }

    // CMakeLists.txt has some special additions for AIX here; how do I test for it?

    if android {
        println!("cargo:rustc-link-lib=dl");
        build
            .file(unix_path.join("android-ifaddrs.c"))
            .file(unix_path.join("linux-core.c"))
            .file(unix_path.join("linux-inotify.c"))
            .file(unix_path.join("linux-syscalls.c"))
            .file(unix_path.join("procfs-exepath.c"))
            .file(unix_path.join("pthread-fixes.c"))
            .file(unix_path.join("random-getentropy.c"))
            .file(unix_path.join("random-getrandom.c"))
            .file(unix_path.join("random-sysctl-linux.c"))
            .file(unix_path.join("sysinfo-loadavg.c"));
    }

    // in CMakeLists.txt, this also tests for OS/390
    if apple || android || linux {
        build.file(unix_path.join("proctitle.c"));
    }

    if dragonfly || freebsd {
        build.file(unix_path.join("freebsd.c"));
    }

    if dragonfly || freebsd || netbsd || openbsd {
        build
            .file(unix_path.join("posix-hrtime.c"))
            .file(unix_path.join("bsd-proctitle.c"));
        println!("cargo:rustc-link-lib=kvm");
    }

    if apple || dragonfly || freebsd || netbsd || openbsd {
        build
            .file(unix_path.join("bsd-ifaddrs.c"))
            .file(unix_path.join("kqueue.c"));
    }

    if freebsd {
        build.file(unix_path.join("random-getrandom.c"));
    }

    if apple || openbsd {
        build.file(unix_path.join("random-getentropy.c"));
    }

    if apple {
        build
            .define("_DARWIN_UNLIMITED_SELECT", "1")
            .define("_DARWIN_USE_64_BIT_INODE", "1")
            .file(unix_path.join("darwin-proctitle.c"))
            .file(unix_path.join("darwin.c"))
            .file(unix_path.join("fsevents.c"));
    }

    if linux {
        build
            .define("_GNU_SOURCE", None)
            .define("_POSIX_C_SOURCE", "200112")
            .file(unix_path.join("linux-core.c"))
            .file(unix_path.join("linux-inotify.c"))
            .file(unix_path.join("linux-syscalls.c"))
            .file(unix_path.join("procfs-exepath.c"))
            .file(unix_path.join("random-getrandom.c"))
            .file(unix_path.join("random-sysctl-linux.c"))
            .file(unix_path.join("sysinfo-loadavg.c"));
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=rt");
    }

    if netbsd {
        build.file(unix_path.join("netbsd.c"));
    }

    if openbsd {
        build.file(unix_path.join("openbsd.c"));
    }

    // CMakeLists.txt has a check for OS/390 here again

    if solaris {
        build
            .define("__EXTENSIONS__", None)
            .define("_XOPEN_SOURCE", "500")
            .file(unix_path.join("no-proctitle.c"))
            .file(unix_path.join("sunos.c"));
        println!("cargo:rustc-link-lib=kstat");
        println!("cargo:rustc-link-lib=nsl");
        println!("cargo:rustc-link-lib=sendfile");
        println!("cargo:rustc-link-lib=socket");
    }

    build.compile("uv");
    Ok(())
}

fn generate_bindings<P: AsRef<Path>>(include_path: &P) -> Result<()> {
    println!("Generating bindings for libuv...");

    // bindgen needs the path as a String
    let include_path = include_path.as_ref();
    let header_path = include_path.join("uv.h");

    // generate ffi bindings
    let bindings = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .clang_arg(format!("-I{}", include_path.display()))
        .whitelist_type("uv_.+")
        .whitelist_function("uv_.+")
        .whitelist_var("(?i)uv_.+")
        .generate()
        .map_err(|_| Error::BindgenError)?;

    // write to file
    let outdir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings
        .write_to_file(&outdir)
        .map_err(|e| Error::PathError(outdir.to_string_lossy().into(), e))?;

    Ok(())
}

fn main() {
    let source_path = PathBuf::from("libuv");
    let mut include_path = source_path.join("include");

    // try pkg-config first
    if let Some(maybe_include) = try_pkgconfig() {
        // pkg-config successfully found a version of libuv, but may not be able to find headers...
        // that's ok, though, we have our own.
        if let Some(incl) = maybe_include {
            include_path = incl;
        }
    } else {
        build(&source_path).unwrap();
    }

    // generate bindings
    generate_bindings(&include_path).unwrap();
    println!("cargo:include={}", include_path.to_string_lossy());
}
