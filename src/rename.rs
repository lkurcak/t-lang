//! Renaming a variable, function, type, field, variant or type parameter
//! everywhere it's named, for `t lsp` and for the fixes of the naming lints
//! (see `naming.rs`).
//!
//! What gets renamed follows what names refer to (`Info::occurrences`), not
//! how they're spelled. A rename that would change what any name refers to
//! is refused: the renamed program is checked again, and every name in it
//! must refer to what it did before. That catches the new name shadowing
//! something or being shadowed, a nested `fn` capturing a local it now
//! names, and two things of the same name in one scope.

use crate::ast::*;
use crate::check::{self, Info, Note, NoteKind};
use crate::error::{Edit, Span};
use crate::lexer::{self, Tok};
use crate::load::{self, Read};
use crate::source::Sources;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Parses and checks the program that starts at `path`, whose text is
/// `src`, for renaming in it. Only a program that checks can be renamed
/// in: one that doesn't may have names nothing knows the meaning of yet.
pub fn checked(path: &Path, src: &str, read: Read) -> Result<(Sources, Program, Info), String> {
    let (sources, prog) = load::load(path, src, read);
    let mut prog = prog.map_err(|_| "the program has a syntax error; fix it first".to_string())?;
    let (info, result) = check::check(&mut prog, &mut Vec::new());
    match result {
        Ok(()) => Ok((sources, prog, info)),
        Err(_) => Err("the program has a type error; fix it first".to_string()),
    }
}

/// The program `sources` are, with its own files' texts replaced by
/// `texts`, in the same order, parsed again, with its `t.toml` and its
/// dependencies as they were.
fn reparsed(
    sources: &Sources,
    texts: &[String],
) -> (Sources, Result<Program, crate::error::Error>) {
    let files = sources.program_files();
    let read = |path: &Path| match files.iter().position(|f| f.path == path) {
        Some(i) => Ok(texts[i].clone()),
        None => match sources
            .files()
            .iter()
            .find(|f| f.path == path && !f.is_library())
        {
            Some(f) => Ok(sources.src(f).to_string()),
            None => crate::no_files(path),
        },
    };
    load::load(&files[0].path, &texts[0], &read)
}

/// What renaming at `offset` renames, as the note there, or why nothing
/// can be renamed there.
pub fn target<'i>(
    sources: &Sources,
    prog: &Program,
    info: &'i Info,
    offset: usize,
) -> Result<&'i Note, String> {
    let Some(note) = info.note_at(offset) else {
        return Err(match word_at(sources.text(), offset) {
            name if crate::types::IntTy::named(name).is_some() || name == "bool" => {
                format!("`{name}` is a built-in type")
            }
            _ => "there's nothing here to rename".to_string(),
        });
    };
    let Some(def) = note.def else {
        return Err(match &note.kind {
            NoteKind::Trait(t) => format!("`{t}` is a built-in trait"),
            kind => format!("`{}` is built in", name(kind)),
        });
    };
    if sources.in_library(def.start) || sources.in_library(offset) {
        return Err(format!(
            "`{}` is a built-in library's, which can't be changed",
            word_at(sources.text(), def.start)
        ));
    }
    if sources.in_others(def.start) || sources.in_others(offset) {
        return Err(format!(
            "`{}` is a package's this one depends on: rename it there",
            word_at(sources.text(), def.start)
        ));
    }
    if let NoteKind::Module(m) = &note.kind {
        return Err(format!(
            "`{m}` is a module, and modules can't be renamed yet"
        ));
    }
    let indexes = |name: &str| name == "index" || name == "index_mut";
    if let NoteKind::Fn(name) = &note.kind
        && indexes(name)
        && (prog.impls.iter())
            .flat_map(|d| &d.methods)
            .any(|f| f.name.span == def)
    {
        return Err(format!("`{name}` can't be renamed: it's what `x[i]` calls"));
    }
    if matches!(&note.kind, NoteKind::Var(n) if n == "self") {
        return Err("`self` can't be renamed: it's what a method's receiver is called".into());
    }
    let main = prog
        .fns
        .iter()
        .find(|f| f.module == ROOT && f.name.name == "main");
    if main.is_some_and(|m| m.name.span == def) {
        return Err("`main` can't be renamed: it's where the program starts".to_string());
    }
    Ok(note)
}

/// The identifier `offset` is in or just after, if any.
fn word_at(src: &str, offset: usize) -> &str {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let start = src[..offset].rfind(|c| !is_word(c)).map_or(0, |i| i + 1);
    let end = src[offset..]
        .find(|c| !is_word(c))
        .map_or(src.len(), |i| offset + i);
    &src[start..end]
}

/// The edits that rename what's declared at `def` to `new`, in `sources`,
/// which parse to `prog` and check, as `info` says. Every name for it changes,
/// and nothing else, comments included. In a shorthand `P { x }`, renaming
/// the variable gives `P { x: new }` and renaming the field `P { new: x }`.
pub fn rename(
    sources: &Sources,
    prog: &Program,
    info: &Info,
    def: Span,
    new: &str,
) -> Result<Vec<Edit>, String> {
    // Its declaration: in a shorthand pattern `P { x }`, the variable, as
    // the field is declared elsewhere.
    let decl = info
        .notes
        .iter()
        .find(|n| n.span == def && n.def == Some(def))
        .ok_or("there's nothing here to rename")?;
    if let NoteKind::Module(m) = &decl.kind {
        return Err(format!(
            "`{m}` is a module, and modules can't be renamed yet"
        ));
    }
    let old = name(&decl.kind);
    let is_type = matches!(
        decl.kind,
        NoteKind::Struct(_) | NoteKind::Enum(_) | NoteKind::TypeParam(_)
    );
    valid(new, is_type)?;
    if matches!(decl.kind, NoteKind::EffectParam(_)) && ["panic", "alloc"].contains(&new) {
        return Err(format!("`{new}` is a built-in effect"));
    }
    if new == old {
        return Ok(Vec::new());
    }

    let is_field = matches!(decl.kind, NoteKind::Field(_));
    let shorthands = shorthands(prog);
    let renamed: Vec<Renamed> = info
        .occurrences(def)
        .into_iter()
        // A use through another name, given by `use .. as`, keeps it.
        .filter(|span| sources.text().get(span.start..span.end) == Some(old))
        .map(|span| match (shorthands.contains(&span), is_field) {
            (false, _) => Renamed {
                edit: Edit::replace(span, new),
                field: (0, new.len()),
                var: (0, new.len()),
            },
            (true, true) => Renamed {
                edit: Edit::replace(span, format!("{new}: {old}")),
                field: (0, new.len()),
                var: (new.len() + 2, old.len()),
            },
            (true, false) => Renamed {
                edit: Edit::replace(span, format!("{old}: {new}")),
                field: (0, old.len()),
                var: (old.len() + 2, new.len()),
            },
        })
        .collect();
    let edits: Vec<Edit> = renamed.iter().map(|r| r.edit.clone()).collect();
    let cant = |why: String| format!("can't rename `{old}` to `{new}`: {why}");
    let (sources_after, prog_after) = reparsed(sources, &sources.edited(&edits));
    let mut prog_after = prog_after.map_err(|e| cant(e.message))?;
    let after = sources_after.text();
    let line = |offset: usize| sources_after.line(offset);
    let (info_after, result) = check::check(&mut prog_after, &mut Vec::new());
    if let Err(e) = result {
        let line = line(e.span.start);
        return Err(cant(format!(
            "line {line} would be an error: {}",
            e.message
        )));
    }

    // Every name must refer to what it did. Each is looked for where it is
    // after the rename, and as a variable or not, since a shorthand names a
    // field and a variable in one place.
    let defs_after: HashMap<(Span, bool), Option<Span>> = info_after
        .notes
        .iter()
        .map(|n| ((n.span, is_var(n)), n.def))
        .collect();
    let target = moved(&renamed, def, is_var(decl));
    let describe = |def: Option<Span>, name: &str| match def {
        None => format!("the built-in `{name}`"),
        Some(d) => {
            let what = info_after
                .notes
                .iter()
                .find(|n| n.span == d && n.def == Some(d))
                .map_or("definition", |n| what(&n.kind));
            format!(
                "the {what} `{}` on line {}",
                &after[d.start..d.end],
                line(d.start)
            )
        }
    };
    for n in &info.notes {
        let var = is_var(n);
        let at = moved(&renamed, n.span, var);
        let was = n.def.map(|d| moved(&renamed, d, var));
        let now = defs_after.get(&(at, var)).copied();
        if now == Some(was) {
            continue;
        }
        let name = &after[at.start..at.end];
        let line = line(at.start);
        let why = match now {
            None => format!("`{name}` on line {line} would no longer mean anything"),
            Some(now) if was == Some(target) => format!(
                "`{name}` on line {line} would refer to {} instead",
                describe(now, name)
            ),
            Some(now) if now == Some(target) => format!(
                "it would shadow {}, used on line {line}",
                describe(was, name)
            ),
            Some(now) => format!(
                "`{name}` on line {line} would refer to {} instead of {}",
                describe(now, name),
                describe(was, name)
            ),
        };
        return Err(cant(why));
    }
    Ok(edits)
}

/// One edit of a rename, with where the field and the variable it names
/// end up in its text, as offset and length: in a shorthand they're in
/// different places.
struct Renamed {
    edit: Edit,
    field: (usize, usize),
    var: (usize, usize),
}

/// Where the name at `span` ends up once `renamed` is done, as a variable
/// or not.
fn moved(renamed: &[Renamed], span: Span, var: bool) -> Span {
    let mut shift = 0isize;
    let shifted = |at: usize, by: isize| at.checked_add_signed(by).expect("in the source");
    for r in renamed {
        let e = &r.edit;
        if e.span == span {
            let (offset, len) = if var { r.var } else { r.field };
            let start = shifted(span.start, shift) + offset;
            return Span::new(start, start + len);
        }
        if e.span.end <= span.start {
            shift += e.text.len() as isize - (e.span.end - e.span.start) as isize;
        }
    }
    Span::new(shifted(span.start, shift), shifted(span.end, shift))
}

fn is_var(n: &Note) -> bool {
    matches!(n.kind, NoteKind::Var(_))
}

/// Is `name` a name: an identifier other than `_`, and not a keyword?
pub fn is_name(name: &str) -> bool {
    valid(name, false).is_ok()
}

/// Is `new` something a name can be changed to? A type's can't be `i64` or
/// `bool`.
fn valid(new: &str, is_type: bool) -> Result<(), String> {
    match lexer::lex(new).as_deref() {
        Ok([t, eof]) if eof.tok == Tok::Eof && t.span == Span::new(0, new.len()) => match &t.tok {
            Tok::Ident(name) if name == "_" => Err("`_` can't be a name".to_string()),
            Tok::Ident(name)
                if is_type && (crate::types::IntTy::named(name).is_some() || name == "bool") =>
            {
                Err(format!("`{name}` is a built-in type"))
            }
            Tok::Ident(_) => Ok(()),
            Tok::Int(..) => Err(format!("`{new}` isn't a name")),
            _ if new.chars().all(|c| c.is_ascii_alphabetic()) => {
                Err(format!("`{new}` is a keyword"))
            }
            _ => Err(format!("`{new}` isn't a name")),
        },
        _ => Err(format!("`{new}` isn't a name")),
    }
}

/// The name a note is for, as written: the variant's own for a variant.
fn name(kind: &NoteKind) -> &str {
    match kind {
        NoteKind::Var(n)
        | NoteKind::Fn(n)
        | NoteKind::Trait(n)
        | NoteKind::Struct(n)
        | NoteKind::Enum(n)
        | NoteKind::TypeParam(n)
        | NoteKind::EffectParam(n)
        | NoteKind::ConstParam(n)
        | NoteKind::Module(n)
        | NoteKind::Field(n)
        | NoteKind::Variant(_, n)
        | NoteKind::Alias(n) => n,
    }
}

/// What kind of thing a note is for, in words.
fn what(kind: &NoteKind) -> &'static str {
    match kind {
        NoteKind::Var(_) => "variable",
        NoteKind::Fn(_) => "function",
        NoteKind::Trait(_) => "trait",
        NoteKind::Struct(_) => "struct",
        NoteKind::Enum(_) => "enum",
        NoteKind::TypeParam(_) => "type parameter",
        NoteKind::EffectParam(_) => "effect parameter",
        NoteKind::ConstParam(_) => "const parameter",
        NoteKind::Module(_) => "module",
        NoteKind::Field(_) => "field",
        NoteKind::Variant(..) => "variant",
        NoteKind::Alias(_) => "type alias",
    }
}

/// Where a struct literal or pattern in `prog` uses the shorthand `P { x }`,
/// for `P { x: x }`: the span of each such `x`.
fn shorthands(prog: &Program) -> HashSet<Span> {
    let mut out = HashSet::new();
    for (f, ..) in prog.all_fns() {
        shorthand_exprs(&f.body, &mut out);
    }
    out
}

fn shorthand_exprs(e: &Expr, out: &mut HashSet<Span>) {
    match &e.kind {
        ExprKind::StructLit(_, _, inits) => {
            out.extend(
                inits
                    .iter()
                    .filter(|i| i.value.span == i.name.span)
                    .map(|i| i.name.span),
            );
        }
        ExprKind::Block(stmts, _) => {
            for stmt in stmts {
                if let Stmt::Let { pat, .. } = stmt {
                    shorthand_pats(pat, out);
                }
            }
        }
        ExprKind::Match(_, arms) => arms.iter().for_each(|a| shorthand_pats(&a.pat, out)),
        _ => {}
    }
    for c in e.children() {
        shorthand_exprs(c, out);
    }
}

fn shorthand_pats(p: &Pattern, out: &mut HashSet<Span>) {
    match &p.kind {
        PatternKind::Struct(_, _, fields, _) => {
            for f in fields {
                if f.pat.span == f.name.span {
                    out.insert(f.name.span);
                }
                shorthand_pats(&f.pat, out);
            }
        }
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) | PatternKind::Array(pats) => {
            pats.iter().for_each(|p| shorthand_pats(p, out));
        }
        PatternKind::Wild
        | PatternKind::Bind(..)
        | PatternKind::Int(..)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}
