//! Git dependencies (see docs/27-git-dependencies.md): packages fetched
//! from repositories made here, in a temporary directory, over `file://`,
//! by the `t` command, with a cache of its own. No test needs the network.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh directory for test `name`.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("git")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {}", text(&out.stderr));
    text(&out.stdout).trim().to_string()
}

/// Writes `files` into `dir`, a repository made if it isn't one, and
/// commits them; returns the commit's hash.
fn commit(dir: &Path, files: &[(&str, &str)]) -> String {
    std::fs::create_dir_all(dir).unwrap();
    if !dir.join(".git").exists() {
        git(dir, &["init", "-q"]);
    }
    write(dir, files);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "commit"]);
    git(dir, &["rev-parse", "HEAD"])
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, src) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, src).unwrap();
    }
}

/// `dir`'s `file://` URL: `file:///C:/a/b` on Windows.
fn url(dir: &Path) -> String {
    let path = dir.display().to_string().replace('\\', "/");
    match path.starts_with('/') {
        true => format!("file://{path}"),
        false => format!("file:///{path}"),
    }
}

/// A `t.toml` line for a git dependency.
fn dep(name: &str, repo: &Path, rev: &str) -> String {
    format!("{name} = {{ git = \"{}\", rev = \"{rev}\" }}\n", url(repo))
}

/// A `t.toml` of git dependencies, for a package that uses std, as the
/// programs here do to print.
fn manifest(deps: &[String]) -> String {
    format!("libraries = [\"std\"]\n\n[dependencies]\n{}", deps.concat())
}

/// Runs `t` on `args` in `dir`, with `cache` as its cache.
fn t(dir: &Path, cache: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_t"))
        .current_dir(dir)
        .env("T_LANG_CACHE", cache)
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Runs the program in `dir`, and asserts it succeeds; returns its output.
fn runs(dir: &Path, cache: &Path) -> String {
    let out = t(dir, cache, &["main.t"]);
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        text(&out.stdout),
        text(&out.stderr)
    );
    text(&out.stdout)
}

/// Checks the program in `dir`, and asserts it fails; returns what it says.
fn fails(dir: &Path, cache: &Path) -> String {
    let out = t(dir, cache, &["check", "main.t"]);
    assert!(!out.status.success(), "it checked: {}", text(&out.stderr));
    text(&out.stderr)
}

const MAIN: &str = "fn main(os: std::Os) -> Result<(), std::io::Error> {\n    \
                    let mut out = os.stdout;\n    \
                    out.line(&g::f())?;\n    \
                    Result::Ok(())\n}\n";

/// A library that exports `f`, returning `n`.
fn lib(n: i64) -> String {
    format!("export fn f() -> i64 {{ {n} }}\n")
}

#[test]
fn a_program_runs_with_a_package_from_git() {
    let top = scratch("runs");
    let rev = commit(&top.join("g"), &[("lib.t", &lib(42))]);
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    let cache = top.join("cache");
    assert_eq!(runs(&app, &cache), "42\n");
    // One checkout, by its commit, of the files alone.
    let slots: Vec<_> = std::fs::read_dir(cache.join("git")).unwrap().collect();
    assert_eq!(slots.len(), 1);
    let checkout = slots[0].as_ref().unwrap().path().join(&rev);
    assert!(checkout.join("lib.t").is_file());
    assert!(!checkout.join(".git").exists(), "no repository is kept");
    let names: Vec<_> = std::fs::read_dir(checkout.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, [std::ffi::OsString::from(&rev)], "no temporary left");
}

/// The rev is what's fetched, not the branch's newest commit.
#[test]
fn the_rev_is_what_is_fetched() {
    let top = scratch("rev");
    let old = commit(&top.join("g"), &[("lib.t", &lib(1))]);
    commit(&top.join("g"), &[("lib.t", &lib(2))]);
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &old)])),
        ],
    );
    assert_eq!(runs(&app, &top.join("cache")), "1\n");
}

/// Once fetched, a rev is never fetched again: it can't change.
#[test]
fn a_fetched_package_needs_no_repository() {
    let top = scratch("offline");
    let rev = commit(&top.join("g"), &[("lib.t", &lib(7))]);
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    let cache = top.join("cache");
    assert_eq!(runs(&app, &cache), "7\n");
    std::fs::remove_dir_all(top.join("g")).unwrap();
    assert_eq!(runs(&app, &cache), "7\n");
}

/// Two `t`s fetching one rev at once both get it, whichever is first.
#[test]
fn fetching_at_once_is_fine() {
    let top = scratch("race");
    let rev = commit(&top.join("g"), &[("lib.t", &lib(5))]);
    let cache = top.join("cache");
    let apps: Vec<PathBuf> = (0..4)
        .map(|i| {
            let app = top.join(format!("app{i}"));
            write(
                &app,
                &[
                    ("main.t", MAIN),
                    ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
                ],
            );
            app
        })
        .collect();
    std::thread::scope(|s| {
        let runs: Vec<_> = (apps.iter())
            .map(|app| s.spawn(|| runs(app, &cache)))
            .collect();
        for r in runs {
            assert_eq!(r.join().unwrap(), "5\n");
        }
    });
}

/// A package from git can have modules in files and depend, by path, on
/// another package in its own repository.
#[test]
fn a_package_from_git_has_files_and_path_dependencies() {
    let top = scratch("inner");
    let rev = commit(
        &top.join("g"),
        &[
            (
                "lib.t",
                "mod m;\nexport fn f() -> i64 with panic { m::g() + inner::h() }\n",
            ),
            ("m.t", "pub fn g() -> i64 { 40 }\n"),
            ("t.toml", "[dependencies]\ninner = { path = \"inner\" }\n"),
            ("inner/lib.t", "export fn h() -> i64 { 2 }\n"),
        ],
    );
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    assert_eq!(runs(&app, &top.join("cache")), "42\n");
}

/// A package from git can depend on one from another repository.
#[test]
fn a_package_from_git_depends_on_one_from_git() {
    let top = scratch("nested");
    let base = commit(
        &top.join("base"),
        &[("lib.t", "export fn b() -> i64 { 41 }\n")],
    );
    let rev = commit(
        &top.join("g"),
        &[
            (
                "lib.t",
                "export fn f() -> i64 with panic { base::b() + 1 }\n",
            ),
            (
                "t.toml",
                &manifest(&[dep("base", &top.join("base"), &base)]),
            ),
        ],
    );
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    assert_eq!(runs(&app, &top.join("cache")), "42\n");
}

/// A program and a package it depends on can both depend on one rev: it's
/// one package, so its types are one type.
#[test]
fn one_rev_is_one_package() {
    let top = scratch("diamond");
    let rev = commit(
        &top.join("g"),
        &[(
            "lib.t",
            "#[derive(Copy, Discard)]\nexport struct P { export x: i64 }\n\
             export fn f() -> i64 { 42 }\n",
        )],
    );
    let app = top.join("app");
    let g = dep("g", &top.join("g"), &rev);
    write(
        &app,
        &[
            (
                "main.t",
                "fn main(os: std::Os) -> Result<(), std::io::Error> {\n    \
                 let mut out = os.stdout;\n    \
                 let p: g::P = h::p();\n    \
                 out.line(&p.x)?;\n    \
                 Result::Ok(())\n}\n",
            ),
            (
                "t.toml",
                &manifest(&[g.clone(), "h = { path = \"h\" }\n".into()]),
            ),
            ("h/lib.t", "export fn p() -> g::P { g::P { x: g::f() } }\n"),
            ("h/t.toml", &manifest(&[g])),
        ],
    );
    assert_eq!(runs(&app, &top.join("cache")), "42\n");
}

/// A program has one version of each package, for now.
#[test]
fn two_revs_of_one_repository_are_an_error() {
    let top = scratch("versions");
    let one = commit(&top.join("g"), &[("lib.t", &lib(1))]);
    let two = commit(&top.join("g"), &[("lib.t", &lib(2))]);
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            (
                "t.toml",
                &manifest(&[
                    dep("g", &top.join("g"), &one),
                    "h = { path = \"h\" }\n".into(),
                ]),
            ),
            ("h/lib.t", "export fn f() -> i64 { g::f() }\n"),
            ("h/t.toml", &manifest(&[dep("g", &top.join("g"), &two)])),
        ],
    );
    let err = fails(&app, &top.join("cache"));
    assert!(err.contains("two versions of"), "{err}");
    assert!(err.contains(&one[..7]) && err.contains(&two[..7]), "{err}");
}

/// One repository, at one rev, gets one name, as a directory does.
#[test]
fn one_package_from_git_has_one_name() {
    let top = scratch("names");
    let rev = commit(&top.join("g"), &[("lib.t", &lib(1))]);
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            (
                "t.toml",
                &manifest(&[
                    dep("g", &top.join("g"), &rev),
                    "h = { path = \"h\" }\n".into(),
                ]),
            ),
            ("h/lib.t", "export fn f() -> i64 { k::f() }\n"),
            ("h/t.toml", &manifest(&[dep("k", &top.join("g"), &rev)])),
        ],
    );
    let err = fails(&app, &top.join("cache"));
    assert!(err.contains("call it the same everywhere"), "{err}");
}

/// Outside its checkout, a path means nothing a package's author chose.
#[test]
fn a_package_from_git_cant_depend_on_a_path_outside_it() {
    let top = scratch("escape");
    write(&top, &[("outside/lib.t", "export fn h() -> i64 { 1 }\n")]);
    let rev = commit(
        &top.join("repo"),
        &[
            ("lib.t", "export fn f() -> i64 { o::h() }\n"),
            ("t.toml", "[dependencies]\no = { path = \"../outside\" }\n"),
        ],
    );
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("repo"), &rev)])),
        ],
    );
    let err = fails(&app, &top.join("cache"));
    assert!(err.contains("outside the repository"), "{err}");
}

/// Nor through a link: the checkout's files are what's in the commit, and
/// a link is a file that holds its target.
#[cfg(unix)]
#[test]
fn a_package_from_git_cant_link_outside_it() {
    let top = scratch("link");
    write(&top, &[("outside/lib.t", "export fn h() -> i64 { 1 }\n")]);
    let repo = top.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::os::unix::fs::symlink(top.join("outside"), repo.join("inner")).unwrap();
    let rev = commit(
        &repo,
        &[
            ("lib.t", "export fn f() -> i64 { o::h() }\n"),
            ("t.toml", "[dependencies]\no = { path = \"inner\" }\n"),
        ],
    );
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &repo, &rev)])),
        ],
    );
    let err = fails(&app, &top.join("cache"));
    assert!(
        err.contains("can't read `<g@") && err.contains("package `o`"),
        "{err}"
    );
}

#[test]
fn a_rev_that_isnt_there_is_an_error() {
    let top = scratch("missing");
    commit(&top.join("g"), &[("lib.t", &lib(1))]);
    let app = top.join("app");
    let rev = "0123456789abcdef0123456789abcdef01234567";
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), rev)])),
        ],
    );
    let cache = top.join("cache");
    let err = fails(&app, &cache);
    assert!(err.contains("can't fetch"), "{err}");
    assert!(err.contains("t.toml"), "points at the entry: {err}");
    // Nothing half-fetched is left to be taken for the package.
    let left: Vec<_> = walk(&cache.join("git"));
    assert!(left.iter().all(|p| !p.ends_with(rev)), "{left:?}");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten() {
        let p = e.unwrap().path();
        out.push(p.clone());
        if p.is_dir() {
            out.extend(walk(&p));
        }
    }
    out
}

#[test]
fn a_rev_without_a_library_is_an_error() {
    let top = scratch("no_lib");
    let rev = commit(&top.join("g"), &[("README", "nothing\n")]);
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    let err = fails(&app, &top.join("cache"));
    assert!(err.contains("the first file of package `g`"), "{err}");
}

/// An error in a package from git is shown in its file, by the package's
/// name and rev rather than where the cache happens to be.
#[test]
fn errors_in_a_package_from_git_name_it_by_rev() {
    let top = scratch("blame");
    let rev = commit(
        &top.join("g"),
        &[("lib.t", "export fn f() -> i64 { true }\n")],
    );
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    let cache = top.join("cache");
    let err = fails(&app, &cache);
    assert!(err.contains(&format!("<g@{}>/lib.t", &rev[..7])), "{err}");
    assert!(!err.contains(&cache.display().to_string()), "{err}");
}

/// Fetching runs nothing from the repository: not its hooks, not the
/// user's.
#[test]
fn fetching_runs_no_hooks() {
    let top = scratch("hooks");
    let rev = commit(&top.join("g"), &[("lib.t", &lib(3))]);
    let hooks = top.join("hooks");
    let marker = top.join("ran");
    write(
        &hooks,
        &[(
            "post-checkout",
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )],
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let hook = hooks.join("post-checkout");
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let app = top.join("app");
    write(
        &app,
        &[
            ("main.t", MAIN),
            ("t.toml", &manifest(&[dep("g", &top.join("g"), &rev)])),
        ],
    );
    let out = Command::new(env!("CARGO_BIN_EXE_t"))
        .current_dir(&app)
        .env("T_LANG_CACHE", top.join("cache"))
        // The user's configuration, as git reads it from the environment.
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", &hooks)
        .arg("main.t")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(!marker.exists(), "a hook ran");
}
