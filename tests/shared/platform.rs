//! Building programs for the freestanding target in tests
//! (docs/39-freestanding.md): the object `t build` makes is checked to
//! need nothing but the platform's functions, then linked with
//! `platform.c`, a platform for a hosted machine, to run it.

// Each test binary uses only some of this.
#![allow(dead_code)]

use std::path::Path;

/// The headers a freestanding C program may include: C11's freestanding
/// ones (§4 of the standard).
const HEADERS: [&str; 9] = [
    "float.h",
    "iso646.h",
    "limits.h",
    "stdalign.h",
    "stdarg.h",
    "stdbool.h",
    "stddef.h",
    "stdint.h",
    "stdnoreturn.h",
];

/// What a freestanding object may need from outside: the platform's
/// functions, and what C compilers call on their own even when
/// freestanding.
const NEEDS: [&str; 9] = [
    "t_platform_write",
    "t_platform_halt",
    "t_platform_alloc",
    "t_platform_realloc",
    "t_platform_free",
    "memcpy",
    "memmove",
    "memset",
    "memcmp",
];

/// Checks that C program `c` includes only freestanding headers.
pub fn headers(c: &str) -> Result<(), String> {
    for line in c.lines() {
        let Some(h) = line.trim().strip_prefix("#include") else {
            continue;
        };
        let h = h.trim().trim_matches(['<', '>', '"']);
        if !HEADERS.contains(&h) {
            return Err(format!("the C includes `{h}`, which isn't freestanding"));
        }
    }
    Ok(())
}

/// Builds freestanding C program `c` into an executable at `exe`, through
/// an object in the same directory: checks the object needs only what
/// `NEEDS` says (unless `$T_LANG_CFLAGS` adds flags, as sanitizers need
/// their runtimes), then links it with `platform.c`.
pub fn link(c: &str, exe: &Path) -> Result<(), String> {
    headers(c)?;
    let obj = exe.with_extension("o");
    t_lang::cgen::compile(c, &obj, t_lang::Target::Freestanding)?;
    let flags = std::env::var("T_LANG_CFLAGS").unwrap_or_default();
    if flags.trim().is_empty() {
        let nm = std::process::Command::new("nm")
            .arg("-u")
            .arg(&obj)
            .output()
            .map_err(|e| format!("cannot run `nm`: {e}"))?;
        for line in String::from_utf8_lossy(&nm.stdout).lines() {
            let name = line.split_whitespace().last().unwrap_or("");
            let name = match cfg!(target_os = "macos") {
                true => name.strip_prefix('_').unwrap_or(name),
                false => name,
            };
            if !name.is_empty() && !NEEDS.contains(&name) {
                return Err(format!(
                    "the object needs `{name}`, which isn't the platform's"
                ));
            }
        }
    }
    let platform = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/shared/platform.c");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let out = std::process::Command::new(&cc)
        .args(["-std=c11", "-O2", "-o"])
        .arg(exe)
        .arg(&obj)
        .arg(&platform)
        .args(flags.split_whitespace())
        .output()
        .map_err(|e| format!("cannot run `{cc}`: {e}"))?;
    let _ = std::fs::remove_file(&obj);
    match out.status.success() {
        true => Ok(()),
        false => Err(format!(
            "linking with the platform failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        )),
    }
}
