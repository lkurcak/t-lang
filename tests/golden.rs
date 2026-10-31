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
/// and `tests/t/*/main.t`, or `tests/t/*/lib.t` for a library alone.
fn programs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter_map(|p| match p.is_dir() {
            true if !p.join("main.t").exists() => Some(p.join("lib.t")),
            true => Some(p.join("main.t")),
            false => p.extension().is_some_and(|ext| ext == "t").then_some(p),
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no tests found in {}", dir.display());
    paths
}

/// `f` of each of `paths`, in order, worked out on as many threads as there
/// are cores. Each thread takes the next program as it finishes one, so a
/// slow program holds up only itself.
fn on_all_cores<R: Send>(paths: &[PathBuf], f: impl Fn(&Path) -> R + Sync) -> Vec<R> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut results: Vec<(usize, R)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads.min(paths.len()))
            .map(|_| {
                s.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(i) else {
                            return done;
                        };
                        done.push((i, f(path)));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    results.sort_by_key(|&(i, _)| i);
    results.into_iter().map(|(_, r)| r).collect()
}

/// What a test is called: its file's name, or its directory's.
fn name(path: &Path) -> String {
    let path = match path
        .file_name()
        .is_some_and(|n| n == "main.t" || n == "lib.t")
    {
        true => path.parent().unwrap(),
        false => path,
    };
    path.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn golden() {
    let paths = programs();
    let failures: Vec<String> = on_all_cores(&paths, |path| {
        let name = name(path);
        let files = expect::files_of(path);
        let all: String = files.iter().map(|(_, src)| src.as_str()).collect();
        let want = Expectations::of(&all);
        if want.output.is_empty()
            && want.errors.is_empty()
            && want.error.is_none()
            && want.warnings.is_empty()
            && want.tests.is_empty()
        {
            return Some(format!(
                "{name}: has no `// expect:`, `// error:`, `// warning:` or `// test:` lines"
            ));
        }
        let failure = expect::check_files(&files).err()?;
        Some(format!("{name}: {failure}"))
    })
    .into_iter()
    .flatten()
    .collect();

    if !failures.is_empty() {
        panic!(
            "{} of {} golden tests failed:\n\n{}\n",
            failures.len(),
            paths.len(),
            failures.join("\n\n")
        );
    }
}

/// Every program in `examples/` runs, and prints what its `// expect:`
/// lines say.
#[test]
fn examples() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut failures = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "t") {
            continue;
        }
        let files = expect::files_of(&path);
        if let Err(failure) = expect::check_files(&files) {
            failures.push(format!("{}: {failure}", name(&path)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
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
            Ok(program) => without_std(&program.lower()),
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

/// What `t mono` prints of `exe` without the std library's functions,
/// structs and enums, the copies of others made for them, as
/// `<i64 as core::fmt::Display>::fmt::<std::io::Stdout>`, and what only they
/// use of the other libraries: what printing adds, the same in every
/// program, and not what a `.mono` file is there to show.
fn without_std(exe: &t_lang::mono::Executable) -> String {
    use std::collections::HashSet;
    let program = exe.program();
    let library = |name: &str| {
        let name = name.strip_prefix('<').unwrap_or(name);
        [
            "core::", "alloc::", "std::", "[", "str", "char", "bool", "i", "u",
        ]
        .iter()
        .any(|p| name.starts_with(p))
            && !name.starts_with("impl")
    };
    let names = (program.types.iter().map(|d| d.name().name.clone()))
        .chain(program.fns.iter().map(|f| f.name.name.clone()));
    let mut kept: HashSet<String> = HashSet::new();
    let mut left: Vec<String> = Vec::new();
    for name in names {
        match library(&name) {
            true if !name.contains("std::") => left.push(name),
            true => {}
            false => {
                kept.insert(name);
            }
        }
    }
    // What the program refers to, and what that does, and so on.
    loop {
        let text = exe.render_only(&|n| kept.contains(n));
        // A name that's part of a longer one keeps more than it needs to,
        // which only makes the file longer.
        let (now, later): (Vec<_>, Vec<_>) = left.into_iter().partition(|n| text.contains(n));
        left = later;
        if now.is_empty() {
            return text;
        }
        kept.extend(now);
    }
}

/// Every node of a program has an id of its own (`ast::NodeId`), by which
/// the checker files what it learns about it, so two nodes with the same id
/// must be one node, copied: of the same kind, at the same place. So must
/// every id be one the program's counter has handed out. Checked once the
/// checker has rewritten the program (paths resolved, `for` loops and
/// ranges written out), in programs that check and programs that don't.
#[test]
fn node_ids_are_unique() {
    let results = on_all_cores(&programs(), |path| {
        let files = expect::files_of(path);
        let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
            Some((_, src)) => Ok(src.clone()),
            None => t_lang::no_files(p),
        };
        let (_, prog) = t_lang::load::load(path, &files[0].1, &read);
        let Ok(mut prog) = prog else {
            return Ok(0);
        };
        let _ = t_lang::check::check(&mut prog, &mut Vec::new());
        let why = |why| format!("{}: {why}", name(path));
        node_ids(&format!("{prog:?}"), prog.next_id).map_err(why)
    });
    let nodes: usize = results.iter().flatten().sum();
    let failures: Vec<&str> = results
        .iter()
        .filter_map(|r| r.as_ref().err().map(String::as_str))
        .collect();
    assert!(nodes > 100_000, "only {nodes} nodes");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Reads the nodes off `debug`, a program's derived `Debug`, which shows
/// every field: each `id: NodeId(n)`,
/// with the struct it's in and that struct's span, if it has one before
/// its id. How many there are, or two different ones with one id.
fn node_ids(debug: &str, next_id: u32) -> Result<usize, String> {
    use std::collections::HashMap;
    const SPAN: &str = "span: Span { start: ";
    const ID: &str = "id: NodeId(";
    let number = |s: &str| -> (u32, usize) {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap();
        (s[..end].parse().unwrap(), end)
    };
    // A struct, by name, with its span.
    type Node<'a> = (&'a str, Option<(u32, u32)>);
    // The structs the scan is in, innermost last.
    let mut stack: Vec<Node> = Vec::new();
    let mut seen: HashMap<u32, Node> = HashMap::new();
    let mut count = 0;
    let mut i = 0;
    while i < debug.len() {
        let rest = &debug[i..];
        if rest.starts_with('"') {
            // A name or a doc comment's text, which may hold anything.
            let mut chars = rest.char_indices().skip(1);
            while let Some((k, c)) = chars.next() {
                match c {
                    '\\' => {
                        chars.next();
                    }
                    '"' => {
                        i += k;
                        break;
                    }
                    _ => {}
                }
            }
        } else if let Some(span) = rest.strip_prefix(SPAN) {
            let (start, n) = number(span);
            let (end, m) = number(&span[n + ", end: ".len()..]);
            stack.last_mut().expect("in a struct").1 = Some((start, end));
            // Past the span's `}`.
            i += SPAN.len() + n + ", end: ".len() + m + " ".len();
        } else if let Some(id) = rest.strip_prefix(ID) {
            let (id, n) = number(id);
            let node = *stack.last().expect("in a struct");
            if id >= next_id {
                return Err(format!(
                    "`{}` has id {id}, but only {next_id} were made",
                    node.0
                ));
            }
            if let Some(&other) = seen.get(&id)
                && other != node
            {
                return Err(format!("{node:?} and {other:?} have the same id, {id}"));
            }
            seen.insert(id, node);
            count += 1;
            i += ID.len() + n;
        } else if rest.starts_with('{') {
            let before = debug[..i].trim_end();
            let start = before
                .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                .map_or(0, |k| k + 1);
            stack.push((&before[start..], None));
        } else if rest.starts_with('}') {
            stack.pop();
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    Ok(count)
}

/// Renaming anything declared in a program to a name nothing else has is
/// accepted, and leaves a program that does the same. So every name for
/// it is found, and nothing else is touched.
#[test]
fn renaming_keeps_meaning() {
    // Every rename checks a program again, the core library and all.
    let results = on_all_cores(&programs(), renames_in);
    let failures: Vec<String> = results.iter().flat_map(|(f, _)| f.clone()).collect();
    let renamed: usize = results.iter().map(|(_, n)| n).sum();
    assert!(renamed > 100, "only {renamed} renames");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Renames each thing declared in the program at `path` in turn: what went
/// wrong, and how many renames there were.
fn renames_in(path: &Path) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut renamed = 0;
    let name = name(path);
    let files = expect::files_of(path);
    let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
        Some((_, src)) => Ok(src.clone()),
        None => t_lang::no_files(p),
    };
    let Ok((sources, prog, info)) = t_lang::rename::checked(path, &files[0].1, &read) else {
        return (failures, renamed);
    };
    let src = sources.text();
    let before = expect::run(&files);
    let mut defs: Vec<_> = info
        .notes
        .iter()
        .filter(|n| n.def == Some(n.span) && !sources.in_others(n.span.start))
        .collect();
    defs.sort_by_key(|n| n.span.start);
    defs.dedup_by_key(|n| n.span);
    for n in defs {
        if t_lang::rename::target(&sources, &prog, &info, n.span.start).is_err() {
            continue;
        }
        let new = match n.kind {
            // An effect parameter too: its kind is declared, so the case of
            // its name doesn't matter.
            NoteKind::Struct(_) | NoteKind::Enum(_) | NoteKind::TypeParam(_) => "Renamed",
            NoteKind::EffectParam(_) => "Renamed",
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
        let paths = sources.program_files().iter().map(|f| f.path.clone());
        let mut after: expect::Files = paths.zip(sources.edited(&edits)).collect();
        // Its `t.toml` and dependencies, which a rename leaves as they are.
        let others = (sources.files().iter())
            .filter(|f| !f.is_library() && f.kind != t_lang::source::Kind::Program);
        after.extend(others.map(|f| (f.path.clone(), sources.src(f).to_string())));
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
    (failures, renamed)
}

/// A package checks the same on its own as it does in any program that
/// depends on it (lemma P2 of docs/26-packages.md): each dependency of
/// each program in `tests/t` checks alone, unless the program's error is
/// in it, and then alone it has the same one.
#[test]
fn dependencies_check_as_they_do_alone() {
    use t_lang::source::Kind;
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in programs() {
        let files = expect::files_of(&path);
        if !files.iter().any(|(p, _)| p.ends_with("t.toml")) {
            continue;
        }
        let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
            Some((_, src)) => Ok(src.clone()),
            None => t_lang::no_files(p),
        };
        let program = t_lang::check_files(&path, &files[0].1, &read);
        let sources = &program.sources;
        let firsts = (sources.files().iter())
            .filter(|f| f.kind == Kind::Dependency && f.path.ends_with("lib.t"))
            .filter(|f| f.declared.is_none());
        for first in firsts {
            checked += 1;
            let alone = t_lang::check_files(&first.path, sources.src(first), &read);
            let at = |c: &t_lang::Checked| {
                (c.result.as_ref().err()).map(|e| {
                    (
                        c.sources.file_at(e.span.start).path.clone(),
                        e.message.clone(),
                    )
                })
            };
            let (theirs, mine) = (at(&program), at(&alone));
            let dir = first.path.parent().unwrap();
            let in_it = theirs.as_ref().filter(|(p, _)| p.starts_with(dir));
            if in_it.cloned() != mine {
                failures.push(format!(
                    "{}: {} gives {mine:?} alone, but {theirs:?} in the program",
                    name(&path),
                    first.path.display()
                ));
            }
        }
    }
    assert!(checked >= 10, "only {checked} dependencies");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
