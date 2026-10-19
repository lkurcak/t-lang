//! Runs every `tests/t/*.t` program and compares it against the expectations
//! written in its comments: `// expect:`, `// error:` and `// warning:` (see
//! `shared/expect.rs`). Every test must have at least one. A program of
//! several files is a directory, `tests/t/<name>/main.t` and its modules'
//! files.
//!
//! A `tests/t/<name>.mono` file next to a test holds what `t mono` prints
//! for it: the program after monomorphisation. Run with `BLESS=1` to rewrite
//! those files from what it prints now.

#[path = "shared/expect.rs"]
mod expect;

use expect::Expectations;
use std::fs;
use std::path::{Path, PathBuf};
use t_lang::check::NoteKind;

/// The first file of every program in `tests/t`, in order: `tests/t/*.t`
/// and `tests/t/*/main.t`.
fn programs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter_map(|p| match p.is_dir() {
            true => Some(p.join("main.t")),
            false => p.extension().is_some_and(|ext| ext == "t").then_some(p),
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no tests found in {}", dir.display());
    paths
}

/// What a test is called: its file's name, or its directory's.
fn name(path: &Path) -> String {
    let path = match path.file_name().is_some_and(|n| n == "main.t") {
        true => path.parent().unwrap(),
        false => path,
    };
    path.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn golden() {
    let paths = programs();

    let mut failures = Vec::new();
    for path in &paths {
        let name = name(path);
        let files = expect::files_of(path);
        let all: String = files.iter().map(|(_, src)| src.as_str()).collect();
        let want = Expectations::of(&all);
        if want.output.is_empty() && want.error.is_none() && want.warnings.is_empty() {
            failures.push(format!(
                "{name}: has no `// expect:`, `// error:` or `// warning:` lines"
            ));
            continue;
        }
        if let Err(failure) = expect::check_files(&files) {
            failures.push(format!("{name}: {failure}"));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} golden tests failed:\n\n{}\n",
            failures.len(),
            paths.len(),
            failures.join("\n\n")
        );
    }
}

/// What monomorphisation makes of each program with a `.mono` file.
#[test]
fn mono() {
    let bless = std::env::var_os("BLESS").is_some();
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in programs() {
        let mono = path.with_extension("mono");
        if !mono.exists() {
            continue;
        }
        checked += 1;
        let name = name(&path);
        let src = fs::read_to_string(&path).unwrap();
        let checked = t_lang::check_files(&path, &src, &t_lang::load::read_file);
        let actual = match checked.result {
            Ok(program) => program.to_string(),
            Err(e) => {
                failures.push(format!("{name}: {}", e.render_in(&checked.sources)));
                continue;
            }
        };
        if bless {
            fs::write(&mono, &actual).unwrap();
        } else if fs::read_to_string(&mono).unwrap() != actual {
            failures.push(format!(
                "{name}: `t mono` printed something else (run with BLESS=1 to update \
                 {}):\n{actual}",
                mono.display()
            ));
        }
    }
    assert!(checked > 0, "no `.mono` files");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// A program without generics, nested functions or local types prints as
/// itself: parsing what `t mono` prints gives a program that prints the
/// same, and does the same. (The names of the copies of generic things,
/// like `id::<int>`, of lifted nested functions and local types, like
/// `main::helper`, and of closures' types, which a function that returns
/// one names, like `{closure#12}`, don't parse.)
#[test]
fn printing_round_trips() {
    let mut failures = Vec::new();
    for path in programs() {
        let name = name(&path);
        let files = expect::files_of(&path);
        let Ok(program) = expect::run_checked(&files) else {
            continue;
        };
        let printed = program.to_string();
        let lifted = program.fns.iter().map(|f| &f.name.name);
        let lifted = lifted.chain(program.types.iter().map(|d| &d.name().name));
        if printed.contains("::<")
            || printed.contains("{closure#")
            || lifted.into_iter().any(|name| name.contains("::"))
        {
            continue;
        }
        let reprinted = match t_lang::check_source(&printed).result {
            Ok(p) => p.to_string(),
            Err(e) => {
                failures.push(format!("{name}: {}", e.render(&name, &printed)));
                continue;
            }
        };
        if reprinted != printed {
            failures.push(format!("{name}: printed\n{printed}\nthen\n{reprinted}"));
            continue;
        }
        let (before, after) = (expect::run(&files), t_lang::run_source(&printed));
        if !expect::same_behaviour(&before, &after) {
            failures.push(format!(
                "{name}: does something else once printed:\n{printed}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Renaming anything declared in a program to a name nothing else has is
/// accepted, and leaves a program that does the same. So every name for
/// it is found, and nothing else is touched.
#[test]
fn renaming_keeps_meaning() {
    let mut failures = Vec::new();
    let mut renamed = 0;
    for path in programs() {
        let name = name(&path);
        let files = expect::files_of(&path);
        let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
            Some((_, src)) => Ok(src.clone()),
            None => t_lang::no_files(p),
        };
        let Ok((sources, prog, info)) = t_lang::rename::checked(&path, &files[0].1, &read) else {
            continue;
        };
        let src = sources.text();
        let before = expect::run(&files);
        let mut defs: Vec<_> = info
            .notes
            .iter()
            .filter(|n| n.def == Some(n.span))
            .collect();
        defs.sort_by_key(|n| n.span.start);
        defs.dedup_by_key(|n| n.span);
        for n in defs {
            if t_lang::rename::target(&sources, &prog, &info, n.span.start).is_err() {
                continue;
            }
            let new = match n.kind {
                NoteKind::Struct(_) | NoteKind::Enum(_) | NoteKind::TypeParam(_) => "Renamed",
                NoteKind::Variant(..) => "Renamed",
                _ => "renamed",
            };
            let old = &src[n.span.start..n.span.end];
            let edits = match t_lang::rename::rename(&sources, &prog, &info, n.span, new) {
                Ok(edits) => edits,
                Err(why) => {
                    failures.push(format!("{name}: renaming `{old}` failed: {why}"));
                    continue;
                }
            };
            renamed += 1;
            let paths = sources.files().iter().map(|f| f.path.clone());
            let after: expect::Files = paths.zip(sources.edited(&edits)).collect();
            // Printed values and messages show the names of functions,
            // types, fields and variants.
            let shown = |run: t_lang::RunResult, renamed: bool| {
                let mut out = run.output;
                out.push(match run.result {
                    Ok(v) => v.to_string(),
                    Err(e) => e.message,
                });
                if renamed {
                    out.iter_mut().for_each(|l| *l = l.replace(new, old));
                }
                out
            };
            if shown(before.clone(), false) != shown(expect::run(&after), true) {
                let after: String = after.iter().map(|(_, src)| src.as_str()).collect();
                failures.push(format!(
                    "{name}: renaming `{old}` changed what it does:\n{after}"
                ));
            }
        }
    }
    assert!(renamed > 100, "only {renamed} renames");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
