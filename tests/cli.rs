//! The `t` command line: what it prints and how it exits, outside of what
//! a program does (`golden.rs`) and `t fmt` (`fmt.rs`).

use std::process::{Command, Output};

fn t(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_t"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_goes_to_stdout_and_succeeds() {
    for flag in ["--help", "-h"] {
        let out = t(&[flag]);
        assert_eq!(out.status.code(), Some(0));
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.contains("usage: t [run|check|mono] <file.t>"),
            "{stdout}"
        );
        assert!(stdout.contains("t --version"), "{stdout}");
        assert!(out.stderr.is_empty());
    }
}

#[test]
fn version_is_the_crate_version() {
    for flag in ["--version", "-V"] {
        let out = t(&[flag]);
        assert_eq!(out.status.code(), Some(0));
        let want = format!("t {}\n", env!("CARGO_PKG_VERSION"));
        assert_eq!(String::from_utf8(out.stdout).unwrap(), want);
    }
}

/// A mistake still gets the short usage, on stderr, and exit code 2.
#[test]
fn bad_arguments_print_usage() {
    for args in [
        &[][..],
        &["--nope"],
        &["check", "a.t", "b.t"],
        &["tree-sitter"],
        &["tree-sitter", "--out"],
        &["tree-sitter", "--helix"],
        &["tree-sitter", "--nope", "dir"],
        &["test"],
        &["test", "--nope"],
        &["test", "a.t", "f", "g"],
        &["run", "--deny-warnings", "a.t"],
    ] {
        let out = t(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.starts_with("usage: t "), "{args:?}: {stderr}");
    }
}

/// A program of several files reads them from beside its `main.t`, and
/// says which file an error is in.
#[test]
fn reads_the_files_of_modules() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/t");
    let out = t(&[&format!("{dir}/module_files/main.t")]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "0\n12\n25\n42\n7\n0\n"
    );

    let out = t(&["check", &format!("{dir}/err_module_file_type_error/main.t")]);
    assert_eq!(out.status.code(), Some(1));
    // With `\` between a path's parts on Windows.
    let stderr = String::from_utf8(out.stderr).unwrap().replace('\\', "/");
    assert!(
        stderr.contains("err_module_file_type_error/a/b.t:2:5"),
        "{stderr}"
    );
}

/// A program reads the packages its `t.toml` names; a library, which
/// starts in `lib.t`, checks on its own, but has no `main` to run.
#[test]
fn reads_packages_and_checks_libraries() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/t");
    let out = t(&[&format!("{dir}/pkg_basic/main.t")]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "2\n2\n2\n");

    let lib = format!("{dir}/pkg_basic/counter/lib.t");
    let out = t(&["check", &lib]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let out = t(&[&lib]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("has no `main` to run"), "{stderr}");
}

/// `t test` says how each test went, as it goes, then why each failure
/// failed, then how many passed and failed; it fails if any did.
#[test]
fn test_runs_the_tests_and_says_how_they_went() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/t");
    let out = t(&["test", &format!("{dir}/test_basic.t")]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let head = "test doubles ... ok\n\
                test parsing::inner ... ok\n\
                test parsing::deeper::deepest ... ok\n\
                test fails ... FAILED\n\
                \n\
                --- fails\n\
                runtime error: panicked: assertion failed\n";
    assert!(stdout.starts_with(head), "{stdout}");
    assert!(stdout.ends_with("\n3 passed; 1 failed\n"), "{stdout}");
    // Not `main`'s output: it isn't run.
    assert!(!stdout.contains("\n4\n"), "{stdout}");

    // With a filter, only the tests whose names have it in them.
    let out = t(&["test", &format!("{dir}/test_basic.t"), "parsing"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        "test parsing::inner ... ok\ntest parsing::deeper::deepest ... ok\n\n2 passed; 0 failed\n"
    );

    // A library's, with no `main`.
    let out = t(&["test", &format!("{dir}/test_library/lib.t")]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    // A program that doesn't check runs no tests.
    let out = t(&["test", &format!("{dir}/err_test_params.t")]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}

/// `--deny-warnings` makes `t check` and `t test` fail when there are
/// warnings, after saying them; tests still run.
#[test]
fn deny_warnings_fails_on_warnings() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/t");
    let warned = format!("{dir}/test_dead_code.t");
    let out = t(&["check", &warned]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    for args in [
        &["check", "--deny-warnings", &warned][..],
        &["check", &warned, "--deny-warnings"],
    ] {
        let out = t(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}: {out:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.contains("function `unused` is never used"),
            "{stderr}"
        );
        assert!(
            stderr.contains("there was a warning, and `--deny-warnings`"),
            "{stderr}"
        );
    }
    let out = t(&["test", "--deny-warnings", &warned]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("test uses_helper ... ok"), "{stdout}");
    let out = t(&["test", &warned]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    // Nothing to deny: as without it.
    let clean = format!("{dir}/test_library/lib.t");
    let out = t(&["check", "--deny-warnings", &clean]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

/// `t tree-sitter <dir>` installs the parser and queries as Neovim looks for
/// them, replacing what's there.
#[test]
fn tree_sitter_installs_the_parser_and_queries() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("tree_sitter_cli");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("queries/t")).unwrap();
    std::fs::write(dir.join("queries/t/highlights.scm"), "old").unwrap();

    let out = t(&["tree-sitter", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
    assert!(std::fs::metadata(dir.join("parser/t.so")).unwrap().len() > 0);
    assert_eq!(
        std::fs::read_to_string(dir.join("queries/t/highlights.scm")).unwrap(),
        tree_sitter_t::HIGHLIGHTS_QUERY
    );
    let names: Vec<_> = std::fs::read_dir(dir.join("parser"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["t.so"], "no temporary files left behind");

    // Again, with nothing to change: the files are left as they are, so an
    // editor with the parser loaded keeps a file that matches it.
    let modified = |name: &str| {
        std::fs::metadata(dir.join(name))
            .unwrap()
            .modified()
            .unwrap()
    };
    let before = [
        modified("parser/t.so"),
        modified("queries/t/highlights.scm"),
    ];
    std::thread::sleep(std::time::Duration::from_millis(10));
    let out = t(&["tree-sitter", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        [
            modified("parser/t.so"),
            modified("queries/t/highlights.scm")
        ],
        before
    );
}

/// `t tree-sitter --helix <dir>` puts the parser where Helix looks for it.
#[test]
fn tree_sitter_installs_for_helix() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("tree_sitter_helix");
    let _ = std::fs::remove_dir_all(&dir);

    let out = t(&["tree-sitter", "--helix", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let parser = if cfg!(windows) { "t.dll" } else { "t.so" };
    assert!(
        std::fs::metadata(dir.join("grammars").join(parser))
            .unwrap()
            .len()
            > 0
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("queries/t/highlights.scm")).unwrap(),
        tree_sitter_t::HIGHLIGHTS_QUERY
    );
    assert!(!dir.join("parser").exists());
}

/// Runs the program `src`, as `t` does, with `input` for its stdin.
fn run_with_input(name: &str, src: &str, input: &[u8]) -> Output {
    use std::io::Write;
    let dir = std::env::temp_dir().join(format!("t-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.t");
    std::fs::write(&path, src).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_t"))
        .arg(&path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    out
}

/// What echoes its input, line by line, after a prompt.
const ECHO: &str = "\
use alloc::string::String;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut input = os.stdin;
    let mut out = os.stdout;
    let mut line = String::new();
    out.put(&\"> \")?;
    while input.read_line(&mut line)? {
        out.put(&line.len())?;
        out.line(&line.as_str())?;
        out.put(&\"> \")?;
    }
    out.line(&\"bye\")
}
";

/// A program reads its input a line at a time, without the line break,
/// `\n` or `\r\n`; the last line needn't have one.
#[test]
fn programs_read_lines_of_input() {
    let out = run_with_input("lines", ECHO, "añ\r\nb\n\nlast".as_bytes());
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, "> 3añ\n> 1b\n> 0\n> 4last\n> bye\n");
    assert!(out.stderr.is_empty());
}

/// Input that isn't UTF-8 is an error, which `main` gives back: it's
/// written to stderr, after what was written before it, and the program
/// fails.
#[test]
fn input_that_isnt_text_fails_the_program() {
    let out = run_with_input("bytes", ECHO, b"ok\n\xFF\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "> 2ok\n> ");
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "error: stream did not contain valid UTF-8\n"
    );
}

/// A small write succeeds into the interpreter's buffer, then fails when
/// the buffer is flushed. The command must still report that failure.
#[cfg(unix)]
#[test]
fn buffered_output_errors_fail_the_command() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    for (file, message) in [
        ("examples/brew.t", "runtime error: cannot flush stdout:"),
        (
            "tests/t/panic_message.t",
            "runtime error: panicked: the end",
        ),
    ] {
        let (writer, reader) = UnixStream::pair().unwrap();
        drop(reader);
        let writer: OwnedFd = writer.into();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let out = Command::new(env!("CARGO_BIN_EXE_t"))
            .arg(path)
            .stdout(std::process::Stdio::from(writer))
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains(message), "{stderr}");
        assert!(stderr.contains(&format!("{file}:")), "{stderr}");
    }
}
