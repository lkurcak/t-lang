//! The C backend: turns the program the interpreter runs into one C11
//! file, which a C compiler makes a native program of (`t build`). See
//! docs/33-c-backend.md.
//!
//! What it translates is exactly what the interpreter runs: the copies of
//! each function's MIR that monomorphisation made, with the drops drop
//! elaboration worked out and the glue monomorphisation wrote (B1 of the
//! backend design). Each statement becomes the C that does what the
//! interpreter does for it, in the same order; nothing is lowered a second
//! way. So where the two could disagree is only in what one statement
//! does, which the differential tests check, by running every program of
//! the test corpus both ways (`tests/compiled.rs`).
//!
//! The C it writes has no undefined behaviour, whatever the program does
//! (C2): integers are computed in 64-bit types and checked before they
//! could overflow, or for what the checker proved can't, computed so that
//! they wrap, as a proof that's wrong must not become undefined behaviour
//! (C3); a union's member is read only once its tag says it's there;
//! pointers are only ever to the type they point to; every local starts
//! zeroed, so nothing reads what was never written. What only the
//! interpreter checks, refinements, effects, leaks and what the checker
//! promises can't happen, the checker proved, and isn't compiled (C4).
//! Breaking an `unsafe` block's obligations is undefined behaviour here,
//! as it is what `unsafe` means: `t run` is the checking mode.
//!
//! How values are laid out is unspecified (decided with the backend
//! design): for now, a struct is a C struct of its fields, in order, an
//! enum a tag and a union of its variants' fields, a reference or box a
//! pointer, a slice or `&str` a pointer and a length, and a type that
//! holds nothing a byte.

use crate::ast::{BinOp, FnDecl, TypeExprKind, UnOp};
use crate::error::{Error, Span};
use crate::mir::*;
use crate::mono::{Executable, Shape, glue_name};
use crate::source::Sources;
use crate::types::{IntTy, Region, Type};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// The runtime every compiled program starts with.
const SHIM: &str = include_str!("cgen/shim.h");

/// How to write the C.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// With `#line` directives, so that a debugger and the C compiler's
    /// messages show the T source each statement comes from.
    pub lines: bool,
}

/// The C program for `exe`, whose files are `sources`.
pub fn emit(exe: &Executable, sources: &Sources, options: Options) -> String {
    let mut g = Gen::new(exe, sources, options);
    g.run();
    g.finish()
}

/// The C name of each integer type, and the suffix of the runtime's
/// helpers for it.
fn c_int(t: IntTy) -> &'static str {
    match t {
        IntTy::I8 => "int8_t",
        IntTy::I16 => "int16_t",
        IntTy::I32 => "int32_t",
        IntTy::I64 | IntTy::Isize => "int64_t",
        IntTy::U8 => "uint8_t",
        IntTy::U16 => "uint16_t",
        IntTy::U32 => "uint32_t",
        IntTy::U64 | IntTy::Usize => "uint64_t",
    }
}

fn suffix(t: IntTy) -> &'static str {
    match t {
        IntTy::Isize => "i64",
        IntTy::Usize => "u64",
        t => t.name(),
    }
}

/// `n`, of type `t`, as a C expression of `t`'s C type.
fn int_lit(n: i128, t: IntTy) -> String {
    let ty = c_int(t);
    match t.signed() {
        true if n == i128::from(i64::MIN) => format!("(({ty})INT64_MIN)"),
        true => format!("(({ty})INT64_C({n}))"),
        false => format!("(({ty})UINT64_C({n}))"),
    }
}

/// `s` as a C string literal.
fn c_str(s: &[u8]) -> String {
    let mut out = String::from("\"");
    for &b in s {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'?' => out.push_str("\\?"),
            0x20..=0x7e => out.push(b as char),
            b => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out.push('"');
    out
}

/// A C identifier for what's called `name` in T: its words, without the
/// modules of its paths, `<core::option::Option::<i64>>::unwrap` as
/// `Option_i64_unwrap`.
fn sanitize(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !(chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();
        // A module on a path, followed by `::` and more of the path.
        let module = chars.get(i) == Some(&':')
            && chars.get(i + 1) == Some(&':')
            && chars
                .get(i + 2)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_');
        if !module {
            words.push(word);
        }
    }
    let mut s = words.join("_");
    s.truncate(48);
    s
}

/// How the body of a closure takes the closure: `Some(Some(m))` by
/// reference, `&mut` if `m`; `Some(None)` by value; `None` not at all.
fn env_by(body: &Body) -> Option<Option<bool>> {
    let env = body.locals.get(1).filter(|l| l.kind == LocalKind::Env)?;
    Some(match &env.ty {
        Type::Ref(_, m, _) => Some(*m),
        _ => None,
    })
}

/// The copy of a function that closure `code` is in.
fn closure_root(code: &str) -> &str {
    code.split("::{closure#").next().unwrap_or(code)
}

/// What tells C structs apart: a type's symbols, which leave out its
/// regions, and for a slice, that it's the reference to one.
fn key(t: &Type) -> String {
    match t {
        Type::Slice(e) => format!("&[{}]", e.symbols()),
        t => t.symbols(),
    }
}

fn unsized_ty(t: &Type) -> bool {
    matches!(t, Type::Slice(_) | Type::Str)
}

/// A reference to `t`.
fn ref_to(t: Type) -> Type {
    Type::Ref(Region::Erased, false, Box::new(t))
}

struct Gen<'e> {
    exe: &'e Executable,
    sources: &'e Sources,
    options: Options,
    decls: HashMap<&'e str, &'e FnDecl>,
    /// The C name of each type that's a C struct, by its type's symbols
    /// (`key`), which leave out its regions (a slice's is the pointer and
    /// length a reference to one is).
    tys: HashMap<String, String>,
    ty_order: Vec<Type>,
    /// Function pointer types, by their C signature.
    fn_ptrs: HashMap<String, String>,
    fn_ptr_defs: String,
    /// Every C identifier at file scope taken so far.
    taken: HashSet<String>,
    /// The C name of each function, closure and glue body, and of each
    /// intrinsic's wrapper, by its name in T.
    fn_names: HashMap<String, String>,
    /// Bodies still to write.
    todo: Vec<String>,
    protos: String,
    defs: String,
    lits: HashMap<Vec<u8>, String>,
    lit_defs: String,
    sites: HashMap<Span, usize>,
    site_texts: Vec<String>,
}

/// The body being written: its name and MIR.
struct Cx<'b> {
    name: &'b str,
    body: &'b Body,
}

impl<'e> Gen<'e> {
    fn new(exe: &'e Executable, sources: &'e Sources, options: Options) -> Gen<'e> {
        let decls = (exe.program().fns.iter())
            .map(|f| (f.name.name.as_str(), f))
            .collect();
        // The runtime's names are taken.
        let taken = (SHIM.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')))
            .filter(|w| w.starts_with("t_") || w.starts_with("T_"))
            .map(str::to_string)
            .collect();
        Gen {
            exe,
            sources,
            options,
            decls,
            tys: HashMap::new(),
            ty_order: Vec::new(),
            fn_ptrs: HashMap::new(),
            fn_ptr_defs: String::new(),
            taken,
            fn_names: HashMap::new(),
            todo: Vec::new(),
            protos: String::new(),
            defs: String::new(),
            lits: HashMap::new(),
            lit_defs: String::new(),
            sites: HashMap::new(),
            site_texts: Vec::new(),
        }
    }

    /// A C identifier at file scope like `base`, not taken yet.
    fn fresh(&mut self, base: &str) -> String {
        let base = match base.is_empty() {
            true => "x".to_string(),
            false => base.to_string(),
        };
        let mut name = base.clone();
        let mut n = 1;
        while !self.taken.insert(name.clone()) {
            n += 1;
            name = format!("{base}_{n}");
        }
        name
    }

    // Types.

    /// The C type of a value of `t`.
    fn cty(&mut self, t: &Type) -> String {
        let t = t.erased();
        match &t {
            Type::Int(i) => c_int(*i).to_string(),
            Type::Bool => "bool".to_string(),
            Type::Char => "uint32_t".to_string(),
            Type::Unit | Type::Never => "t_unit".to_string(),
            Type::Ref(_, _, inner) | Type::Box(inner) => match &**inner {
                Type::Slice(_) => self.named(&inner.erased()),
                Type::Str => self.named(&Type::Slice(Box::new(Type::Int(IntTy::U8)))),
                inner => format!("{}*", self.cty(inner)),
            },
            Type::Fn(sig) => {
                let ret = self.cty(&sig.ret);
                let params: Vec<String> = sig.params.iter().map(|p| self.cty(p)).collect();
                let params = match params.is_empty() {
                    true => "void".to_string(),
                    false => params.join(", "),
                };
                let key = format!("{ret} (*)({params})");
                if let Some(name) = self.fn_ptrs.get(&key) {
                    return name.clone();
                }
                let name = self.fresh(&format!("t_fn{}", self.fn_ptrs.len()));
                let _ = writeln!(self.fn_ptr_defs, "typedef {ret} (*{name})({params});");
                self.fn_ptrs.insert(key, name.clone());
                name
            }
            Type::Array(..) | Type::Adt(..) => self.named(&t),
            t => panic!("no C type for `{t}`"),
        }
    }

    /// The C struct for `t`: an ADT, closure, array or (as what a
    /// reference to one is) slice.
    fn named(&mut self, t: &Type) -> String {
        if let Some(name) = self.tys.get(&key(t)) {
            return name.clone();
        }
        let base = match t {
            Type::Slice(e) => format!("T_slice_{}", sanitize(&e.symbols())),
            t => format!("T_{}", sanitize(&t.symbols())),
        };
        let name = self.fresh(&base);
        self.tys.insert(key(t), name.clone());
        self.ty_order.push(t.clone());
        name
    }

    fn shape(&self, t: &Type) -> &'e Shape {
        let exe: &'e Executable = self.exe;
        (exe.shape(t)).unwrap_or_else(|| panic!("no shape for `{t}`"))
    }

    /// The types of the fields of struct or closure `t`.
    fn fields(&self, t: &Type) -> Vec<Type> {
        match self.shape(t) {
            Shape::Struct(fs) | Shape::Closure(fs) => fs.iter().map(Type::erased).collect(),
            s => panic!("fields of `{t}`, a {s:?}"),
        }
    }

    /// The variants of enum `t`: each one's name and its fields' types.
    fn variants(&self, t: &Type) -> Vec<(String, Vec<Type>)> {
        match self.shape(t) {
            Shape::Enum(vs) => (vs.iter())
                .map(|(n, fs)| (n.clone(), fs.iter().map(Type::erased).collect()))
                .collect(),
            s => panic!("variants of `{t}`, a {s:?}"),
        }
    }

    /// The index of variant `name` of enum `t`.
    fn variant(&self, t: &Type, name: &str) -> usize {
        let vs = self.variants(t);
        (vs.iter().position(|(n, _)| n == name))
            .unwrap_or_else(|| panic!("no variant `{name}` of `{t}`"))
    }

    /// The definition of C struct `t` (see `named`), and the types it
    /// holds by value, which must be defined first.
    fn define(&mut self, t: &Type) -> (String, Vec<Type>) {
        let name = self.tys[&key(t)].clone();
        let mut deps = Vec::new();
        let mut members = String::new();
        let mut member = |g: &mut Gen, ty: &Type, field: &str, out: &mut String| {
            let c = g.cty(ty);
            // A reference to a slice holds the C struct of one.
            deps.push(match ty.erased() {
                Type::Ref(_, _, inner) | Type::Box(inner) => match *inner {
                    Type::Slice(e) => Type::Slice(e),
                    Type::Str => Type::Slice(Box::new(Type::Int(IntTy::U8))),
                    inner => Type::Ref(Region::Erased, false, Box::new(inner)),
                },
                t => t,
            });
            let _ = write!(out, " {c} {field};");
        };
        match t {
            Type::Slice(e) => {
                let e = self.cty(e);
                let _ = write!(members, " {e} *ptr; uint64_t len;");
            }
            Type::Array(e, n) => match **n {
                Type::Nat(0) => members.push_str(" uint8_t _;"),
                Type::Nat(n) => {
                    let c = self.cty(e);
                    deps.push(e.erased());
                    let _ = write!(members, " {c} a[{n}];");
                }
                ref n => panic!("an array of length `{n}`"),
            },
            Type::Adt(symbol, _, args) => match self.shape(t) {
                Shape::Struct(_) | Shape::Closure(_) => {
                    let fs = self.fields(t);
                    if fs.is_empty() {
                        members.push_str(" uint8_t _;");
                    }
                    for (i, f) in fs.iter().enumerate() {
                        member(self, f, &format!("f{i}"), &mut members);
                    }
                }
                Shape::Enum(_) => {
                    members.push_str(" uint32_t tag;");
                    let vs = self.variants(t);
                    let mut union = String::new();
                    for (v, (_, fs)) in vs.iter().enumerate() {
                        if fs.is_empty() {
                            continue;
                        }
                        let mut inner = String::new();
                        for (i, f) in fs.iter().enumerate() {
                            member(self, f, &format!("f{i}"), &mut inner);
                        }
                        let _ = write!(union, " struct {{{inner} }} v{v};");
                    }
                    if !union.is_empty() {
                        let _ = write!(members, " union {{{union} }} u;");
                    }
                }
                Shape::Intrinsic => match symbol.as_str() {
                    "alloc::raw::RawBuf" => {
                        let e = self.cty(&args[0]);
                        let _ = write!(members, " {e} *ptr; uint64_t cap;");
                    }
                    crate::check::ARENA => {
                        members.push_str(" void **items; uint64_t len; uint64_t cap;");
                    }
                    _ => members.push_str(" uint8_t _;"),
                },
            },
            t => panic!("no C struct for `{t}`"),
        }
        let t_name = t.to_string().replace("*/", "* /");
        (
            format!("struct {name} {{{members} }}; /* {t_name} */\n"),
            deps,
        )
    }

    // Places and operands.

    fn local_ty(cx: &Cx, l: Local) -> Type {
        cx.body.locals[l.0].ty.erased()
    }

    /// The C lvalue of `place`, and its type. A place behind a reference
    /// to a slice or `str` is the reference, as that's how C holds it.
    fn place(&mut self, cx: &Cx, place: &Place) -> (String, Type) {
        let mut e = format!("_{}", place.local.0);
        let mut t = Self::local_ty(cx, place.local);
        for p in &place.proj {
            (e, t) = match (p, &t) {
                (Proj::Deref, Type::Ref(_, _, inner) | Type::Box(inner)) => {
                    let inner = inner.erased();
                    match unsized_ty(&inner) {
                        true => (e, inner),
                        false => (format!("(*{e})"), inner),
                    }
                }
                (Proj::Field(i), Type::Array(elem, _)) => (format!("{e}.a[{i}]"), elem.erased()),
                (Proj::Field(i), _) => {
                    let f = self.fields(&t)[*i].clone();
                    (format!("{e}.f{i}"), f)
                }
                (Proj::Variant(v, i), _) => {
                    let f = self.variants(&t)[*v].1[*i].clone();
                    (format!("{e}.u.v{v}.f{i}"), f)
                }
                (p, t) => panic!("{p:?} of `{t}`"),
            };
        }
        (e, t)
    }

    fn place_ty(&mut self, cx: &Cx, place: &Place) -> Type {
        self.place(cx, place).1
    }

    /// The type of what operand `o` gives.
    fn operand_ty(&mut self, cx: &Cx, o: &Operand) -> Type {
        match o {
            Operand::Copy(p) | Operand::Move(p) => self.place_ty(cx, p),
            Operand::Const(c) => match c {
                Const::Int(_, t) => t.erased(),
                Const::Bool(_) => Type::Bool,
                Const::Char(_) => Type::Char,
                Const::Unit => Type::Unit,
                Const::Str(_) => ref_to(Type::Str),
                Const::Bytes(_) => ref_to(Type::Slice(Box::new(Type::Int(IntTy::U8)))),
                c => panic!("the type of `{c:?}`"),
            },
        }
    }

    /// The C expression of what operand `o` gives: a copy of what's at
    /// its place (moving leaves the bytes there, which nothing reads).
    fn operand(&mut self, cx: &Cx, o: &Operand) -> String {
        match o {
            Operand::Copy(p) | Operand::Move(p) => self.place(cx, p).0,
            Operand::Const(c) => match c {
                Const::Int(n, Type::Int(t)) => int_lit(*n, *t),
                Const::Int(_, t) => panic!("an integer of type `{t}`"),
                Const::Bool(b) => b.to_string(),
                Const::Char(c) => format!("UINT32_C({})", u32::from(*c)),
                Const::Unit => "(t_unit){0}".to_string(),
                Const::Str(s) => self.literal(s.as_bytes()),
                Const::Bytes(b) => self.literal(b),
                Const::Fn(name, _) => self.fn_value(name),
                Const::Param(_) | Const::Receiver(_) => panic!("`{c:?}` as a value"),
            },
        }
    }

    /// A string or byte string literal, as a reference to its bytes.
    fn literal(&mut self, bytes: &[u8]) -> String {
        let sl = self.cty(&ref_to(Type::Str));
        if bytes.is_empty() {
            return format!("(({sl}){{(uint8_t *)T_NOWHERE, 0}})");
        }
        let name = match self.lits.get(bytes) {
            Some(n) => n.clone(),
            None => {
                let n = self.fresh(&format!("t_lit{}", self.lits.len()));
                let _ = writeln!(
                    self.lit_defs,
                    "static uint8_t {n}[{}] = {};",
                    bytes.len(),
                    c_str(bytes)
                );
                self.lits.insert(bytes.to_vec(), n.clone());
                n
            }
        };
        format!("(({sl}){{{name}, {}}})", bytes.len())
    }

    // Functions.

    /// The C name of the function, closure or glue body called `name`,
    /// which is then written.
    fn body_name(&mut self, name: &str) -> String {
        if let Some(c) = self.fn_names.get(name) {
            return c.clone();
        }
        let c = self.fresh(&format!("t_{}", sanitize(name)));
        self.fn_names.insert(name.to_string(), c.clone());
        self.todo.push(name.to_string());
        c
    }

    fn is_intrinsic(&self, name: &str) -> bool {
        self.decls.get(name).is_some_and(|d| d.intrinsic.is_some())
    }

    /// Function `name` as a value: a pointer to its C function.
    fn fn_value(&mut self, name: &str) -> String {
        if self.is_intrinsic(name) {
            panic!("intrinsic `{name}` as a value");
        }
        if self.exe.body(name).is_none() {
            panic!("`{name}` has no body");
        }
        self.body_name(name)
    }

    /// The site of `span`, as a C expression: where a panic there is
    /// shown. One in a library is shown where the innermost call from
    /// the program's own code is, as `Error::blamed_caller` does.
    fn site(&mut self, span: Span) -> String {
        match self.sources.in_library(span.start) {
            true => "t_site".to_string(),
            false => self.site_id(span).to_string(),
        }
    }

    fn site_id(&mut self, span: Span) -> usize {
        if let Some(&i) = self.sites.get(&span) {
            return i;
        }
        let text = Error::runtime(span, "").render_in(self.sources);
        let text = match text.split_once('\n') {
            Some((_, rest)) => format!("{rest}\n"),
            None => String::new(),
        };
        self.site_texts.push(text);
        let i = self.site_texts.len() - 1;
        self.sites.insert(span, i);
        i
    }

    /// The prototype of body `name`, its C name `c`.
    fn signature(&mut self, c: &str, body: &Body) -> String {
        let ret = self.cty(&body.locals[0].ty);
        let params: Vec<String> = (1..=body.params)
            .map(|i| format!("{} _{i}", self.cty(&body.locals[i].ty)))
            .collect();
        let params = match params.is_empty() {
            true => "void".to_string(),
            false => params.join(", "),
        };
        format!("static {ret} {c}({params})")
    }

    fn run(&mut self) {
        let main = self.exe.body("main").is_some();
        if main {
            self.body_name("main");
            if let Some(r) = self.exe.report("main") {
                self.body_name(r);
            }
            if let Some(g) = self.exe.entry_drop("main") {
                self.body_name(g);
            }
        }
        while let Some(name) = self.todo.pop() {
            let exe: &'e Executable = self.exe;
            let (name, body) = exe.body_named(&name).expect("a body");
            self.function(name, body);
        }
    }

    /// Writes the C function of body `name`.
    fn function(&mut self, name: &str, body: &Body) {
        let c = self.fn_names[name].clone();
        let sig = self.signature(&c, body);
        let _ = writeln!(self.protos, "{sig};");
        let cx = Cx { name, body };
        let mut out = String::new();
        let t_name = name.replace("*/", "* /");
        let _ = writeln!(out, "/* {t_name} */\n{sig} {{");
        // Every local but the parameters, zeroed, so that nothing reads
        // what was never written (C2).
        for i in (0..body.locals.len()).filter(|i| *i == 0 || *i > body.params) {
            let ty = self.cty(&body.locals[i].ty);
            let _ = writeln!(out, "    {ty} _{i} = {{0}};");
        }
        let mut line = None;
        for (b, block) in body.blocks.iter().enumerate() {
            let Some(term) = &block.term else { continue };
            let _ = writeln!(out, "bb{b}:;");
            for s in &block.stmts {
                self.line(&mut out, &mut line, s.span);
                self.statement(&cx, s, &mut out);
            }
            self.terminator(&cx, term, &mut out);
        }
        out.push_str("}\n\n");
        self.defs.push_str(&out);
    }

    fn line(&mut self, out: &mut String, last: &mut Option<(String, usize)>, span: Span) {
        if !self.options.lines {
            return;
        }
        let (file, local) = self.sources.local(span);
        let src = self.sources.src(file);
        let n = 1 + src[..local.start.min(src.len())].matches('\n').count();
        let path = self.sources.shown(file);
        if last.as_ref() != Some(&(path.clone(), n)) {
            let _ = writeln!(out, "#line {n} {}", c_str(path.as_bytes()));
            *last = Some((path, n));
        }
    }

    fn statement(&mut self, cx: &Cx, s: &Statement, out: &mut String) {
        match &s.kind {
            StmtKind::Assign { place, value, .. } => self.assign(cx, place, value, s.span, out),
            StmtKind::Drop(p, t) => {
                let (e, _) = self.place(cx, p);
                let glue = self.drop_glue(&t.erased());
                let _ = writeln!(out, "    {glue}({e});");
            }
            StmtKind::Free(p) => {
                let (e, _) = self.place(cx, p);
                let _ = writeln!(out, "    t_free({e});");
            }
            // What only the borrow checker and the interpreter look at.
            StmtKind::Inspect(_)
            | StmtKind::Dead(_)
            | StmtKind::Consume(..)
            | StmtKind::Refine(..)
            | StmtKind::Kept(_) => {}
        }
    }

    /// The C name of the drop glue of `t`.
    fn drop_glue(&mut self, t: &Type) -> String {
        let name = glue_name("{drop}", t);
        if self.exe.body(&name).is_some() {
            return self.body_name(&name);
        }
        let Type::Adt(symbol, _, args) = t else {
            panic!("no drop glue for `{t}`")
        };
        assert_eq!(symbol, crate::check::ARENA, "no drop glue for `{t}`");
        if let Some(c) = self.fn_names.get(&name) {
            return c.clone();
        }
        // An arena's values, each by its box's glue, in the order they
        // were put in, as the interpreter drops them; then the arena.
        let c = self.fresh(&format!("t_{}", sanitize(&name)));
        self.fn_names.insert(name, c.clone());
        let a = self.cty(t);
        let elem = args[0].erased();
        let e = self.cty(&elem);
        let boxed = self.drop_glue(&Type::Box(Box::new(elem)));
        let sig = format!("static t_unit {c}({a} x)");
        let _ = writeln!(self.protos, "{sig};");
        let _ = writeln!(
            self.defs,
            "{sig} {{\n    for (uint64_t i = 0; i < x.len; i++) {boxed}(({e} *)x.items[i]);\n    \
             free(x.items);\n    return (t_unit){{0}};\n}}\n"
        );
        c
    }

    fn assign(&mut self, cx: &Cx, place: &Place, value: &Rvalue, span: Span, out: &mut String) {
        let (dest, dest_ty) = self.place(cx, place);
        // There's no value of `!`: what reads one can't run, and the C
        // types wouldn't fit, as the place may be of any type.
        let never = |g: &mut Gen, o: &Operand| match o.place() {
            Some(p) => g.place_ty(cx, p) == Type::Never,
            None => false,
        };
        if value.operands().into_iter().any(|o| never(self, o)) {
            out.push_str("    t_unreachable();\n");
            return;
        }
        let e = match value {
            Rvalue::Use(o) => self.operand(cx, o),
            Rvalue::Ref(_, _, p) => {
                let (e, t) = self.place(cx, p);
                match unsized_ty(&t) {
                    true => e,
                    false => format!("&{e}"),
                }
            }
            Rvalue::Unary(op, o, id) => {
                let t = self.operand_ty(cx, o);
                let x = self.operand(cx, o);
                match (op, &t) {
                    (UnOp::Neg, Type::Int(i)) => match self.proved(*id) {
                        true => format!("t_wneg_{}({x})", suffix(*i)),
                        false => format!("t_neg_{}({x}, {})", suffix(*i), self.site(span)),
                    },
                    (UnOp::Not, Type::Int(i)) => format!("(({})~{x})", c_int(*i)),
                    (UnOp::Not, _) => format!("(!{x})"),
                    (op, t) => panic!("`{op}` of `{t}`"),
                }
            }
            Rvalue::Binary(op, l, r, id) => self.binary(cx, *op, l, r, *id, span),
            Rvalue::Cast(o, to, id) => self.cast(cx, o, &to.erased(), *id, span),
            Rvalue::Aggregate(k, t, os) => {
                let t = t.erased();
                let vals: Vec<String> = os.iter().map(|o| self.operand(cx, o)).collect();
                match k {
                    Agg::Box => {
                        let Type::Box(inner) = &t else {
                            panic!("a box of type `{t}`")
                        };
                        let inner = self.cty(inner);
                        let site = self.site(span);
                        let _ = writeln!(
                            out,
                            "    {{ {inner} *p_ = t_alloc(sizeof({inner}), {site}); *p_ = {}; {dest} = p_; }}",
                            vals[0]
                        );
                        return;
                    }
                    Agg::Repeat => {
                        let Type::Array(elem, n) = &t else {
                            panic!("a repeat of type `{t}`")
                        };
                        let Type::Nat(n) = **n else {
                            panic!("an array of no length")
                        };
                        let a = self.cty(&t);
                        if n == 0 {
                            let _ = writeln!(out, "    {dest} = ({a}){{0}};");
                            return;
                        }
                        let e = self.cty(elem);
                        let _ = writeln!(
                            out,
                            "    {{ {e} v_ = {}; {a} r_; for (uint64_t i_ = 0; i_ < {n}; i_++) r_.a[i_] = v_; {dest} = r_; }}",
                            vals[0]
                        );
                        return;
                    }
                    Agg::Array => {
                        let a = self.cty(&t);
                        match vals.is_empty() {
                            true => format!("(({a}){{0}})"),
                            false => format!("(({a}){{{{{}}}}})", vals.join(", ")),
                        }
                    }
                    Agg::Struct => {
                        let c = self.cty(&t);
                        match vals.is_empty() {
                            true => format!("(({c}){{0}})"),
                            false => format!("(({c}){{{}}})", vals.join(", ")),
                        }
                    }
                    Agg::Variant(v) => {
                        let c = self.cty(&t);
                        match vals.is_empty() {
                            true => format!("(({c}){{.tag = {v}}})"),
                            false => {
                                format!("(({c}){{.tag = {v}, .u.v{v} = {{{}}}}})", vals.join(", "))
                            }
                        }
                    }
                    Agg::Closure(id) => match &t {
                        // One that captures nothing is its body.
                        Type::Fn(_) => {
                            let code = crate::mono::closure_copy(closure_root(cx.name), *id);
                            self.body_name(&code)
                        }
                        _ => {
                            let c = self.cty(&t);
                            match vals.is_empty() {
                                true => format!("(({c}){{0}})"),
                                false => format!("(({c}){{{}}})", vals.join(", ")),
                            }
                        }
                    },
                }
            }
            Rvalue::Call(f, args) => {
                let (call, body, never) = self.call(cx, f, args, &dest_ty, span);
                // What never returns gives nothing to write: its place may
                // be of any type.
                let assign = match never {
                    true => format!("{call};"),
                    false => format!("{dest} = {call};"),
                };
                // A call of a body from the program's own code is where a
                // panic in a library under it is shown.
                if body && !self.sources.in_library(span.start) {
                    let site = self.site_id(span);
                    let _ = writeln!(
                        out,
                        "    {{ uint32_t s_ = t_site; t_site = {site}; {assign} t_site = s_; }}"
                    );
                } else {
                    let _ = writeln!(out, "    {assign}");
                }
                return;
            }
        };
        let _ = writeln!(out, "    {dest} = {e};");
    }

    fn proved(&self, id: crate::ast::NodeId) -> bool {
        self.exe.program().proved.contains(&id)
    }

    fn binary(
        &mut self,
        cx: &Cx,
        op: BinOp,
        l: &Operand,
        r: &Operand,
        id: crate::ast::NodeId,
        span: Span,
    ) -> String {
        let t = self.operand_ty(cx, l);
        let (a, b) = (self.operand(cx, l), self.operand(cx, r));
        let int = match &t {
            Type::Int(i) => Some(*i),
            _ => None,
        };
        match (op, int) {
            (BinOp::Eq, _) if t == Type::Unit => "true".to_string(),
            (BinOp::Ne, _) if t == Type::Unit => "false".to_string(),
            (BinOp::Eq, _) => format!("({a} == {b})"),
            (BinOp::Ne, _) => format!("({a} != {b})"),
            (BinOp::Lt, _) => format!("({a} < {b})"),
            (BinOp::Le, _) => format!("({a} <= {b})"),
            (BinOp::Gt, _) => format!("({a} > {b})"),
            (BinOp::Ge, _) => format!("({a} >= {b})"),
            (BinOp::BitAnd, Some(i)) => format!("(({})({a} & {b}))", c_int(i)),
            (BinOp::BitOr, Some(i)) => format!("(({})({a} | {b}))", c_int(i)),
            (BinOp::BitXor, Some(i)) => format!("(({})({a} ^ {b}))", c_int(i)),
            (BinOp::Shr, Some(i)) => format!("t_shr_{}({a}, {b})", suffix(i)),
            (op, Some(i)) => {
                let f = match op {
                    BinOp::Add => "add",
                    BinOp::Sub => "sub",
                    BinOp::Mul => "mul",
                    BinOp::Div => "div",
                    BinOp::Rem => "rem",
                    BinOp::Shl => "shl",
                    op => panic!("`{op}` of integers"),
                };
                match self.proved(id) {
                    true => format!("t_w{f}_{}({a}, {b})", suffix(i)),
                    false => format!("t_{f}_{}({a}, {b}, {})", suffix(i), self.site(span)),
                }
            }
            (op, _) => panic!("`{op}` of `{t}`"),
        }
    }

    /// `o as to`: the same number, or character, if it's one of `to`'s.
    fn cast(
        &mut self,
        cx: &Cx,
        o: &Operand,
        to: &Type,
        id: crate::ast::NodeId,
        span: Span,
    ) -> String {
        let from = self.operand_ty(cx, o);
        let x = self.operand(cx, o);
        let (signed, lo, hi) = match &from {
            Type::Int(i) => {
                let (lo, hi) = i.range_64();
                (i.signed(), lo, hi)
            }
            Type::Char => (false, 0, 0x10FFFF),
            t => panic!("`as` from `{t}`"),
        };
        let wide = match signed {
            true => format!("((int64_t){x})"),
            false => format!("((uint64_t){x})"),
        };
        let proved = self.proved(id);
        match to {
            Type::Char => {
                if proved {
                    return format!("((uint32_t){wide})");
                }
                let site = self.site(span);
                match signed {
                    true => format!(
                        "({wide} >= 0 && t_is_char((uint64_t){wide}) ? (uint32_t){wide} : (t_char_fail_i({wide}, {site}), 0u))"
                    ),
                    false => format!(
                        "(t_is_char({wide}) ? (uint32_t){wide} : (t_char_fail_u({wide}, {site}), 0u))"
                    ),
                }
            }
            Type::Int(t) => {
                let c = c_int(*t);
                if proved {
                    return format!("t_wrap_{}((uint64_t){wide})", suffix(*t));
                }
                let (tlo, thi) = t.range_64();
                let mut conds = Vec::new();
                if lo < tlo {
                    conds.push(format!("{wide} >= INT64_C({tlo})"));
                }
                if hi > thi {
                    conds.push(match signed {
                        true => format!("{wide} <= INT64_C({thi})"),
                        false => format!("{wide} <= UINT64_C({thi})"),
                    });
                }
                if conds.is_empty() {
                    return format!("(({c}){wide})");
                }
                let site = self.site(span);
                let what = c_str(t.a().as_bytes());
                let fail = match signed {
                    true => format!("t_as_fail_i({wide}, {what}, {site})"),
                    false => format!("t_as_fail_u({wide}, {what}, {site})"),
                };
                format!("({} ? ({c}){wide} : ({fail}, ({c})0))", conds.join(" && "))
            }
            t => panic!("`as` to `{t}`"),
        }
    }

    /// The C call of `f` with `args`, giving a `ret`, whether it calls a
    /// body (not an intrinsic), and whether what it calls never returns.
    fn call(
        &mut self,
        cx: &Cx,
        f: &Operand,
        args: &[Operand],
        ret: &Type,
        span: Span,
    ) -> (String, bool, bool) {
        match f {
            Operand::Const(Const::Fn(name, _)) if self.is_intrinsic(name) => {
                let never = matches!(self.decls[name.as_str()].ret.kind, TypeExprKind::Never);
                let ret = if never { Type::Never } else { ret.clone() };
                let tys: Vec<Type> = args.iter().map(|a| self.operand_ty(cx, a)).collect();
                let w = self.intrinsic(name, &tys, &ret);
                let mut vals: Vec<String> = args.iter().map(|a| self.operand(cx, a)).collect();
                vals.push(self.site(span));
                (format!("{w}({})", vals.join(", ")), false, never)
            }
            Operand::Const(Const::Fn(name, _)) => {
                let c = self.fn_value(name);
                let never = self.exe.body(name).expect("a body").locals[0].ty == Type::Never;
                let vals: Vec<String> = args.iter().map(|a| self.operand(cx, a)).collect();
                (format!("{c}({})", vals.join(", ")), true, never)
            }
            Operand::Const(Const::Receiver(_)) => {
                let (recv, rest) = args.split_first().expect("a receiver");
                let t = self.operand_ty(cx, recv);
                let e = self.operand(cx, recv);
                let vals: Vec<String> = rest.iter().map(|a| self.operand(cx, a)).collect();
                let never = self.returns_never(&t);
                (self.call_value(&e, &t, vals), true, never)
            }
            f => {
                let t = self.operand_ty(cx, f);
                let e = self.operand(cx, f);
                let vals: Vec<String> = args.iter().map(|a| self.operand(cx, a)).collect();
                let never = self.returns_never(&t);
                (self.call_value(&e, &t, vals), true, never)
            }
        }
    }

    /// Does calling what's of type `t` never return?
    fn returns_never(&self, t: &Type) -> bool {
        match t {
            Type::Fn(sig) => *sig.ret == Type::Never,
            Type::Ref(_, _, inner) => self.returns_never(inner),
            Type::Adt(..) => (self.exe.closure_body(t))
                .and_then(|c| self.exe.body(c))
                .is_some_and(|b| b.locals[0].ty == Type::Never),
            _ => false,
        }
    }

    /// The C call of what `e`, of type `t`, is: a function, a closure, or
    /// a reference to one, as the interpreter's `call` sees it.
    fn call_value(&mut self, e: &str, t: &Type, args: Vec<String>) -> String {
        let join = |first: Option<String>, args: Vec<String>| -> String {
            first.into_iter().chain(args).collect::<Vec<_>>().join(", ")
        };
        match t {
            Type::Fn(_) => format!("({e})({})", join(None, args)),
            Type::Ref(_, _, inner) => match &**inner {
                Type::Fn(_) => format!("(*{e})({})", join(None, args)),
                Type::Ref(..) => self.call_value(&format!("(*{e})"), inner, args),
                Type::Adt(..) => {
                    let (c, by) = self.closure_fn(inner);
                    match by {
                        Some(Some(_)) => format!("{c}({})", join(Some(e.to_string()), args)),
                        None => format!("{c}({})", join(None, args)),
                        Some(None) => panic!("a closure taken by value called through a reference"),
                    }
                }
                t => panic!("a call of a `&{t}`"),
            },
            Type::Adt(..) => {
                let (c, by) = self.closure_fn(t);
                match by {
                    Some(None) => format!("{c}({})", join(Some(e.to_string()), args)),
                    None => format!("{c}({})", join(None, args)),
                    Some(Some(_)) => panic!("a closure taken by reference called by value"),
                }
            }
            t => panic!("a call of a `{t}`"),
        }
    }

    /// The C function closure type `t` runs, and how it takes the closure.
    fn closure_fn(&mut self, t: &Type) -> (String, Option<Option<bool>>) {
        let exe: &'e Executable = self.exe;
        let code = (exe.closure_body(&t.erased()))
            .unwrap_or_else(|| panic!("no body for closure type `{t}`"));
        let body = exe.body(code).expect("a closure's body");
        (self.body_name(code), env_by(body))
    }

    fn terminator(&mut self, cx: &Cx, term: &Term, out: &mut String) {
        match term {
            Term::Goto(t) => {
                let _ = writeln!(out, "    goto bb{};", t.0);
            }
            Term::Branch(o, t, e) => {
                let c = self.operand(cx, o);
                let _ = writeln!(out, "    if ({c}) goto bb{}; else goto bb{};", t.0, e.0);
            }
            Term::Match(p, pats, targets) => {
                let (e, t) = self.place(cx, p);
                for (pat, target) in pats.iter().zip(targets) {
                    let c = self.pat(pat, &e, &t);
                    let _ = writeln!(out, "    if ({c}) goto bb{};", target.0);
                }
                out.push_str("    t_unreachable();\n");
            }
            Term::Return => out.push_str("    return _0;\n"),
            Term::Unreachable => out.push_str("    t_unreachable();\n"),
        }
    }

    /// The C condition that the value `e`, of type `t`, matches `pat`.
    fn pat(&mut self, pat: &Pat, e: &str, t: &Type) -> String {
        let all = |conds: Vec<String>| match conds.is_empty() {
            true => "1".to_string(),
            false => format!("({})", conds.join(" && ")),
        };
        match pat {
            Pat::Any => "1".to_string(),
            Pat::Bool(true) => format!("({e})"),
            Pat::Bool(false) => format!("(!{e})"),
            Pat::Range(lo, hi) => {
                let lit = |n: i128| match t {
                    Type::Int(i) if i.signed() => format!("INT64_C({n})"),
                    _ => format!("UINT64_C({n})"),
                };
                let mut conds = Vec::new();
                if let Some(lo) = lo {
                    conds.push(format!("{e} >= {}", lit(*lo)));
                }
                if let Some(hi) = hi {
                    conds.push(format!("{e} <= {}", lit(*hi)));
                }
                all(conds)
            }
            Pat::Variant(v, pats) => {
                let fields = self.variants(t)[*v].1.clone();
                let mut conds = vec![format!("{e}.tag == {v}")];
                for (i, p) in pats.iter().enumerate() {
                    if !matches!(p, Pat::Any) {
                        conds.push(self.pat(p, &format!("{e}.u.v{v}.f{i}"), &fields[i]));
                    }
                }
                all(conds)
            }
            Pat::Struct(fs) => {
                let fields = self.fields(t);
                let conds = (fs.iter())
                    .filter(|(_, p)| !matches!(p, Pat::Any))
                    .map(|(i, p)| self.pat(p, &format!("{e}.f{i}"), &fields[*i]))
                    .collect();
                all(conds)
            }
            Pat::Array(pats) => {
                let Type::Array(elem, _) = t else {
                    panic!("an array pattern of `{t}`")
                };
                let conds = (pats.iter().enumerate())
                    .filter(|(_, p)| !matches!(p, Pat::Any))
                    .map(|(i, p)| self.pat(p, &format!("{e}.a[{i}]"), elem))
                    .collect();
                all(conds)
            }
            Pat::Or(alts) => {
                let alts: Vec<String> = alts.iter().map(|p| self.pat(p, e, t)).collect();
                format!("({})", alts.join(" || "))
            }
            Pat::Deref(p) => {
                let Type::Ref(_, _, inner) = t else {
                    panic!("a `&` pattern of `{t}`")
                };
                self.pat(p, &format!("(*{e})"), &inner.erased())
            }
        }
    }

    // Intrinsics.

    /// The C function for intrinsic copy `copy`, taking `params` and giving
    /// `ret`, and then a site, where it may stop the program.
    fn intrinsic(&mut self, copy: &str, params: &[Type], ret: &Type) -> String {
        if let Some(c) = self.fn_names.get(copy) {
            return c.clone();
        }
        let c = self.fresh(&format!("t_{}", sanitize(copy)));
        self.fn_names.insert(copy.to_string(), c.clone());
        let r = self.cty(ret);
        let mut ps: Vec<String> = (params.iter().enumerate())
            .map(|(i, p)| format!("{} a{i}", self.cty(p)))
            .collect();
        ps.push("uint32_t s".to_string());
        let sig = format!("static {r} {c}({})", ps.join(", "));
        let body = self.intrinsic_body(copy, params, ret);
        let _ = writeln!(self.protos, "{sig};");
        let t_name = copy.replace("*/", "* /");
        let _ = writeln!(
            self.defs,
            "/* {t_name} */\n{sig} {{\n    (void)s;\n{body}}}\n"
        );
        c
    }

    fn intrinsic_body(&mut self, copy: &str, params: &[Type], ret: &Type) -> String {
        let of = copy.split("::<").next().unwrap_or_default();
        let op = copy.rsplit("::").next().unwrap_or_default();
        let r = self.cty(ret);
        let unit = "    return (t_unit){0};\n".to_string();
        let int = |t: &Type| match t {
            Type::Int(i) => *i,
            t => panic!("`{copy}` of `{t}`"),
        };
        // The element type of what a reference to a slice, array, buffer
        // or arena points to, or that a buffer is.
        let elem = |t: &Type| -> Type {
            let t = match t {
                Type::Ref(_, _, inner) => &**inner,
                t => t,
            };
            match t {
                Type::Slice(e) | Type::Array(e, _) => e.erased(),
                Type::Adt(_, _, args) => args[0].erased(),
                Type::Str => Type::Int(IntTy::U8),
                t => panic!("the element type of `{t}`"),
            }
        };
        let some_none = |g: &mut Gen| (g.variant(ret, "Some"), g.variant(ret, "None"));
        match of {
            "core::num::checked_mul" => {
                let i = int(&params[0]);
                let (some, none) = some_none(self);
                let wide = if i.signed() { "int64_t" } else { "uint64_t" };
                format!(
                    "    if (t_mul_ok_{s}(a0, a1)) return ({r}){{.tag = {some}, .u.v{some} = {{({t})(({wide})a0 * a1)}}}};\n    \
                     return ({r}){{.tag = {none}}};\n",
                    s = suffix(i),
                    t = c_int(i),
                )
            }
            "core::num::wrapping_mul" => {
                format!("    return t_wmul_{}(a0, a1);\n", suffix(int(&params[0])))
            }
            "core::num::wrap" => {
                let from = int(&params[0]);
                let wide = if from.signed() { "int64_t" } else { "uint64_t" };
                format!(
                    "    return t_wrap_{}((uint64_t)({wide})a0);\n",
                    suffix(int(ret))
                )
            }
            "core::num::least" | "core::num::greatest" => {
                let i = int(ret);
                let (lo, hi) = i.range_64();
                let n = if of.ends_with("least") { lo } else { hi };
                format!("    return {};\n", int_lit(n, i))
            }
            "core::panic::panic" => "    t_panic(a0.ptr, a0.len, s);\n".to_string(),
            "alloc::raw::out_of_memory" => "    t_fail(\"out of memory\", s);\n".to_string(),
            "core::str::from_utf8_unchecked" => "    return a0;\n".to_string(),
            "core::mem::swap" => {
                let Type::Ref(_, _, inner) = &params[0] else {
                    panic!("`swap` of `{}`", params[0])
                };
                let t = self.cty(inner);
                format!("    {t} t = *a0;\n    *a0 = *a1;\n    *a1 = t;\n{unit}")
            }
            "core::slice::empty" | "core::slice::empty_mut" => {
                let e = self.cty(&elem(ret));
                format!("    return ({r}){{({e} *)T_NOWHERE, 0}};\n")
            }
            "core::array::from_fn_mut" => {
                let Type::Array(_, n) = ret else {
                    panic!("`from_fn` of `{ret}`")
                };
                let Type::Nat(n) = **n else {
                    panic!("an array of no length")
                };
                if n == 0 {
                    return format!("    (void)a0;\n    return ({r}){{0}};\n");
                }
                let call = self.call_value("a0", &params[0], vec!["i".to_string()]);
                format!(
                    "    {r} r = {{0}};\n    for (uint64_t i = 0; i < {n}; i++) r.a[i] = {call};\n    return r;\n"
                )
            }
            "str::as_bytes" | "<str>::as_bytes" => "    return a0;\n".to_string(),
            "str::len" | "<str>::len" => "    return a0.len;\n".to_string(),
            _ if of.starts_with("<core::panic::Message>::") => match op {
                "new" => format!("    t_msg_new();\n    return ({r}){{0}};\n"),
                "push_str" => format!("    t_msg_push(a1.ptr, a1.len);\n{unit}"),
                "push_char" => format!("    t_msg_char(a1);\n{unit}"),
                "abort" => "    t_end(s);\n".to_string(),
                _ => panic!("unknown intrinsic `{copy}`"),
            },
            _ if of.starts_with("<std::io::") => match copy.strip_prefix("<std::io::") {
                Some("Stdin>::read_byte") => "    return t_read_byte();\n".to_string(),
                Some("Stdout>::write_byte") => "    return t_write_byte(stdout, a1);\n".to_string(),
                Some("Stderr>::write_byte") => "    return t_write_byte(stderr, a1);\n".to_string(),
                _ => panic!("unknown intrinsic `{copy}`"),
            },
            "<alloc::arena::Arena" => {
                let e = self.cty(&elem(&params.first().cloned().unwrap_or(ret.clone())));
                match op {
                    "new" => format!("    return ({r}){{0}};\n"),
                    "alloc" => format!(
                        "    {e} *p = t_alloc(sizeof({e}), s);\n    *p = a1;\n    \
                         t_grow_ptrs(&a0->items, &a0->cap, a0->len, s);\n    \
                         a0->items[a0->len++] = p;\n    return p;\n"
                    ),
                    "len" => "    return a0->len;\n".to_string(),
                    _ => panic!("unknown intrinsic `{copy}`"),
                }
            }
            "<alloc::raw::RawBuf" => {
                let buf = match op {
                    "new" | "empty" => ret.clone(),
                    _ => params[0].clone(),
                };
                let e = self.cty(&elem(&buf));
                let broken = "t_fail(\"unsafe code broke its obligation\", s)";
                match op {
                    "new" => format!(
                        "    if (a0 == 0) return ({r}){{({e} *)T_NOWHERE, 0}};\n    \
                         if (a0 > UINT64_MAX / sizeof({e})) t_oom_slots(a0, s);\n    \
                         {e} *p = malloc(a0 * sizeof({e}));\n    if (!p) t_oom_slots(a0, s);\n    \
                         return ({r}){{p, a0}};\n"
                    ),
                    "empty" => format!("    return ({r}){{({e} *)T_NOWHERE, 0}};\n"),
                    "cap" => "    return a0->cap;\n".to_string(),
                    "grow" => format!(
                        "    uint64_t n = a0->cap + a1;\n    \
                         if (n < a0->cap || n > UINT64_MAX / sizeof({e})) t_oom_slots(n, s);\n    \
                         if (n == 0) return (t_unit){{0}};\n    \
                         {e} *p = a0->cap ? realloc(a0->ptr, n * sizeof({e})) : malloc(n * sizeof({e}));\n    \
                         if (!p) t_oom_slots(n, s);\n    a0->ptr = p;\n    a0->cap = n;\n{unit}"
                    ),
                    "slice" | "slice_mut" => format!(
                        "    if (!(a1 <= a2 && a2 <= a0->cap)) {broken};\n    \
                         return ({r}){{a1 ? a0->ptr + a1 : a0->ptr, a2 - a1}};\n"
                    ),
                    "write" => {
                        format!("    if (a1 >= a0->cap) {broken};\n    a0->ptr[a1] = a2;\n{unit}")
                    }
                    "read" => {
                        format!("    if (a1 >= a0->cap) {broken};\n    return a0->ptr[a1];\n")
                    }
                    "free" => format!("    if (a0.cap) free(a0.ptr);\n{unit}"),
                    _ => panic!("unknown intrinsic `{copy}`"),
                }
            }
            _ if of.starts_with("<[") => {
                // An array's, by reference.
                if let Type::Ref(_, _, inner) = &params[0]
                    && let Type::Array(e, n) = &**inner
                {
                    let Type::Nat(n) = **n else {
                        panic!("an array of no length")
                    };
                    let e = self.cty(e);
                    return match op {
                        "len" => format!("    (void)a0;\n    return UINT64_C({n});\n"),
                        "as_slice" | "as_mut_slice" if n == 0 => {
                            format!("    (void)a0;\n    return ({r}){{({e} *)T_NOWHERE, 0}};\n")
                        }
                        "as_slice" | "as_mut_slice" => format!("    return ({r}){{a0->a, {n}}};\n"),
                        _ => panic!("unknown intrinsic `{copy}`"),
                    };
                }
                match op {
                    "len" => "    return a0.len;\n".to_string(),
                    "index" | "index_mut" => {
                        "    if (a1 >= a0.len) t_bug(\"index out of range\");\n    \
                         return a0.ptr + a1;\n"
                            .to_string()
                    }
                    "get" | "get_mut" => {
                        let (some, none) = some_none(self);
                        format!(
                            "    if (a1 < a0.len) return ({r}){{.tag = {some}, .u.v{some} = {{a0.ptr + a1}}}};\n    \
                             return ({r}){{.tag = {none}}};\n"
                        )
                    }
                    "split_at" | "split_at_mut" => {
                        let (some, none) = some_none(self);
                        let split = self.variants(ret)[some].1[0].clone();
                        let sp = self.cty(&split);
                        let sl = self.cty(&params[0]);
                        format!(
                            "    if (a1 > a0.len) return ({r}){{.tag = {none}}};\n    \
                             {sp} v = {{({sl}){{a0.ptr, a1}}, ({sl}){{a1 ? a0.ptr + a1 : a0.ptr, a0.len - a1}}}};\n    \
                             return ({r}){{.tag = {some}, .u.v{some} = {{v}}}};\n"
                        )
                    }
                    _ => panic!("unknown intrinsic `{copy}`"),
                }
            }
            _ => panic!("unknown intrinsic `{copy}`"),
        }
    }

    // The program.

    fn finish(mut self) -> String {
        let main = self.main();
        // The structs, each after those it holds by value.
        let mut defs: HashMap<String, (String, Vec<Type>)> = HashMap::new();
        let mut i = 0;
        while i < self.ty_order.len() {
            let t = self.ty_order[i].clone();
            let d = self.define(&t);
            defs.insert(key(&t), d);
            i += 1;
        }
        let mut structs = String::new();
        let mut done = HashSet::new();
        fn visit(
            t: &Type,
            defs: &HashMap<String, (String, Vec<Type>)>,
            done: &mut HashSet<String>,
            out: &mut String,
        ) {
            let Some((def, deps)) = defs.get(&key(t)) else {
                return;
            };
            if !done.insert(key(t)) {
                return;
            }
            for d in deps {
                visit(d, defs, done, out);
            }
            out.push_str(def);
        }
        for t in &self.ty_order {
            visit(t, &defs, &mut done, &mut structs);
        }
        let mut out = String::new();
        out.push_str("/* Written by `t build` (https://github.com/lkurcak/t-lang). */\n");
        out.push_str(SHIM);
        out.push('\n');
        for t in &self.ty_order {
            let name = &self.tys[&key(t)];
            let _ = writeln!(out, "typedef struct {name} {name};");
        }
        out.push_str(&self.fn_ptr_defs);
        out.push_str(&structs);
        out.push('\n');
        out.push_str(&self.protos);
        out.push('\n');
        out.push_str(&self.lit_defs);
        out.push_str("\nstatic const char *const t_sites[] = {\n");
        for s in &self.site_texts {
            let _ = writeln!(out, "    {},", c_str(s.as_bytes()));
        }
        out.push_str("    \"\",\n};\n\n");
        out.push_str("static const char *t_site_text(uint32_t site) { return t_sites[site]; }\n\n");
        out.push_str(&self.defs);
        out.push_str(&main);
        out
    }

    /// C's `main`: runs T's, reports the error it gives back, if it gives
    /// back one, drops what it gives back, and ends as `t run` does.
    fn main(&mut self) -> String {
        let exe: &'e Executable = self.exe;
        let Some(body) = exe.body("main") else {
            return String::new();
        };
        let start = self.site_id(Span::new(0, 0));
        let main = self.body_name("main");
        // The functions `main` gets to are all written: these are only
        // looked up.
        let ret_ty = body.locals[0].ty.erased();
        let ret = self.cty(&ret_ty);
        let args = match body.params {
            0 => String::new(),
            _ => format!("({}){{0}}", self.cty(&body.locals[1].ty)),
        };
        let mut out = String::new();
        let _ = writeln!(
            out,
            "int main(void) {{\n    t_start();\n    t_site = {start};"
        );
        let _ = writeln!(out, "    {ret} r = {main}({args});");
        let mut failed = "false".to_string();
        let _ = writeln!(out, "    (void)r;");
        if let Some(report) = exe.report("main") {
            let err = self.variant(&ret_ty, "Err");
            let rb = exe.body(report).expect("a report's body");
            let report = self.fn_names[report].clone();
            let e_ty = self.cty(&self.variants(&ret_ty)[err].1[0].clone());
            let stderr = self.cty(&rb.locals[2].ty);
            let _ = writeln!(
                out,
                "    if (r.tag == {err}) {{ {e_ty} e = r.u.v{err}.f0; {report}(&e, ({stderr}){{0}}); }}"
            );
            let _ = writeln!(out, "    bool failed = r.tag == {err};");
            failed = "failed".to_string();
        }
        if let Some(glue) = exe.entry_drop("main") {
            let glue = self.fn_names[glue].clone();
            let _ = writeln!(out, "    {glue}(r);");
        }
        let _ = writeln!(out, "    return t_exit({failed});\n}}");
        out
    }
}

/// Compiles C program `c` to an executable at `out`, with the C compiler
/// `$CC` names, or `cc`, and the flags `$T_LANG_CFLAGS` adds after the
/// usual ones. What it says when it fails is the error.
pub fn compile(c: &str, out: &std::path::Path) -> Result<(), String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "t-lang-build-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make `{}`: {e}", dir.display()))?;
    let file = dir.join("main.c");
    let result = (|| {
        std::fs::write(&file, c).map_err(|e| format!("cannot write `{}`: {e}", file.display()))?;
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let extra = std::env::var("T_LANG_CFLAGS").unwrap_or_default();
        let output = std::process::Command::new(&cc)
            .args(["-std=c11", "-O2", "-o"])
            .arg(out)
            .arg(&file)
            .args(extra.split_whitespace())
            .output()
            .map_err(|e| format!("cannot run the C compiler `{cc}`: {e}"))?;
        match output.status.success() {
            true => Ok(()),
            false => Err(format!(
                "the C compiler `{cc}` failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            )),
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}
