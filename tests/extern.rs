//! `extern fn` and volatile accesses (docs/40-extern.md), which only a
//! machine can run: each program of `tests/extern` is built, run, and must
//! write what its `// prints:` lines say to `stdout`, and end with the
//! code its `// exit:` says, or 0.
//!
//! A `hosted_*` program is built for the hosted target, and calls the C
//! library. A `machine_*` one is built for the freestanding target, and
//! linked with `shared/machine.c`, a machine whose device it drives, and
//! its own `.c` beside it, if it has one: its
//! object must need nothing but the platform's functions, the C functions
//! it imports, and none of the platform's it writes itself.
//!
//! The rules of `extern fn` are tested with the other programs, in
//! `tests/t` (`*extern*`, `*volatile*`), and `tests/freestanding`.

#[path = "shared/platform.rs"]
mod platform;

use std::fs;
use std::path::{Path, PathBuf};
use t_lang::Target;

fn cases() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/extern");
    let mut out: Vec<PathBuf> = (fs::read_dir(dir).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "t"))
        .collect();
    out.sort();
    out
}

/// The text after `marker` on each line of `src` that has it.
fn directives<'a>(src: &'a str, marker: &str) -> Vec<&'a str> {
    (src.lines())
        .filter_map(|l| Some(l[l.find(marker)? + marker.len()..].trim()))
        .collect()
}

/// The names of the C functions `src` imports: `extern fn f(..);`.
fn imports(src: &str) -> Vec<String> {
    (src.lines())
        .filter(|l| l.starts_with("extern fn ") && l.trim_end().ends_with(';'))
        .filter_map(|l| Some(l["extern fn ".len()..l.find('(')?].to_string()))
        .collect()
}

#[test]
#[cfg_attr(windows, ignore = "the C backend is for Linux and macOS, for now")]
fn what_needs_the_machine_runs_on_one() {
    let dir = std::env::temp_dir().join(format!("t-lang-extern-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let mut failures = Vec::new();
    let mut n = 0;
    for p in cases() {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let src = fs::read_to_string(&p).unwrap();
        let target = match name.starts_with("machine_") {
            true => Target::Freestanding,
            false => Target::Hosted,
        };
        n += 1;
        let options = t_lang::cgen::Options {
            lines: false,
            target,
        };
        let c = match t_lang::build_c(&p, &src, &t_lang::load::read_file, options).result {
            Ok(c) => c,
            Err(e) => {
                failures.push(format!("{name} didn't build: {}", e.message));
                continue;
            }
        };
        // Each access of a `Volatile` is C's `volatile` (lemma V): its
        // reads and its writes.
        let read = |line: &str| line.contains("return *(volatile ");
        let write =
            |line: &str| line.trim_start().starts_with("*(volatile ") && line.contains(" = ");
        let uses = |m: &str| src.contains(&format!(".{m}("));
        if src.contains("Volatile::")
            && ((uses("read") && !c.lines().any(read)) || (uses("write") && !c.lines().any(write)))
        {
            failures.push(format!(
                "{name}: its C reads or writes a `Volatile` but not volatilely"
            ));
        }
        let exe = dir.join(&name);
        let built = match target {
            Target::Hosted => t_lang::cgen::compile(&c, &exe, target),
            Target::Freestanding => {
                let more: Vec<PathBuf> = Some(p.with_extension("c"))
                    .filter(|c| c.exists())
                    .into_iter()
                    .collect();
                platform::link_with(&c, &exe, "machine.c", &more, &imports(&src))
            }
        };
        if let Err(e) = built {
            failures.push(format!("{name}: {e}"));
            continue;
        }
        let out = std::process::Command::new(&exe).output().unwrap();
        let want: String = directives(&src, "// prints:")
            .iter()
            .map(|l| format!("{l}\n"))
            .collect();
        let got = String::from_utf8_lossy(&out.stdout);
        if !got.starts_with(&want) {
            failures.push(format!("{name}: printed {got:?}, not {want:?}"));
        }
        let code: i32 = directives(&src, "// exit:")
            .first()
            .map_or(0, |c| c.parse().unwrap());
        if out.status.code() != Some(code) {
            failures.push(format!(
                "{name}: ended with {}, not {code}; it wrote {:?}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    let _ = fs::remove_dir_all(&dir);
    assert!(n >= 3, "only {n} cases");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
