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
    for args in [&[][..], &["--nope"], &["check", "a.t", "b.t"]] {
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
