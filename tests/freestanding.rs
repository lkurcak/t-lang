//! The freestanding target (docs/39-freestanding.md): each `err_*` of
//! `tests/freestanding` must fail to build for it, with the error its
//! `// error:` (or, in a `t.toml`, `# error:`) comment says, at that line;
//! each `ok_*` must build, into an object that needs only the platform's
//! functions, and run, linked with a platform for this machine.
//!
//! A directory is a program: its `main.t`, or its `app/main.t`.
//! `compiled.rs` runs every other program that needs no std this way too.

#[path = "shared/platform.rs"]
mod platform;

use std::fs;
use std::path::{Path, PathBuf};
use t_lang::Target;

/// The cases, by path: a `.t` file, or a directory.
fn cases() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/freestanding");
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    out.sort();
    out
}

/// The program case `p` is: its first file.
fn root(p: &Path) -> PathBuf {
    match p.is_dir() {
        true if p.join("main.t").exists() => p.join("main.t"),
        true => p.join("app/main.t"),
        false => p.to_path_buf(),
    }
}

/// Every file of case `p`.
fn files(p: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut todo = vec![p.to_path_buf()];
    while let Some(p) = todo.pop() {
        match p.is_dir() {
            true => todo.extend(fs::read_dir(&p).unwrap().map(|e| e.unwrap().path())),
            false => out.push(p),
        }
    }
    out.sort();
    out
}

/// The error case `p` expects: its text, and the file and line it's at.
fn expected(p: &Path) -> (String, PathBuf, usize) {
    for f in files(p) {
        let src = fs::read_to_string(&f).unwrap();
        for (i, line) in src.lines().enumerate() {
            for marker in ["// error:", "# error:"] {
                if let Some(at) = line.find(marker) {
                    return (line[at + marker.len()..].trim().to_string(), f, i + 1);
                }
            }
        }
    }
    panic!("{} says no error", p.display())
}

fn build(p: &Path) -> t_lang::Built {
    let root = root(p);
    let src = fs::read_to_string(&root).unwrap();
    let options = t_lang::cgen::Options {
        lines: false,
        target: Target::Freestanding,
    };
    t_lang::build_c(&root, &src, &t_lang::load::read_file, options)
}

#[test]
fn what_needs_the_hosted_target_doesnt_build() {
    let mut failures = Vec::new();
    let mut n = 0;
    for p in cases() {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("err_") {
            continue;
        }
        n += 1;
        let (want, file, line) = expected(&p);
        let built = build(&p);
        let e = match built.result {
            Ok(_) => {
                failures.push(format!("{name}: built"));
                continue;
            }
            Err(e) => e,
        };
        let (at, local) = built.sources.local(e.span);
        let shown = built.sources.src(at)[..local.start].matches('\n').count() + 1;
        let same_file = fs::canonicalize(&at.path).ok() == fs::canonicalize(&file).ok();
        if !e.message.contains(&want) || !same_file || shown != line {
            failures.push(format!(
                "{name}: wanted `{want}` at {}:{line}, got:\n{}",
                file.display(),
                e.render_in(&built.sources)
            ));
        }
    }
    assert!(n >= 4, "only {n} cases");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
#[cfg_attr(windows, ignore = "the C backend is for Linux and macOS, for now")]
fn what_needs_no_std_builds_and_runs() {
    let dir = std::env::temp_dir().join(format!("t-lang-freestanding-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mut n = 0;
    for p in cases() {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("ok_") {
            continue;
        }
        n += 1;
        let c = match build(&p).result {
            Ok(c) => c,
            Err(e) => panic!("{name} didn't build: {}", e.message),
        };
        let exe = dir.join(name.trim_end_matches(".t"));
        if let Err(e) = platform::link(&c, &exe) {
            panic!("{name}: {e}");
        }
        let out = std::process::Command::new(&exe).output().unwrap();
        assert!(out.status.success(), "{name}: {out:?}");
    }
    assert!(n >= 2, "only {n} cases");
    let _ = fs::remove_dir_all(&dir);
}
