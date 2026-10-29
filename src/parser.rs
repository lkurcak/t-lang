//! Recursive-descent parser: tokens -> AST.
//!
//! Grammar (lowest to highest precedence for operators):
//!
//! ```text
//! program  = item*
//! item     = head vis? (fn_decl | struct | enum | trait | mod | use) | head impl
//! mod      = "mod" IDENT ("{" item* "}" | ";")           -- `;`: in a file of its own
//! use      = "use" tree ";"
//! tree     = path ("as" IDENT)? | path "::" "{" (tree ("," tree)* ","?)? "}"
//! path     = seg ("::" IDENT)*                       -- `a::b::C`
//! seg      = IDENT | "crate" | "super" | "self"
//! vis      = "pub" | "export"
//! head     = attr*
//! attr     = "#" "[" "allow" "(" (IDENT ("," IDENT)* ","?)? ")" "]"
//!          | "#" "[" "derive" "(" (derived ("," derived)* ","?)? ")" "]"
//!          | "#" "[" "intrinsic" "]"               -- on a `fn`, a fieldless `struct` or an
//!                                                     inherent impl's method, in a library
//!          | "#" "[" "measure" "]"                 -- on a `fn`: one the refinement logic
//!                                                     can name (docs/21-measures.md)
//! derived  = IDENT ("where" IDENT ":" bound ("+" bound)* ("," IDENT ":" bound ("+" bound)*)*)?
//! fn_decl  = "fn" IDENT generics? "(" (param ("," param)* ","?)? ")" ("->" type)? where?
//!            (block | ";")                          -- `;` only for an intrinsic
//! where    = "where" type ":" bound ("+" bound)* ("," type ":" bound ("+" bound)*)* ","?
//! generics = "<" (gparam ("," gparam)* ","?)? ">"     -- lifetimes before types
//! gparam   = LIFETIME (":" LIFETIME ("+" LIFETIME)*)? | IDENT (":" bound ("+" bound)*)?
//! bound    = LIFETIME | path                          -- a lifetime, or a trait
//!          | path "<" (IDENT ("=" type | ":" path ("+" path)*)),* ">"
//!          | path "(" (type ("," type)*)? ")" ("->" type)?  -- `Fn(i64) -> i64`
//! param    = "mut"? IDENT ":" type
//! trait    = "trait" IDENT (":" path ("+" path)*)? "{" (head method)* "}"
//! impl     = "impl" generics? (path "for")? type "{" (head vis? method)* "}"
//! method   = "fn" IDENT generics? "(" ((receiver | param) ("," param)* ","?)? ")"
//!            ("->" type)? where? (block | ";")       -- `;` only in a trait
//! receiver = "mut"? "self" | "&" LIFETIME? "mut"? "self"
//! struct   = "struct" IDENT generics? "{" (head vis? param ("," head vis? param)* ","?)? "}"
//! enum     = "enum" IDENT generics? "{" (head variant ("," head variant)* ","?)? "}"
//! variant  = IDENT ("(" type ("," type)* ","? ")")?
//! type     = path ("<" targ ("," targ)* ","? ">")? | "(" ")"
//!          | "fn" "(" (type ("," type)*)? ")" ("->" type)?
//!          | "&" LIFETIME? "mut"? type | "!"
//!          | "impl" bound ("+" bound)*               -- only in a return type
//! targ     = LIFETIME | type
//! block    = "{" (let | head (fn_decl | struct | enum) | expr ";" | block_like ";"?)* expr? "}"
//! block_like = if | match | label? (while | loop | for) | block | "unsafe" block
//! let      = "let" pattern (":" type)? "=" expr ";"
//! expr     = range (("=" | "+=" | "-=" | "*=" | "/=" | "%=") expr)?  -- assignment
//! range    = or (".." or)?                         -- no chaining
//! or       = and ("||" and)*
//! and      = cmp ("&&" cmp)*
//! cmp      = add (("==" | "!=" | "<" | "<=" | ">" | ">=") add)?   -- no chaining
//! add      = mul (("+" | "-") mul)*
//! mul      = cast (("*" | "/" | "%") cast)*
//! cast     = unary ("as" IDENT)*
//! unary    = ("-" | "!" | "*" | "&" "mut"?) unary | call
//! postfix  = primary ("(" args ")" | "." IDENT | "." IDENT targs? "(" args ")")*
//! primary  = INT | "true" | "false" | IDENT targs? | "(" ")" | "(" expr ")" | block
//!          | if | match | label? (while | loop | for) | "unsafe" block
//!          | path targs? "{" field_init,* "}"
//!          | path targs? "::" IDENT ("(" args ")")? | path "::" IDENT targs | lambda
//!          | "return" expr? | "break" LIFETIME? expr? | "continue" LIFETIME?
//! label    = LIFETIME ":"
//! targs    = "::" "<" type ("," type)* ","? ">"            -- the "turbofish"
//! lambda   = "move"? ("||" | "|" (lparam ("," lparam)* ","?)? "|") ("->" type block | expr)
//! lparam   = "mut"? IDENT (":" type)?
//! if       = "if" expr block ("else" (block | if))?
//! while    = "while" expr block
//! loop     = "loop" block
//! for      = "for" pattern "in" expr block
//! match    = "match" expr "{" (pattern "=>" expr ","?)* "}"  -- `,` optional after a block/if/match
//! field_init = IDENT (":" expr)?                             -- `x` is short for `x: x`
//!
//! pattern  = pat_atom ("|" pat_atom)*
//! pat_atom = "_" | "mut"? IDENT | int | pat_range | "true" | "false" | "(" ")"
//!          | path targs? "::" IDENT ("(" pattern,* ")")?
//!          | path targs? "{" (IDENT (":" pattern)?),* ".."? "}"
//! pat_range = int ".." int? | int "..=" int | ".." int | "..=" int
//! int      = "-"? INT
//! ```
//!
//! An `INT` is digits, with `_`s between them if wanted, and a suffix that
//! names an integer type if it says which one it is: `255u8`, `1_000_i64`.
//! A pattern's can't have one: its type is that of what it matches.
//!
//! Like Rust, an `if`, `match` or block at the *start* of a statement is the
//! whole statement, so `if c { a } else { b } - x` is two statements, not a
//! subtraction. Without a `;` such a statement must have type `()` (checked in
//! `check/expr.rs`), which turns that mistake into a type error. Parenthesize to
//! use one in a larger expression: `(if c { a } else { b }) - x`.
//!
//! `return` and `break` take an expression after them if one can start
//! there (after `break`'s label, if it has one), and it runs as far right as an expression can, as a closure's
//! body does: `return a + b` returns `a + b`.
//!
//! A `fn`, `struct` or `enum` in a block is local to it (see `ast::Stmt`).
//! A `trait`, `impl`, `mod` or `use` can only be in a module.
//!
//! Which names in a path are modules isn't known here: `a::f(1)` reads as
//! variant `f` of a type `a`, and `a::Point` in a type as the projection
//! `T::Item` does, until `check/modules.rs` resolves names and rewrites them.
//!
//! `self` and `Self` are names, not keywords, but `self` can only be a
//! method's receiver: no other parameter or pattern can bind it.
//!
//! `&&` is one token, for "and", so where a `&` starts a type or an
//! expression, `&&` is two of them: `&&x` is `&(&x)`.
//!
//! A `///` doc comment documents the `fn`, `struct`, `enum`, field or
//! variant after it. The parser takes doc comments out of the tokens first,
//! and attaches each run of them to what starts right after it (`doc`). The
//! ones nothing takes are `Program::stray_docs`. The `#[allow(..)]`
//! attributes before something can come before or after its doc comment.
//!
//! One ambiguity: in `if x { .. }` the `x {` could start a struct literal. Like
//! Rust, struct literals aren't allowed directly in the head of an `if` or
//! `match` (write `if (P { .. }) == p` instead). `no_struct` tracks that.

use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::lexer::{OpEq, Tok, Token};
use std::sync::Arc;

/// Parses a file as the crate root. A `mod m;` in it is a module of no
/// items yet, `in_file`: `load.rs` reads its file and parses that into it
/// with `parse_into`.
pub fn parse(tokens: Vec<Token>) -> Result<Program, Error> {
    let mut prog = empty_program(&tokens);
    parse_into(tokens, &mut prog, ROOT)?;
    Ok(prog)
}

/// A partial parse for editor features only. It must not enter checking or
/// executable lowering: a missing item can change what the program means.
pub(crate) struct EditorParse {
    pub program: Program,
    pub errors: Vec<Error>,
}

/// Keeps complete items around syntax errors, without inventing AST nodes.
/// Lexing remains strict; callers handle lexical errors separately.
pub(crate) fn parse_for_editor(tokens: Vec<Token>) -> EditorParse {
    let mut program = empty_program(&tokens);
    let mut p = Parser::new(tokens, ROOT, program.next_id);
    p.errors = Some(Vec::new());
    if let Err(e) = p.items(&mut program, Tok::Eof) {
        p.errors.as_mut().unwrap().push(e);
    }
    program.next_id = p.next_id;
    program.stray_docs.extend(p.stray_docs());
    EditorParse {
        program,
        errors: p.errors.unwrap(),
    }
}

fn empty_program(tokens: &[Token]) -> Program {
    let end = tokens.last().map_or(0, |t| t.span.end);
    Program {
        mods: vec![ModDecl {
            doc: None,
            allows: Vec::new(),
            span: Span::new(0, end),
            name: Ident {
                name: "crate".into(),
                span: Span::new(0, 0),
                id: NodeId(0),
            },
            parent: None,
            vis: Vis::Private,
            in_file: false,
        }],
        core: None,
        libraries: Vec::new(),
        uses: Vec::new(),
        types: Vec::new(),
        traits: Vec::new(),
        impls: Vec::new(),
        fns: Vec::new(),
        stray_docs: Vec::new(),
        next_id: 1,
        proved: Default::default(),
        measures: Default::default(),
    }
}

/// Parses a file's items into `module` of `prog`.
pub fn parse_into(tokens: Vec<Token>, prog: &mut Program, module: usize) -> Result<(), Error> {
    let mut p = Parser::new(tokens, module, prog.next_id);
    p.items(prog, Tok::Eof)?;
    prog.next_id = p.next_id;
    prog.stray_docs.extend(p.stray_docs());
    Ok(())
}

struct Parser {
    /// Every token but doc comments.
    toks: Vec<Token>,
    /// The doc comments, `///` and `//!` in order, and whether each is
    /// attached to something yet.
    docs: Vec<Token>,
    attached: Vec<bool>,
    pos: usize,
    /// Set while parsing the head of an `if` or `match`, where `x {` must not
    /// be read as a struct literal.
    no_struct: bool,
    /// The module being parsed, by its index in `Program::mods`.
    module: usize,
    /// The id the next node gets: `Program::next_id`, while parsing.
    next_id: u32,
    /// Only editor parsing recovers; the compiler and formatter fail fast.
    errors: Option<Vec<Error>>,
}

impl Parser {
    fn new(tokens: Vec<Token>, module: usize, next_id: u32) -> Self {
        let (docs, toks): (Vec<Token>, Vec<Token>) = tokens
            .into_iter()
            .partition(|t| matches!(t.tok, Tok::Doc(_) | Tok::InnerDoc(_)));
        Self {
            toks,
            attached: vec![false; docs.len()],
            docs,
            pos: 0,
            no_struct: false,
            module,
            next_id,
            errors: None,
        }
    }

    /// An id for a new node (see `NodeId`).
    fn id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

    /// `a::b` from `a` and `b`: one name for a path, spanning both.
    fn join(&mut self, a: &Ident, b: &Ident) -> Ident {
        Ident {
            name: format!("{}::{}", a.name, b.name),
            span: a.span.to(b.span),
            id: self.id(),
        }
    }

    /// The segments of a path, as one name: see `Parser::path`.
    fn join_all(&mut self, segs: &[&Ident]) -> Ident {
        let mut out = segs[0].clone();
        for s in &segs[1..] {
            out = self.join(&out, s);
        }
        out
    }

    /// A type written as a plain path, `Show` or `a::Show`, as the one name
    /// `Parser::path` would make of it: where a trait's name is expected.
    fn type_path(&mut self, t: &TypeExpr) -> Option<Ident> {
        match &t.kind {
            TypeExprKind::Named(name, args) if args.is_empty() => Some(Ident {
                name: name.clone(),
                span: t.span,
                id: self.id(),
            }),
            TypeExprKind::Proj(inner, None, name) => {
                let inner = self.type_path(inner)?;
                Some(self.join(&inner, name))
            }
            _ => None,
        }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok
    }

    /// Runs `f` with struct literals allowed (`true`) or not, then restores the
    /// previous setting. Inside any brackets they're allowed again, since
    /// there's no ambiguity there.
    fn structs<T>(&mut self, allowed: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        let saved = std::mem::replace(&mut self.no_struct, !allowed);
        let result = f(self);
        self.no_struct = saved;
        result
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    /// The doc comment for what starts at the next token: the `///` lines
    /// between the token before and it, if there are any.
    fn doc(&mut self) -> Option<DocComment> {
        self.docs_before(false)
    }

    /// The `//!` lines between the token before and the next, if there are
    /// any: the doc comment of the module whose body starts there.
    fn inner_doc(&mut self) -> Option<DocComment> {
        self.docs_before(true)
    }

    /// The doc comment lines of the kind `inner` says between the token
    /// before and the next. Those of the other kind are left for others,
    /// or to be stray.
    fn docs_before(&mut self, inner: bool) -> Option<DocComment> {
        let from = match self.pos {
            0 => 0,
            i => self.toks[i - 1].span.end,
        };
        let to = self.span().start;
        let first = self.docs.partition_point(|d| d.span.start < from);
        let last = self.docs.partition_point(|d| d.span.start < to);
        let kind = |d: &Token| matches!(d.tok, Tok::InnerDoc(_)) == inner;
        let lines: Vec<Token> = (first..last)
            .filter(|&i| kind(&self.docs[i]))
            .map(|i| {
                self.attached[i] = true;
                self.docs[i].clone()
            })
            .collect();
        (!lines.is_empty()).then(|| doc_comment(&lines))
    }

    /// The doc comment and attributes of what starts at the next token, or
    /// after the attributes there.
    fn head(&mut self) -> Result<Head, Error> {
        let mut doc = self.doc();
        let mut allows = Vec::new();
        let mut derives = Vec::new();
        let mut intrinsic = None;
        let mut measure = None;
        while *self.peek() == Tok::Hash {
            let start = self.bump().span;
            self.expect(Tok::LBracket)?;
            let derive = match self.peek() {
                Tok::Ident(name) if name == "allow" || name == "derive" => name == "derive",
                Tok::Ident(name) if name == "intrinsic" => {
                    self.bump();
                    intrinsic = Some(start.to(self.expect(Tok::RBracket)?));
                    continue;
                }
                Tok::Ident(name) if name == "measure" => {
                    self.bump();
                    measure = Some(start.to(self.expect(Tok::RBracket)?));
                    continue;
                }
                Tok::Ident(name) => {
                    let message = format!(
                        "unknown attribute `{name}`; the only ones are `allow`, `derive`, \
                         `intrinsic` and `measure`"
                    );
                    return Err(Error::syntax(self.span(), message));
                }
                _ => return Err(self.unexpected("`allow`, `derive`, `intrinsic` or `measure`")),
            };
            self.bump();
            self.expect(Tok::LParen)?;
            let (names, wheres) = match derive {
                true => self.derive_list()?,
                false => (self.comma_list(Tok::RParen, Self::ident)?.0, Vec::new()),
            };
            let end = self.expect(Tok::RBracket)?;
            let span = start.to(end);
            if derive {
                derives.push(Derive {
                    span,
                    names,
                    wheres,
                });
            } else {
                allows.push(Allow { span, lints: names });
            }
        }
        if !(allows.is_empty() && derives.is_empty())
            && let Some(after) = self.doc()
        {
            match &mut doc {
                Some(d) => d.lines.extend(after.lines),
                None => doc = Some(after),
            }
        }
        Ok(Head {
            doc,
            allows,
            derives,
            intrinsic,
            measure,
        })
    }

    /// `Discard, Eq where T: Eq, U: Show)` in a `#[derive(..)]`, after
    /// the `(`: each trait, with the `where` after it, if it has one. The
    /// bounds of a `where` go on while a name and a `:` come after a `,`.
    fn derive_list(&mut self) -> Result<(Vec<Ident>, Vec<Option<Where>>), Error> {
        let mut names = Vec::new();
        let mut wheres = Vec::new();
        while *self.peek() != Tok::RParen {
            names.push(self.path()?);
            let w = match *self.peek() == Tok::Where {
                false => None,
                true => {
                    let start = self.bump().span;
                    let mut bounds = Vec::new();
                    let end = loop {
                        let param = self.ident()?;
                        self.expect(Tok::Colon)?;
                        let mut items = vec![self.trait_bound()?];
                        while self.eat(&Tok::Plus) {
                            items.push(self.trait_bound()?);
                        }
                        let end = items[items.len() - 1].span;
                        bounds.push((param, items));
                        let more = *self.peek() == Tok::Comma
                            && matches!(self.peek2(), Tok::Ident(_))
                            && self
                                .toks
                                .get(self.pos + 2)
                                .is_some_and(|t| t.tok == Tok::Colon);
                        if !more {
                            break end;
                        }
                        self.bump();
                    };
                    Some(Where {
                        span: start.to(end),
                        bounds,
                    })
                }
            };
            wheres.push(w);
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::RParen)?;
        Ok((names, wheres))
    }

    /// The doc comments nothing took, each run of lines of one kind with
    /// no other token between them as one.
    fn stray_docs(&self) -> Vec<DocComment> {
        let mut runs: Vec<Vec<Token>> = Vec::new();
        let mut prev = None;
        for (d, _) in self.docs.iter().zip(&self.attached).filter(|(_, a)| !**a) {
            // The first token after it: the same for lines with nothing
            // between them.
            let next_tok = self.toks.partition_point(|t| t.span.start < d.span.start);
            let this = (next_tok, matches!(d.tok, Tok::InnerDoc(_)));
            match runs.last_mut() {
                Some(run) if prev == Some(this) => run.push(d.clone()),
                _ => runs.push(vec![d.clone()]),
            }
            prev = Some(this);
        }
        runs.iter().map(|run| doc_comment(run)).collect()
    }

    /// The items of the module being parsed, up to `close`: `}`, or the end
    /// of the file for the crate root, and its `//!` doc comment, first.
    fn items(&mut self, prog: &mut Program, close: Tok) -> Result<(), Error> {
        if let Some(doc) = self.inner_doc() {
            prog.mods[self.module].doc = Some(doc);
        }
        while *self.peek() != close {
            if *self.peek() == Tok::Eof {
                return Err(self.unexpected(&close.describe()));
            }
            let start = self.pos;
            // An inline module can add children before it fails. Roll all
            // of them back together, so no item has a dangling module index.
            let counts = self.errors.as_ref().map(|_| {
                [
                    prog.mods.len(),
                    prog.uses.len(),
                    prog.types.len(),
                    prog.traits.len(),
                    prog.impls.len(),
                    prog.fns.len(),
                ]
            });
            if let Err(e) = self.item(prog) {
                let Some(errors) = &mut self.errors else {
                    return Err(e);
                };
                errors.push(e);
                let [mods, uses, types, traits, impls, fns] = counts.unwrap();
                prog.mods.truncate(mods);
                prog.uses.truncate(uses);
                prog.types.truncate(types);
                prog.traits.truncate(traits);
                prog.impls.truncate(impls);
                prog.fns.truncate(fns);
                self.recover_item(start, &close);
            }
        }
        if close != Tok::Eof {
            self.bump();
        }
        Ok(())
    }

    fn item(&mut self, prog: &mut Program) -> Result<(), Error> {
        let head = self.head()?;
        let vis = self.vis();
        let head = match self.peek() {
            Tok::Fn | Tok::Struct | Tok::Trait => head.no_measure(NOT_METHOD)?,
            _ => head.no_intrinsic()?.no_measure(NOT_METHOD)?,
        };
        match self.peek() {
            Tok::Struct => {
                let mut d = self.struct_decl(head)?;
                d.vis = vis;
                prog.types.push(TypeDecl::Struct(d));
            }
            Tok::Enum => {
                let mut d = self.enum_decl(head)?;
                d.vis = vis;
                prog.types.push(TypeDecl::Enum(d));
            }
            Tok::Type => {
                let mut d = self.alias_decl(head.no_derives()?)?;
                d.vis = vis;
                prog.types.push(TypeDecl::Alias(d));
            }
            Tok::Trait => {
                let mut d = self.trait_decl(head.no_derives()?)?;
                d.vis = vis;
                prog.traits.push(d);
            }
            Tok::Impl => {
                if let Some(at) = vis.span() {
                    let word = vis.keyword().trim();
                    return Err(Error::syntax(
                        at,
                        format!(
                            "an `impl` can't be `{word}`: a trait impl is wherever the trait \
                             and type are, and an inherent impl's methods each say if they are"
                        ),
                    )
                    .with_fix(Fix::new(
                        format!("remove the `{word}`"),
                        vec![Edit::delete(Span::new(at.start, self.span().start))],
                    )));
                }
                prog.impls.push(self.impl_decl(head.no_derives()?)?);
            }
            Tok::Mod => self.mod_decl(prog, head.no_derives()?, vis)?,
            Tok::Use => {
                head.no_derives()?.no_allows_on("a `use`")?;
                let d = self.use_decl(vis)?;
                prog.uses.push(d);
            }
            _ => {
                let mut f = self.fn_decl(head.no_derives()?)?;
                f.vis = vis;
                prog.fns.push(f);
            }
        }
        Ok(())
    }

    /// Skip the failed item, including its body and any local declarations.
    /// Replay its consumed tokens to recover brace depth. Parentheses may
    /// be incomplete, so a following item can still start after a bad header.
    fn recover_item(&mut self, start: usize, close: &Tok) {
        let failed_at = self.pos;
        self.pos = start;
        let mut braces = 0usize;
        let mut brackets = 0usize;
        let mut head = matches!(self.peek(), Tok::Hash | Tok::Pub | Tok::Export);
        loop {
            let item = matches!(
                self.peek(),
                Tok::Fn
                    | Tok::Struct
                    | Tok::Enum
                    | Tok::Type
                    | Tok::Trait
                    | Tok::Impl
                    | Tok::Mod
                    | Tok::Use
            );
            let prefix = matches!(self.peek(), Tok::Hash | Tok::Pub | Tok::Export);
            if braces == 0 && brackets == 0 {
                if head && item {
                    // Attributes and visibility belong to this failed item.
                    head = false;
                } else if self.pos > start
                    && self.pos >= failed_at
                    && !head
                    && (item || prefix)
                    && !matches!((self.peek(), self.peek2()), (Tok::Fn, Tok::LParen))
                {
                    return;
                }
            }
            match self.peek() {
                Tok::Eof => return,
                Tok::RBrace if braces == 0 && *close == Tok::RBrace => return,
                Tok::LBrace => braces += 1,
                Tok::RBrace if braces > 0 => {
                    braces -= 1;
                    if braces == 0 && self.pos >= failed_at {
                        self.bump();
                        return;
                    }
                }
                Tok::LBracket => brackets += 1,
                Tok::RBracket => brackets = brackets.saturating_sub(1),
                Tok::Semi if braces == 0 && brackets == 0 => {
                    self.bump();
                    return;
                }
                _ => {}
            }
            self.bump();
        }
    }

    /// `mod name { .. }` or `mod name;`, after its head and `pub` or
    /// `export`, if it has one.
    fn mod_decl(&mut self, prog: &mut Program, head: Head, vis: Vis) -> Result<(), Error> {
        let start = vis.span().unwrap_or(self.span());
        self.expect(Tok::Mod)?;
        let name = self.binder()?;
        if let Some((lib, ..)) = crate::load::LIBRARIES
            .iter()
            .find(|(l, ..)| name.name == *l)
        {
            return Err(Error::syntax(
                name.span,
                format!("a module can't be called `{lib}`: that's the name of the {lib} library"),
            ));
        }
        let in_file = *self.peek() == Tok::Semi;
        // One place for a module's docs: in it, as for one in a file.
        if let Some(doc) = &head.doc {
            let e = Error::syntax(
                doc.span(),
                match in_file {
                    true => format!(
                        "a module's doc comment goes at the start of its file, as `//!` lines, \
                         not before `mod {};`",
                        name.name
                    ),
                    false => "a module's doc comment goes inside it, as `//!` lines at the \
                              start"
                        .to_string(),
                },
            );
            if in_file || *self.peek() != Tok::LBrace {
                return Err(e);
            }
            let mut edits: Vec<Edit> = doc.lines.iter().map(|l| Edit::delete(l.span)).collect();
            let inner: String = doc
                .lines
                .iter()
                .map(|l| format!("\n//!{}", l.text))
                .collect();
            edits.push(Edit::insert(self.span().end, inner));
            return Err(e.with_fix(Fix::new("move it inside, as `//!`", edits)));
        }
        let decl = ModDecl {
            doc: None,
            allows: head.allows,
            span: start,
            name,
            parent: Some(self.module),
            vis,
            in_file,
        };
        if in_file {
            let end = self.bump().span;
            prog.mods.push(ModDecl {
                span: start.to(end),
                ..decl
            });
            return Ok(());
        }
        self.expect(Tok::LBrace)?;
        let index = prog.mods.len();
        prog.mods.push(decl);
        let outer = std::mem::replace(&mut self.module, index);
        let result = self.items(prog, Tok::RBrace);
        self.module = outer;
        result?;
        prog.mods[index].span = start.to(self.toks[self.pos - 1].span);
        Ok(())
    }

    /// `use a::b;`, `use a::b as c;` or `use a::{b, c};`, after its `pub`
    /// or `export`.
    fn use_decl(&mut self, vis: Vis) -> Result<UseDecl, Error> {
        let start = vis.span().unwrap_or(self.span());
        self.expect(Tok::Use)?;
        let tree = self.use_tree()?;
        let end = self.expect(Tok::Semi)?;
        Ok(UseDecl {
            span: start.to(end),
            module: self.module,
            vis,
            tree,
        })
    }

    fn use_tree(&mut self) -> Result<UseTree, Error> {
        let start = self.span();
        let mut path = Vec::new();
        loop {
            match self.peek() {
                Tok::LBrace if !path.is_empty() => {
                    self.bump();
                    let (trees, end) = self.comma_list(Tok::RBrace, Self::use_tree)?;
                    return Ok(UseTree {
                        path,
                        kind: UseKind::Group(trees),
                        span: start.to(end),
                    });
                }
                Tok::Star => {
                    return Err(Error::syntax(
                        self.span(),
                        "glob imports (`*`) aren't supported: name each item, as in \
                         `use a::{b, c};`",
                    ));
                }
                _ => path.push(self.segment()?),
            }
            if !self.eat(&Tok::ColonColon) {
                break;
            }
        }
        let alias = match self.eat(&Tok::As) {
            true => Some(self.binder()?),
            false => None,
        };
        let end = alias.as_ref().map_or(path[path.len() - 1].span, |a| a.span);
        Ok(UseTree {
            path,
            kind: UseKind::Name(alias),
            span: start.to(end),
        })
    }

    /// One segment of a path: a name, or `crate`, `super` or `self`.
    fn segment(&mut self) -> Result<Ident, Error> {
        let name = match self.peek() {
            Tok::Crate => "crate",
            Tok::Super => "super",
            _ => return self.ident(),
        };
        Ok(Ident {
            name: name.into(),
            span: self.bump().span,
            id: self.id(),
        })
    }

    /// `a::b::Show`: a path of plain names, as one name, its segments
    /// joined by `::`, spanning them all. A path to a trait is written
    /// like this.
    fn path(&mut self) -> Result<Ident, Error> {
        let mut path = self.segment()?;
        while *self.peek() == Tok::ColonColon && matches!(self.peek2(), Tok::Ident(_)) {
            self.bump();
            let next = self.ident()?;
            path = self.join(&path, &next);
        }
        Ok(path)
    }

    /// `a::Shape::<i64>::Dot`: names with `::` between, each maybe with
    /// type arguments, up to a `::` that has neither a name nor `<`
    /// after it.
    fn segments(&mut self) -> Result<Vec<Segment>, Error> {
        let mut segs = Vec::new();
        loop {
            let name = self.segment()?;
            let (targs, end) = self.turbofish()?;
            segs.push((name, targs, end));
            if *self.peek() == Tok::ColonColon && matches!(self.peek2(), Tok::Ident(_)) {
                self.bump();
            } else {
                return Ok(segs);
            }
        }
    }

    /// `pub` or `export`, if one is next.
    fn vis(&mut self) -> Vis {
        match self.peek() {
            Tok::Pub => Vis::Pub(self.bump().span),
            Tok::Export => Vis::Export(self.bump().span),
            _ => Vis::Private,
        }
    }

    /// Checks there's no `pub` or `export` next, where nothing can be:
    /// `why`.
    fn no_pub(&mut self, why: &str) -> Result<(), Error> {
        let word = match self.peek() {
            Tok::Pub => "pub",
            Tok::Export => "export",
            _ => return Ok(()),
        };
        let at = self.span();
        let next = self.toks[self.pos + 1].span;
        Err(
            Error::syntax(at, format!("this can't be `{word}`: {why}")).with_fix(Fix::new(
                format!("remove the `{word}`"),
                vec![Edit::delete(Span::new(at.start, next.start))],
            )),
        )
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if t.tok != Tok::Eof {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == tok {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, tok: Tok) -> Result<Span, Error> {
        if *self.peek() == tok {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(&tok.describe()))
        }
    }

    fn unexpected(&self, wanted: &str) -> Error {
        Error::syntax(
            self.span(),
            format!("expected {wanted}, found {}", self.peek().describe()),
        )
    }

    /// A name being bound: anything but `self`, which only a receiver
    /// binds.
    fn binder(&mut self) -> Result<Ident, Error> {
        let name = self.ident()?;
        match name.name.as_str() {
            "self" => Err(Error::syntax(
                name.span,
                "only a method's receiver can be called `self`",
            )),
            _ => Ok(name),
        }
    }

    fn ident(&mut self) -> Result<Ident, Error> {
        match self.peek().clone() {
            Tok::Ident(name) => Ok(Ident {
                name,
                span: self.bump().span,
                id: self.id(),
            }),
            _ => Err(self.unexpected("a name")),
        }
    }

    /// Parses `item ("," item)* ","? close`, after the opening delimiter.
    fn comma_list<T>(
        &mut self,
        close: Tok,
        mut item: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<(Vec<T>, Span), Error> {
        let mut items = Vec::new();
        loop {
            if *self.peek() == close {
                break;
            }
            items.push(item(self)?);
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        let end = self.expect(close)?;
        Ok((items, end))
    }

    /// `comma_list`, but leaving `close` for the caller to take.
    fn comma_list_open<T>(
        &mut self,
        close: Tok,
        mut item: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let mut items = Vec::new();
        while *self.peek() != close {
            items.push(item(self)?);
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        Ok(items)
    }

    fn fn_decl(&mut self, head: Head) -> Result<FnDecl, Error> {
        if *self.peek() != Tok::Fn {
            return Err(self.unexpected("`fn`, `struct`, `enum`, `type`, `trait` or `impl`"));
        }
        Ok(self.function(head, false, false)?.0)
    }

    /// A function, after its head. A method (`method`) can take `self` and,
    /// in a trait (`in_trait`), have no body. Says whether it has one.
    fn function(
        &mut self,
        head: Head,
        method: bool,
        in_trait: bool,
    ) -> Result<(FnDecl, bool), Error> {
        let start = self.expect(Tok::Fn)?;
        let name = self.ident()?;
        let generics = self.generics()?;
        let open = self.expect(Tok::LParen)?;
        let mut first = true;
        let (params, close) = self.comma_list(Tok::RParen, |p| {
            let receiver = std::mem::take(&mut first) && p.at_receiver();
            match receiver {
                true if method => p.receiver(),
                true => Err(Error::syntax(
                    p.span(),
                    "only a method can take `self`: a function in a `trait` or `impl`",
                )),
                false => p.param(),
            }
        })?;
        let ret = if self.eat(&Tok::Arrow) {
            self.ty()?
        } else {
            TypeExpr {
                kind: TypeExprKind::Unit,
                span: close,
                id: self.id(),
            }
        };
        let effects = self.with_effects()?;
        let mut generics = generics;
        if *self.peek() == Tok::Where {
            self.where_clause(&mut generics)?;
        }
        let intrinsic = head.intrinsic;
        let (body, provided) = match self.peek() {
            Tok::Semi if in_trait || intrinsic.is_some() => {
                let span = self.bump().span;
                let body = Expr {
                    kind: ExprKind::Block(Vec::new(), None),
                    span,
                    id: self.id(),
                };
                (body, false)
            }
            Tok::Semi => {
                return Err(Error::syntax(
                    self.span(),
                    "a function needs a body; only a method in a `trait` can have none",
                ));
            }
            _ if intrinsic.is_some() => {
                return Err(Error::syntax(
                    self.span(),
                    "an intrinsic has no body: the compiler provides it; end it with `;`",
                ));
            }
            _ => (self.block()?, true),
        };
        let f = FnDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(body.span),
            module: self.module,
            vis: Vis::Private,
            name,
            generics,
            params,
            parens: open.to(close),
            ret,
            effects,
            body,
            intrinsic,
            measure: head.measure,
        };
        Ok((f, provided))
    }

    /// `where T: Show, Self::Item: Discard` after a function's signature,
    /// or an impl's type, added to its generics. The bounds go on while a `,` is followed by
    /// something other than the body.
    fn where_clause(&mut self, g: &mut Generics) -> Result<(), Error> {
        let start = self.expect(Tok::Where)?;
        let mut end;
        loop {
            let subject = self.ty()?;
            self.expect(Tok::Colon)?;
            let mut items = Vec::new();
            loop {
                items.push(match self.peek() {
                    Tok::Lifetime(_) => BoundItem::plain(self.lifetime()?),
                    _ => self.trait_bound()?,
                });
                if !self.eat(&Tok::Plus) {
                    break;
                }
            }
            end = items[items.len() - 1].span;
            g.wheres.push((subject, items));
            if !self.eat(&Tok::Comma) || matches!(self.peek(), Tok::LBrace | Tok::Semi) {
                break;
            }
        }
        g.where_span = Some(start.to(end));
        Ok(())
    }

    /// Is a receiver next: `self`, `mut self`, `&self`, `&'a mut self`..?
    fn at_receiver(&self) -> bool {
        let at = |i: usize| &self.toks[(self.pos + i).min(self.toks.len() - 1)].tok;
        let mut i = 0;
        if *at(i) == Tok::Amp {
            i += 1;
            if let Tok::Lifetime(_) = at(i) {
                i += 1;
            }
        }
        if *at(i) == Tok::Mut {
            i += 1;
        }
        matches!(at(i), Tok::Ident(n) if n == "self")
    }

    /// A method's receiver, as a parameter named `self` of type `Self`,
    /// `&Self` or `&mut Self`, the type spanning what's written.
    fn receiver(&mut self) -> Result<Param, Error> {
        let start = self.span();
        let by_ref = self.eat(&Tok::Amp);
        let lifetime = match self.peek() {
            Tok::Lifetime(_) => Some(self.lifetime()?),
            _ => None,
        };
        let mut_span = (*self.peek() == Tok::Mut).then(|| self.bump().span);
        let name = self.ident()?;
        if *self.peek() == Tok::Colon {
            return Err(Error::syntax(
                self.span(),
                "a receiver's type isn't written: it's `self`, `&self` or `&mut self`",
            ));
        }
        let self_ty = TypeExpr {
            kind: TypeExprKind::Named("Self".into(), Vec::new()),
            span: name.span,
            id: self.id(),
        };
        let (mutable, ty) = match by_ref {
            false => (mut_span, self_ty),
            true => {
                let kind = TypeExprKind::Ref(lifetime, mut_span.is_some(), Box::new(self_ty));
                let span = start.to(name.span);
                (
                    None,
                    TypeExpr {
                        kind,
                        span,
                        id: self.id(),
                    },
                )
            }
        };
        Ok(Param {
            doc: None,
            allows: Vec::new(),
            mutable,
            vis: Vis::Private,
            name,
            ty,
        })
    }

    /// `trait Show: Super { .. }`, after its head.
    fn trait_decl(&mut self, head: Head) -> Result<TraitDecl, Error> {
        let start = self.expect(Tok::Trait)?;
        let name = self.ident()?;
        if *self.peek() == Tok::Lt {
            return Err(Error::syntax(
                self.span(),
                "a trait can't have type parameters (yet); its methods can",
            ));
        }
        let mut supers = Vec::new();
        if self.eat(&Tok::Colon) {
            supers.push(self.path()?);
            while self.eat(&Tok::Plus) {
                supers.push(self.path()?);
            }
        }
        self.expect(Tok::LBrace)?;
        let mut assoc = Vec::new();
        let mut effects = Vec::new();
        let mut methods = Vec::new();
        while *self.peek() != Tok::RBrace {
            let head = (self.head()?.no_derives()?.no_intrinsic()?).no_measure(TRAIT_MEASURE)?;
            self.no_pub("a trait's methods are public wherever the trait is")?;
            match self.peek() {
                Tok::Type => assoc.push(self.assoc_decl(head)?),
                Tok::Effect => effects.push(self.effect_decl(head)?),
                Tok::Fn => {
                    let (decl, provided) = self.function(head, true, true)?;
                    methods.push(TraitMethod { decl, provided });
                }
                _ => return Err(self.unexpected("`fn`, `type`, `effect` or `}`")),
            }
        }
        let end = self.expect(Tok::RBrace)?;
        Ok(TraitDecl {
            doc: head.doc,
            allows: head.allows,
            intrinsic: head.intrinsic,
            span: start.to(end),
            module: self.module,
            vis: Vis::Private,
            name,
            supers,
            assoc,
            effects,
            methods,
        })
    }

    /// `type Item: Discard;` in a trait, after its head.
    fn assoc_decl(&mut self, head: Head) -> Result<AssocDecl, Error> {
        let start = self.expect(Tok::Type)?;
        head.no_allows()?;
        let name = self.ident()?;
        let mut bounds = Vec::new();
        if self.eat(&Tok::Colon) {
            bounds.push(self.path()?);
            while self.eat(&Tok::Plus) {
                bounds.push(self.path()?);
            }
        }
        if *self.peek() == Tok::Eq {
            return Err(Error::syntax(
                self.span(),
                "an associated type can't have a default: a default method could then be \
                 checked knowing what it is, and an impl that picks another would make that \
                 wrong; each impl says what it is",
            ));
        }
        let end = self.expect(Tok::Semi)?;
        Ok(AssocDecl {
            doc: head.doc,
            span: start.to(end),
            name,
            bounds,
        })
    }

    /// `type Item = i64;` in an impl, after its head.
    fn assoc_def(&mut self, head: Head) -> Result<AssocDef, Error> {
        let start = self.expect(Tok::Type)?;
        head.no_allows()?;
        let name = self.ident()?;
        self.expect(Tok::Eq)?;
        let ty = self.ty()?;
        let end = self.expect(Tok::Semi)?;
        Ok(AssocDef {
            doc: head.doc,
            span: start.to(end),
            name,
            ty,
        })
    }

    /// `effect e;` in a trait, after its head.
    fn effect_decl(&mut self, head: Head) -> Result<EffectDecl, Error> {
        let start = self.expect(Tok::Effect)?;
        head.no_allows()?;
        let name = self.ident()?;
        if *self.peek() == Tok::Eq {
            return Err(Error::syntax(
                self.span(),
                "an associated effect can't have a default, as an associated type can't: \
                 each impl says what it is",
            ));
        }
        let end = self.expect(Tok::Semi)?;
        Ok(EffectDecl {
            doc: head.doc,
            span: start.to(end),
            name,
        })
    }

    /// `effect e = panic;` in an impl, after its head.
    fn effect_def(&mut self, head: Head) -> Result<EffectDef, Error> {
        let start = self.expect(Tok::Effect)?;
        head.no_allows()?;
        let name = self.ident()?;
        self.expect(Tok::Eq)?;
        let effects = self.effect_set(None)?;
        let end = self.expect(Tok::Semi)?;
        Ok(EffectDef {
            doc: head.doc,
            span: start.to(end),
            name,
            effects,
        })
    }

    /// `with panic` or `with {alloc, e}`, if a `with` is next.
    fn with_effects(&mut self) -> Result<Option<EffectSet>, Error> {
        match *self.peek() == Tok::With {
            true => {
                let with = self.bump().span;
                Ok(Some(self.effect_set(Some(with))?))
            }
            false => Ok(None),
        }
    }

    /// One effect, or a set of them in braces, after the `with` at `with`
    /// if there is one.
    fn effect_set(&mut self, with: Option<Span>) -> Result<EffectSet, Error> {
        let start = with.unwrap_or_else(|| self.span());
        if self.eat(&Tok::LBrace) {
            let (items, end) = self.comma_list(Tok::RBrace, Self::effect)?;
            return Ok(EffectSet {
                items,
                span: start.to(end),
                braced: true,
                with: with.is_some(),
            });
        }
        let item = self.effect()?;
        Ok(EffectSet {
            span: start.to(item.span),
            items: vec![item],
            braced: false,
            with: with.is_some(),
        })
    }

    /// `panic`, `alloc`, `unsafe`, an effect parameter `e`, or an
    /// associated effect `I::e` or `<I as Iterator>::e`.
    fn effect(&mut self) -> Result<Effect, Error> {
        if *self.peek() == Tok::Unsafe {
            let span = self.bump().span;
            let name = Ident {
                name: "unsafe".into(),
                span,
                id: self.id(),
            };
            return Ok(Effect {
                kind: EffectKind::Named(name),
                span,
                id: self.id(),
            });
        }
        let t = self.ty()?;
        let kind = match t.kind {
            TypeExprKind::Named(name, args) if args.is_empty() && !name.contains("::") => {
                EffectKind::Named(Ident {
                    name,
                    span: t.span,
                    id: t.id,
                })
            }
            TypeExprKind::Proj(base, tr, name) => EffectKind::Proj(base, tr, name),
            _ => {
                return Err(Error::syntax(
                    t.span,
                    "expected an effect: `panic`, `alloc`, `unsafe`, an effect parameter like `e`, \
                     or an associated effect like `I::e`",
                ));
            }
        };
        Ok(Effect {
            kind,
            span: t.span,
            id: self.id(),
        })
    }

    /// `impl<T> Show for Option<T> { .. }` or `impl Point { .. }`, after
    /// its head.
    fn impl_decl(&mut self, head: Head) -> Result<ImplDecl, Error> {
        let start = self.expect(Tok::Impl)?;
        let mut generics = self.generics()?;
        let first = self.ty()?;
        let (trait_name, self_ty) = match self.eat(&Tok::For) {
            false => (None, first),
            true => match self.type_path(&first) {
                Some(name) => (Some(name), self.ty()?),
                None => {
                    return Err(Error::syntax(
                        first.span,
                        "expected the name of a trait before `for`",
                    ));
                }
            },
        };
        if *self.peek() == Tok::Where {
            self.where_clause(&mut generics)?;
        }
        self.expect(Tok::LBrace)?;
        let mut assoc = Vec::new();
        let mut effects = Vec::new();
        let mut methods = Vec::new();
        while *self.peek() != Tok::RBrace {
            let head = self.head()?.no_derives()?;
            let head = match trait_name {
                None => head,
                Some(_) => head.no_intrinsic()?.no_measure(TRAIT_MEASURE)?,
            };
            let vis = match trait_name {
                Some(_) => {
                    self.no_pub("a trait impl's methods are public wherever the trait is")?;
                    Vis::Private
                }
                None => self.vis(),
            };
            match self.peek() {
                Tok::Type => assoc.push(self.assoc_def(head)?),
                Tok::Effect => effects.push(self.effect_def(head)?),
                Tok::Fn => {
                    let mut f = self.function(head, true, false)?.0;
                    f.vis = vis;
                    methods.push(f);
                }
                _ => return Err(self.unexpected("`fn`, `type`, `effect` or `}`")),
            }
        }
        let end = self.expect(Tok::RBrace)?;
        Ok(ImplDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            module: self.module,
            generics,
            trait_name,
            self_ty,
            assoc,
            effects,
            methods,
        })
    }

    /// `<'a, T: Show + 'a>` after the name of a function, struct or enum,
    /// or after `impl`, if it's there.
    fn generics(&mut self) -> Result<Generics, Error> {
        let mut g = Generics::default();
        if *self.peek() != Tok::Lt {
            return Ok(g);
        }
        let start = self.bump().span;
        let (_, end) = self.comma_list(Tok::Gt, |p| {
            let param = match p.peek() {
                Tok::Lifetime(_) if !g.types.is_empty() => {
                    return Err(Error::syntax(
                        p.span(),
                        "lifetime parameters must come before type parameters",
                    ));
                }
                Tok::Lifetime(_) => {
                    let l = p.lifetime()?;
                    g.lifetimes.push(l.clone());
                    l
                }
                Tok::Const => {
                    p.bump();
                    let t = p.ident()?;
                    p.expect(Tok::Colon)?;
                    let ty = p.ty()?;
                    g.consts.push((t.id, ty));
                    g.types.push(t);
                    return Ok(());
                }
                Tok::Effect => {
                    p.bump();
                    let t = p.ident()?;
                    if *p.peek() == Tok::Colon {
                        return Err(Error::syntax(
                            p.span(),
                            "an effect parameter has no bounds: it stands for any effects",
                        ));
                    }
                    g.effects.push(t.id);
                    g.types.push(t);
                    return Ok(());
                }
                _ => {
                    let t = p.ident()?;
                    g.types.push(t.clone());
                    t
                }
            };
            if p.eat(&Tok::Colon) {
                // A lifetime outlives lifetimes; a type parameter can
                // also implement traits.
                let lifetime = param.name.starts_with('\'');
                let mut bounds = Vec::new();
                loop {
                    bounds.push(match p.peek() {
                        Tok::Ident(_) | Tok::Crate | Tok::Super if !lifetime => p.trait_bound()?,
                        _ if lifetime => BoundItem::plain(p.lifetime()?),
                        Tok::Lifetime(_) => BoundItem::plain(p.lifetime()?),
                        _ => return Err(p.unexpected("a lifetime or a trait")),
                    });
                    if !p.eat(&Tok::Plus) {
                        break;
                    }
                }
                g.bounds.push((param, bounds));
            }
            Ok(())
        })?;
        g.span = Some(start.to(end));
        Ok(g)
    }

    /// `Show`, or `Iter<Item = i64, Other: Discard>`, after a `:` in
    /// generics.
    fn trait_bound(&mut self) -> Result<BoundItem, Error> {
        let name = self.path()?;
        if *self.peek() == Tok::LParen {
            self.bump();
            let (params, mut end) = self.comma_list(Tok::RParen, |p| p.ty())?;
            let ret = match self.eat(&Tok::Arrow) {
                true => {
                    let t = self.ty()?;
                    end = t.span;
                    Some(Box::new(t))
                }
                false => None,
            };
            let effects = self.with_effects()?;
            if let Some(e) = &effects {
                end = e.span;
            }
            return Ok(BoundItem {
                span: name.span.to(end),
                name,
                assoc: Vec::new(),
                sig: Some((params, ret)),
                effects,
            });
        }
        if *self.peek() != Tok::Lt {
            return Ok(BoundItem::plain(name));
        }
        self.bump();
        let (assoc, end) = self.comma_list(Tok::Gt, |p| {
            let name = p.ident()?;
            let kind = match p.peek() {
                Tok::Eq => {
                    p.bump();
                    AssocBoundKind::Eq(p.ty()?)
                }
                Tok::Colon => {
                    p.bump();
                    let mut traits = vec![p.path()?];
                    while p.eat(&Tok::Plus) {
                        traits.push(p.path()?);
                    }
                    AssocBoundKind::Bounds(traits)
                }
                _ => {
                    return Err(Error::syntax(
                        name.span,
                        "a trait has no type parameters; this can only say what one of its \
                         associated types is, as in `Iter<Item = i64>`, or what it implements, \
                         as in `Iter<Item: Discard>`",
                    ));
                }
            };
            Ok(AssocBound { name, kind })
        })?;
        Ok(BoundItem {
            span: name.span.to(end),
            name,
            assoc,
            sig: None,
            effects: None,
        })
    }

    /// `'a`
    fn lifetime(&mut self) -> Result<Ident, Error> {
        match self.peek().clone() {
            Tok::Lifetime(name) => Ok(Ident {
                name,
                span: self.bump().span,
                id: self.id(),
            }),
            _ => Err(self.unexpected("a lifetime, like `'a`")),
        }
    }

    /// A function parameter: `x: i64` or `mut x: i64`.
    fn param(&mut self) -> Result<Param, Error> {
        let mutable = (*self.peek() == Tok::Mut).then(|| self.bump().span);
        let name = self.binder()?;
        self.expect(Tok::Colon)?;
        let ty = self.ty()?;
        Ok(Param {
            doc: None,
            allows: Vec::new(),
            mutable,
            vis: Vis::Private,
            name,
            ty,
        })
    }

    fn struct_decl(&mut self, head: Head) -> Result<StructDecl, Error> {
        let start = self.expect(Tok::Struct)?;
        let name = self.ident()?;
        let generics = self.generics()?;
        self.expect(Tok::LBrace)?;
        let (fields, end) = self.comma_list(Tok::RBrace, |p| {
            let Head { doc, allows, .. } = p.head()?.no_derives()?;
            let vis = p.vis();
            if *p.peek() == Tok::Mut {
                return Err(Error::syntax(
                    p.span(),
                    "a field can't be `mut`: whether it can change depends on where its \
                     struct is",
                ));
            }
            Ok(Param {
                doc,
                allows,
                vis,
                ..p.param()?
            })
        })?;
        if let (Some(_), Some(field)) = (head.intrinsic, fields.first()) {
            return Err(Error::syntax(
                field.name.span,
                "an intrinsic struct has no fields: what's in it is the compiler's business",
            ));
        }
        Ok(StructDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            module: self.module,
            vis: Vis::Private,
            name,
            generics,
            derives: head.derives,
            fields,
            intrinsic: head.intrinsic,
        })
    }

    /// `type Name<'a, T> = Type;`, whose parameters have no bounds: what it
    /// stands for is checked where it's used, as if written there.
    fn alias_decl(&mut self, head: Head) -> Result<AliasDecl, Error> {
        let start = self.expect(Tok::Type)?;
        let name = self.ident()?;
        let generics = self.generics()?;
        if let Some((_, b)) = generics.bounds.first() {
            let at = b
                .first()
                .map_or(generics.span.unwrap_or(name.span), |b| b.span);
            return Err(Error::syntax(
                at,
                "a type alias's parameters can't have bounds; what it stands for is checked \
                 where it's used",
            ));
        }
        self.expect(Tok::Eq)?;
        let ty = self.ty()?;
        let end = self.expect(Tok::Semi)?;
        Ok(AliasDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            module: self.module,
            vis: Vis::Private,
            name,
            generics,
            ty,
        })
    }

    fn enum_decl(&mut self, head: Head) -> Result<EnumDecl, Error> {
        let start = self.expect(Tok::Enum)?;
        let name = self.ident()?;
        let generics = self.generics()?;
        self.expect(Tok::LBrace)?;
        let (variants, end) = self.comma_list(Tok::RBrace, |p| {
            let Head { doc, allows, .. } = p.head()?.no_derives()?;
            p.no_pub("an enum's variants are public wherever it is")?;
            let name = p.ident()?;
            let (fields, span) = if p.eat(&Tok::LParen) {
                let (fields, close) = p.comma_list(Tok::RParen, Self::ty)?;
                (fields, name.span.to(close))
            } else {
                (Vec::new(), name.span)
            };
            Ok(Variant {
                doc,
                allows,
                span,
                name,
                fields,
            })
        })?;
        Ok(EnumDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            module: self.module,
            vis: Vis::Private,
            name,
            generics,
            derives: head.derives,
            variants,
        })
    }

    fn ty(&mut self) -> Result<TypeExpr, Error> {
        let mut t = self.ty_head()?;
        // `T::Item`, `T::Item::Other`: projections without the trait.
        while *self.peek() == Tok::ColonColon
            && matches!(t.kind, TypeExprKind::Named(..) | TypeExprKind::Proj(..))
        {
            self.bump();
            let name = self.ident()?;
            let span = t.span.to(name.span);
            t = TypeExpr {
                kind: TypeExprKind::Proj(Box::new(t), None, name),
                span,
                id: self.id(),
            };
        }
        Ok(t)
    }

    fn ty_head(&mut self) -> Result<TypeExpr, Error> {
        let start = self.span();
        match self.peek().clone() {
            // `<T as Iter>::Item`
            Tok::Lt => {
                self.bump();
                let inner = self.ty()?;
                self.expect(Tok::As)?;
                let tr = self.path()?;
                self.expect(Tok::Gt)?;
                self.expect(Tok::ColonColon)?;
                let name = self.ident()?;
                Ok(TypeExpr {
                    kind: TypeExprKind::Proj(Box::new(inner), Some(tr), name.clone()),
                    span: start.to(name.span),
                    id: self.id(),
                })
            }
            Tok::Ident(_) | Tok::Crate | Tok::Super => {
                let mut segs = vec![self.segment()?];
                while *self.peek() == Tok::ColonColon && matches!(self.peek2(), Tok::Ident(_)) {
                    self.bump();
                    segs.push(self.ident()?);
                }
                // There's no `<` operator in a type, so this can only start
                // type arguments.
                let (args, span) = if self.eat(&Tok::Lt) {
                    let (args, end) = self.comma_list(Tok::Gt, Self::type_arg)?;
                    (args, start.to(end))
                } else {
                    (Vec::new(), start.to(segs[segs.len() - 1].span))
                };
                // `T::Item` is a projection, unless `T` turns out to be a
                // module, which only resolving names can tell (see
                // `check/modules.rs`). With type arguments at the end, or
                // starting at `crate`, `super` or `self`, it can only be a
                // path to a type.
                let path =
                    !args.is_empty() || matches!(segs[0].name.as_str(), "crate" | "super" | "self");
                if segs.len() == 1 || path {
                    let refs: Vec<&Ident> = segs.iter().collect();
                    return Ok(TypeExpr {
                        kind: TypeExprKind::Named(self.join_all(&refs).name, args),
                        span,
                        id: self.id(),
                    });
                }
                let mut t = TypeExpr {
                    span: segs[0].span,
                    kind: TypeExprKind::Named(segs[0].name.clone(), Vec::new()),
                    id: self.id(),
                };
                for name in segs.into_iter().skip(1) {
                    let span = t.span.to(name.span);
                    t = TypeExpr {
                        kind: TypeExprKind::Proj(Box::new(t), None, name),
                        span,
                        id: self.id(),
                    };
                }
                Ok(t)
            }
            Tok::LParen => {
                self.bump();
                let end = self.expect(Tok::RParen)?;
                Ok(TypeExpr {
                    kind: TypeExprKind::Unit,
                    span: start.to(end),
                    id: self.id(),
                })
            }
            Tok::Fn => {
                self.bump();
                self.expect(Tok::LParen)?;
                let (params, close) = self.comma_list(Tok::RParen, Self::ty)?;
                let ret = if self.eat(&Tok::Arrow) {
                    self.ty()?
                } else {
                    TypeExpr {
                        kind: TypeExprKind::Unit,
                        span: close,
                        id: self.id(),
                    }
                };
                let effects = self.with_effects()?;
                let span = start.to(effects.as_ref().map_or(ret.span, |e| e.span));
                Ok(TypeExpr {
                    kind: TypeExprKind::Fn(params, Box::new(ret), effects),
                    span,
                    id: self.id(),
                })
            }
            Tok::Amp => {
                self.bump();
                self.ref_type(start)
            }
            // `&&T` is `&(&T)`.
            Tok::AndAnd => {
                self.bump();
                let inner = self.ref_type(Span::new(start.start + 1, start.end))?;
                let span = start.to(inner.span);
                Ok(TypeExpr {
                    kind: TypeExprKind::Ref(None, false, Box::new(inner)),
                    span,
                    id: self.id(),
                })
            }
            // `[T]`, a slice, or `[T; N]`, an array.
            Tok::LBracket => {
                self.bump();
                let inner = self.ty()?;
                let kind = match self.eat(&Tok::Semi) {
                    true => TypeExprKind::Array(Box::new(inner), Box::new(self.ty()?)),
                    false => TypeExprKind::Slice(Box::new(inner)),
                };
                let end = self.expect(Tok::RBracket)?;
                Ok(TypeExpr {
                    kind,
                    span: start.to(end),
                    id: self.id(),
                })
            }
            // `3`, a length.
            Tok::Int(n, None) => Ok(TypeExpr {
                kind: TypeExprKind::Nat(n),
                span: self.bump().span,
                id: self.id(),
            }),
            // `{ v: i64 | v > 0 }`
            Tok::LBrace => {
                self.bump();
                let binder = self.ident()?;
                self.expect(Tok::Colon)?;
                let base = self.ty()?;
                self.expect(Tok::Pipe)?;
                let pred = self.structs(true, Self::expr)?;
                let end = self.expect(Tok::RBrace)?;
                Ok(TypeExpr {
                    kind: TypeExprKind::Refined(binder, Box::new(base), Box::new(pred)),
                    span: start.to(end),
                    id: self.id(),
                })
            }
            Tok::Bang => Ok(TypeExpr {
                kind: TypeExprKind::Never,
                span: self.bump().span,
                id: self.id(),
            }),
            Tok::Impl => {
                self.bump();
                let mut bounds = Vec::new();
                loop {
                    bounds.push(match self.peek() {
                        Tok::Ident(_) | Tok::Crate | Tok::Super => self.trait_bound()?,
                        _ => return Err(self.unexpected("a trait after `impl`")),
                    });
                    if !self.eat(&Tok::Plus) {
                        break;
                    }
                }
                let span = start.to(bounds.last().unwrap().span);
                Ok(TypeExpr {
                    kind: TypeExprKind::Impl(bounds),
                    span,
                    id: self.id(),
                })
            }
            _ => Err(self.unexpected("a type")),
        }
    }

    /// `&'a mut T`, after the `&`, which is at `start`.
    fn ref_type(&mut self, start: Span) -> Result<TypeExpr, Error> {
        let lifetime = match self.peek() {
            Tok::Lifetime(_) => Some(self.lifetime()?),
            _ => None,
        };
        let mutable = self.eat(&Tok::Mut);
        let inner = self.ty()?;
        let span = start.to(inner.span);
        Ok(TypeExpr {
            kind: TypeExprKind::Ref(lifetime, mutable, Box::new(inner)),
            span,
            id: self.id(),
        })
    }

    /// A type argument: a type, or a lifetime.
    fn type_arg(&mut self) -> Result<TypeExpr, Error> {
        match self.peek().clone() {
            Tok::Lifetime(name) => Ok(TypeExpr {
                kind: TypeExprKind::Lifetime(name),
                span: self.bump().span,
                id: self.id(),
            }),
            _ => self.ty(),
        }
    }

    fn block(&mut self) -> Result<Expr, Error> {
        self.structs(true, Self::block_inner)
    }

    fn block_inner(&mut self) -> Result<Expr, Error> {
        let start = self.expect(Tok::LBrace)?;
        let mut stmts = Vec::new();
        let tail = loop {
            if *self.peek() == Tok::RBrace {
                break None;
            }
            if *self.peek() == Tok::Let {
                stmts.push(self.let_stmt()?);
                continue;
            }
            if matches!(self.peek(), Tok::Trait | Tok::Impl) {
                return Err(Error::syntax(
                    self.span(),
                    format!(
                        "{} can only be at the top level, not in a block",
                        match self.peek() {
                            Tok::Trait => "a `trait`",
                            _ => "an `impl`",
                        }
                    ),
                ));
            }
            if matches!(self.peek(), Tok::Mod | Tok::Use | Tok::Pub | Tok::Export) {
                let what = match self.peek() {
                    Tok::Mod => {
                        "a `mod` can't be in a function: a module's items are only in \
                                 modules"
                    }
                    Tok::Use => "a `use` can't be in a function, only in a module",
                    Tok::Pub => "what's in a function can't be `pub`: only it can see it",
                    _ => "what's in a function can't be `export`: only it can see it",
                };
                return Err(Error::syntax(self.span(), what));
            }
            if matches!(
                self.peek(),
                Tok::Fn | Tok::Struct | Tok::Enum | Tok::Type | Tok::Hash
            ) {
                let head = self.head()?.no_intrinsic()?.no_measure(NOT_METHOD)?;
                let stmt = match self.peek() {
                    Tok::Fn => Stmt::Fn(Box::new(self.fn_decl(head.no_derives()?)?)),
                    Tok::Struct => Stmt::Type(Box::new(TypeDecl::Struct(self.struct_decl(head)?))),
                    Tok::Enum => Stmt::Type(Box::new(TypeDecl::Enum(self.enum_decl(head)?))),
                    Tok::Type => {
                        let d = self.alias_decl(head.no_derives()?)?;
                        Stmt::Type(Box::new(TypeDecl::Alias(d)))
                    }
                    _ => {
                        return Err(
                            self.unexpected("`fn`, `struct`, `enum` or `type` after an attribute")
                        );
                    }
                };
                stmts.push(stmt);
                continue;
            }
            let block_like = matches!(
                self.peek(),
                Tok::If
                    | Tok::Match
                    | Tok::While
                    | Tok::Loop
                    | Tok::For
                    | Tok::Lifetime(_)
                    | Tok::LBrace
                    | Tok::Unsafe
            );
            let e = match self.peek() {
                Tok::If => self.if_expr()?,
                Tok::Match => self.match_expr()?,
                Tok::While | Tok::Loop | Tok::For | Tok::Lifetime(_) => self.loop_expr()?,
                Tok::LBrace => self.block()?,
                Tok::Unsafe => self.unsafe_block()?,
                _ => self.expr()?,
            };
            if self.eat(&Tok::Semi) {
                stmts.push(Stmt::Expr(e));
            } else if *self.peek() == Tok::RBrace {
                break Some(Box::new(e));
            } else if block_like {
                // The statement ends here, `;` or not.
                stmts.push(Stmt::BlockLike(e));
            } else {
                return Err(self.unexpected("`;` or `}`"));
            }
        };
        let end = self.expect(Tok::RBrace)?;
        Ok(Expr {
            kind: ExprKind::Block(stmts, tail),
            span: start.to(end),
            id: self.id(),
        })
    }

    /// `unsafe { .. }`: a block that keeps the obligations of the unsafe
    /// operations in it.
    fn unsafe_block(&mut self) -> Result<Expr, Error> {
        let start = self.expect(Tok::Unsafe)?;
        if *self.peek() != Tok::LBrace {
            return Err(self.unexpected("`{` after `unsafe`"));
        }
        let block = self.block()?;
        Ok(Expr {
            span: start.to(block.span),
            kind: ExprKind::Unsafe(Box::new(block)),
            id: self.id(),
        })
    }

    fn let_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.expect(Tok::Let)?;
        let pat = self.pattern()?;
        let ty = if self.eat(&Tok::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        let init = match self.eat(&Tok::Eq) {
            true => Some(self.expr()?),
            false => None,
        };
        let end = self.expect(Tok::Semi)?;
        Ok(Stmt::Let {
            pat,
            ty: ty.map(Box::new),
            init,
            span: start.to(end),
        })
    }

    /// An expression, which may be an assignment: `x = 1`, `p.x += 2`.
    /// Assignment binds less tightly than anything, and to the right.
    fn expr(&mut self) -> Result<Expr, Error> {
        let lhs = self.range()?;
        let op = match self.peek() {
            Tok::Eq => None,
            Tok::OpEq(op) => Some(match op {
                OpEq::Add => BinOp::Add,
                OpEq::Sub => BinOp::Sub,
                OpEq::Mul => BinOp::Mul,
                OpEq::Div => BinOp::Div,
                OpEq::Rem => BinOp::Rem,
                OpEq::And => BinOp::BitAnd,
                OpEq::Or => BinOp::BitOr,
                OpEq::Xor => BinOp::BitXor,
                OpEq::Shl => BinOp::Shl,
                OpEq::Shr => BinOp::Shr,
            }),
            // `>>=`, which is lexed `>` `>=`: see `Parser::binop`.
            Tok::Gt if *self.peek2() == Tok::Ge && self.adjacent() => {
                self.bump();
                Some(BinOp::Shr)
            }
            _ => return Ok(lhs),
        };
        self.bump();
        let rhs = self.expr()?;
        let span = lhs.span.to(rhs.span);
        let mut lhs = lhs;
        written(&mut lhs);
        Ok(Expr {
            kind: ExprKind::Assign(op, Box::new(lhs), Box::new(rhs)),
            span,
            id: self.id(),
        })
    }

    /// `a..b`, or just `a`. A range binds less tightly than any operator,
    /// so `0..n + 1` is `0..(n + 1)`, and doesn't chain.
    fn range(&mut self) -> Result<Expr, Error> {
        let lhs = self.binary(0)?;
        match self.peek() {
            Tok::DotDot => {}
            Tok::DotDotEq => {
                return Err(Error::syntax(
                    self.span(),
                    "`a..=b` isn't supported (yet); write `a..b + 1`",
                ));
            }
            _ => return Ok(lhs),
        }
        let dots = self.bump().span;
        if !starts_expr(self.peek()) || (self.no_struct && *self.peek() == Tok::LBrace) {
            return Err(Error::syntax(
                dots,
                "a range needs an end: `a..b` (ranges with no end aren't supported yet)",
            ));
        }
        let rhs = self.binary(0)?;
        if matches!(self.peek(), Tok::DotDot | Tok::DotDotEq) {
            return Err(Error::syntax(
                self.span(),
                "ranges can't be chained: parenthesize one",
            ));
        }
        let span = lhs.span.to(rhs.span);
        Ok(Expr {
            kind: ExprKind::Range(Box::new(lhs), Box::new(rhs)),
            span,
            id: self.id(),
        })
    }

    /// Precedence climbing: parses an expression whose operators all bind at
    /// least as tightly as `min_prec`.
    fn binary(&mut self, min_prec: u8) -> Result<Expr, Error> {
        Ok(self.binary_op(min_prec)?.0)
    }

    /// `binary`, and the operator of what it parsed if that's a binary
    /// operation, not in parentheses: whether it may be the operand of
    /// another is `BinOp::nests_in`.
    fn binary_op(&mut self, min_prec: u8) -> Result<(Expr, Option<BinOp>), Error> {
        let mut lhs = self.cast()?;
        let mut lhs_op = None;
        while let Some((op, prec)) = self.binop() {
            if prec < min_prec {
                break;
            }
            let at = self.span();
            if op == BinOp::Shr {
                self.bump();
            }
            self.bump();
            let (rhs, rhs_op) = self.binary_op(prec + 1)?;
            for inner in [lhs_op, rhs_op].into_iter().flatten() {
                if !inner.nests_in(op) {
                    return Err(Error::syntax(
                        at,
                        format!(
                            "use parentheses to say which of `{inner}` and `{op}` comes first: \
                             there's no precedence between them"
                        ),
                    ));
                }
            }
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
                id: self.id(),
            };
            lhs_op = Some(op);

            if prec == CMP_PREC && self.binop().is_some_and(|(_, p)| p == CMP_PREC) {
                return Err(Error::syntax(
                    self.span(),
                    "comparison operators cannot be chained; use `&&` (e.g. `a < b && b < c`)",
                ));
            }
        }
        Ok((lhs, lhs_op))
    }

    /// The binary operator next, and how tightly it binds. `>>` is two
    /// `>`s with nothing between them (see `Tok::Shl`), and `>>=` a `>`
    /// and a `>=`, which isn't a binary operator but an assignment.
    fn binop(&self) -> Option<(BinOp, u8)> {
        if *self.peek() == Tok::Gt && self.adjacent() {
            return match self.peek2() {
                Tok::Gt => Some((BinOp::Shr, BITS_PREC)),
                Tok::Ge => None,
                _ => binop(self.peek()),
            };
        }
        binop(self.peek())
    }

    /// Whether the next two tokens have nothing between them.
    fn adjacent(&self) -> bool {
        let next = (self.pos + 1).min(self.toks.len() - 1);
        self.toks[self.pos].span.end == self.toks[next].span.start
    }

    /// `x as u8 as i64`, or just `x`. `as` binds more tightly than any
    /// binary operator, and less than a prefix one: `-x as u8` is `(-x) as
    /// u8`, and `a * b as u8` is `a * (b as u8)`, as in Rust. What follows
    /// it is a type's name, as only an integer type or a type parameter
    /// can be: so `x as u8 < y` is a comparison, where it would otherwise
    /// start `u8`'s type arguments.
    fn cast(&mut self) -> Result<Expr, Error> {
        let mut e = self.unary()?;
        while self.eat(&Tok::As) {
            let name = self.ident()?;
            let ty = TypeExpr {
                kind: TypeExprKind::Named(name.name, Vec::new()),
                span: name.span,
                id: self.id(),
            };
            let span = e.span.to(ty.span);
            e = Expr {
                kind: ExprKind::As(Box::new(e), Box::new(ty)),
                span,
                id: self.id(),
            };
        }
        Ok(e)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        let op = match self.peek() {
            Tok::Minus => UnOp::Neg,
            Tok::Bang => UnOp::Not,
            Tok::Star => UnOp::Deref,
            Tok::Amp => UnOp::Ref,
            // `&&x` is `&(&x)`.
            Tok::AndAnd => {
                let start = self.bump().span;
                let inner = self.borrow(Span::new(start.start + 1, start.end))?;
                let span = start.to(inner.span);
                return Ok(Expr {
                    kind: ExprKind::Unary(UnOp::Ref, Box::new(inner)),
                    span,
                    id: self.id(),
                });
            }
            _ => return self.postfix(),
        };
        let start = self.bump().span;
        if op == UnOp::Ref {
            return self.borrow(start);
        }
        let mut operand = self.unary()?;
        if op == UnOp::RefMut {
            written(&mut operand);
        }
        let span = start.to(operand.span);
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span,
            id: self.id(),
        })
    }

    /// `&x` or `&mut x`, after the `&`, which is at `start`.
    fn borrow(&mut self, start: Span) -> Result<Expr, Error> {
        let op = match self.eat(&Tok::Mut) {
            true => UnOp::RefMut,
            false => UnOp::Ref,
        };
        let mut operand = self.unary()?;
        if op == UnOp::RefMut {
            written(&mut operand);
        }
        let span = start.to(operand.span);
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span,
            id: self.id(),
        })
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut e = self.primary()?;
        loop {
            if *self.peek() == Tok::LBracket {
                let open = self.bump().span;
                let i = self.expr()?;
                let close = self.expect(Tok::RBracket)?;
                let span = e.span.to(close);
                e = Expr {
                    kind: ExprKind::Index(Box::new(e), Box::new(i), open.to(close), false),
                    span,
                    id: self.id(),
                };
            } else if self.eat(&Tok::LParen) {
                let (args, end) = self.args()?;
                let span = e.span.to(end);
                e = Expr {
                    kind: ExprKind::Call(Box::new(e), args),
                    span,
                    id: self.id(),
                };
            } else if self.eat(&Tok::Dot) {
                let name = self.ident()?;
                let (targs, _) = self.turbofish()?;
                e = match self.eat(&Tok::LParen) {
                    true => {
                        let (args, end) = self.args()?;
                        let span = e.span.to(end);
                        let kind = ExprKind::MethodCall(Box::new(e), name, targs, args);
                        Expr {
                            kind,
                            span,
                            id: self.id(),
                        }
                    }
                    false if !targs.is_empty() => {
                        return Err(self.unexpected("`(`: only a method call has type arguments"));
                    }
                    false => {
                        let span = e.span.to(name.span);
                        let kind = ExprKind::Field(Box::new(e), name);
                        Expr {
                            kind,
                            span,
                            id: self.id(),
                        }
                    }
                };
            } else if *self.peek() == Tok::Question {
                let q = self.bump().span;
                let span = e.span.to(q);
                e = Expr {
                    kind: ExprKind::Try(Box::new(e), q),
                    span,
                    id: self.id(),
                };
            } else {
                return Ok(e);
            }
        }
    }

    /// Call arguments, after the `(`.
    fn args(&mut self) -> Result<(Vec<Expr>, Span), Error> {
        self.structs(true, |p| p.comma_list(Tok::RParen, Self::expr))
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Int(n, suffix) => ExprKind::Int(n, suffix),
            Tok::Byte(b) => ExprKind::Int(u64::from(b), Some(crate::types::IntTy::U8)),
            Tok::Char(c) => ExprKind::Char(c),
            Tok::Str(s) => ExprKind::Str(s),
            Tok::ByteStr(b) => ExprKind::ByteStr(b),
            Tok::True => ExprKind::Bool(true),
            Tok::False => ExprKind::Bool(false),
            Tok::Crate | Tok::Super => return self.path_expr(),
            Tok::Ident(name) => match self.peek2() {
                Tok::ColonColon => return self.path_expr(),
                Tok::LBrace if !self.no_struct => {
                    let name = self.ident()?;
                    return self.struct_lit(name, Vec::new());
                }
                _ => ExprKind::Var(name, Vec::new()),
            },
            Tok::LParen => {
                self.bump();
                if *self.peek() == Tok::RParen {
                    let end = self.bump().span;
                    return Ok(Expr {
                        kind: ExprKind::Unit,
                        span: span.to(end),
                        id: self.id(),
                    });
                }
                let inner = self.structs(true, Self::expr)?;
                let end = self.expect(Tok::RParen)?;
                return Ok(Expr {
                    kind: inner.kind,
                    span: span.to(end),
                    id: self.id(),
                });
            }
            Tok::LBracket => return self.array(),
            Tok::LBrace => return self.block(),
            Tok::Unsafe => return self.unsafe_block(),
            Tok::If => return self.if_expr(),
            Tok::Match => return self.match_expr(),
            Tok::While | Tok::Loop | Tok::For | Tok::Lifetime(_) => return self.loop_expr(),
            Tok::Return | Tok::Break | Tok::Continue => return self.jump(),
            Tok::Pipe | Tok::OrOr | Tok::Move => return self.lambda(),
            _ => return Err(self.unexpected("an expression")),
        };
        self.bump();
        Ok(Expr {
            kind,
            span,
            id: self.id(),
        })
    }

    /// `[a, b, c]` or `[v; n]`.
    fn array(&mut self) -> Result<Expr, Error> {
        let start = self.expect(Tok::LBracket)?;
        let kind = self.structs(true, |p| {
            if *p.peek() != Tok::RBracket {
                let first = p.expr()?;
                if p.eat(&Tok::Semi) {
                    let n = p.expr()?;
                    return Ok(ExprKind::Repeat(Box::new(first), Box::new(n)));
                }
                if p.eat(&Tok::Comma) {
                    let mut rest = p.comma_list_open(Tok::RBracket, Self::expr)?;
                    rest.insert(0, first);
                    return Ok(ExprKind::Array(rest));
                }
                return Ok(ExprKind::Array(vec![first]));
            }
            Ok(ExprKind::Array(Vec::new()))
        })?;
        let end = self.expect(Tok::RBracket)?;
        Ok(Expr {
            kind,
            span: start.to(end),
            id: self.id(),
        })
    }

    fn if_expr(&mut self) -> Result<Expr, Error> {
        let start = self.expect(Tok::If)?;
        let cond = self.structs(false, Self::expr)?;
        let then = self.block()?;
        let els = if self.eat(&Tok::Else) {
            Some(Box::new(if *self.peek() == Tok::If {
                self.if_expr()?
            } else {
                self.block()?
            }))
        } else {
            None
        };
        let span = start.to(els.as_ref().map_or(then.span, |e| e.span));
        Ok(Expr {
            kind: ExprKind::If(Box::new(cond), Box::new(then), els),
            span,
            id: self.id(),
        })
    }

    /// `while c { .. }` or `loop { .. }`, maybe after a label, `'a:`.
    fn loop_expr(&mut self) -> Result<Expr, Error> {
        let start = self.span();
        let label = match self.peek() {
            Tok::Lifetime(_) => {
                let name = self.lifetime()?;
                self.expect(Tok::Colon)?;
                let head = Span::new(name.span.start, self.span().start);
                Some(Label { name, head })
            }
            _ => None,
        };
        if *self.peek() == Tok::For {
            self.bump();
            let pat = self.pattern()?;
            let in_span = self.expect(Tok::In)?;
            let iter = self.structs(false, Self::expr)?;
            let body = self.block()?;
            let span = start.to(body.span);
            let f = ForLoop {
                label,
                pat,
                in_span,
                iter,
                body,
            };
            return Ok(Expr {
                kind: ExprKind::For(Box::new(f)),
                span,
                id: self.id(),
            });
        }
        let (cond, body) = match self.peek() {
            Tok::While => {
                self.bump();
                (Some(self.structs(false, Self::expr)?), self.block()?)
            }
            Tok::Loop => {
                self.bump();
                (None, self.block()?)
            }
            _ => return Err(self.unexpected("`loop`, `while` or `for` after a label")),
        };
        let span = start.to(body.span);
        let kind = match cond {
            Some(c) => ExprKind::While(label, Box::new(c), Box::new(body)),
            None => ExprKind::Loop(label, Box::new(body)),
        };
        Ok(Expr {
            kind,
            span,
            id: self.id(),
        })
    }

    /// `return e`, `break 'a e` or `continue 'a`, where the label is there
    /// if a lifetime comes next, and `e` if an expression can start after
    /// that.
    fn jump(&mut self) -> Result<Expr, Error> {
        let keyword = self.bump();
        let label = match (&keyword.tok, self.peek()) {
            (Tok::Break | Tok::Continue, Tok::Lifetime(_)) => Some(self.lifetime()?),
            _ => None,
        };
        let value = match keyword.tok != Tok::Continue && starts_expr(self.peek()) {
            true => Some(Box::new(self.expr()?)),
            false => None,
        };
        let end = match (&value, &label) {
            (Some(v), _) => v.span,
            (None, Some(l)) => l.span,
            (None, None) => keyword.span,
        };
        let kind = match keyword.tok {
            Tok::Return => ExprKind::Return(value),
            Tok::Break => ExprKind::Break(label, value),
            _ => ExprKind::Continue(label),
        };
        Ok(Expr {
            kind,
            span: keyword.span.to(end),
            id: self.id(),
        })
    }

    fn match_expr(&mut self) -> Result<Expr, Error> {
        let start = self.expect(Tok::Match)?;
        let scrutinee = self.structs(false, Self::expr)?;
        self.expect(Tok::LBrace)?;
        let mut arms = Vec::new();
        while *self.peek() != Tok::RBrace {
            let pat = self.pattern()?;
            self.expect(Tok::FatArrow)?;
            let body = self.structs(true, Self::expr)?;
            let block_like = matches!(
                body.kind,
                ExprKind::Block(..)
                    | ExprKind::If(..)
                    | ExprKind::Match(..)
                    | ExprKind::While(..)
                    | ExprKind::Loop(..)
            );
            arms.push(Arm { pat, body });
            if !self.eat(&Tok::Comma) && !block_like && *self.peek() != Tok::RBrace {
                return Err(self.unexpected("`,` or `}`"));
            }
        }
        let end = self.expect(Tok::RBrace)?;
        Ok(Expr {
            kind: ExprKind::Match(Box::new(scrutinee), arms),
            span: start.to(end),
            id: self.id(),
        })
    }

    /// `::<i64, bool>` after a name, if it's there: explicit type arguments.
    /// Like Rust, the `::` keeps the `<` from reading as less-than.
    fn turbofish(&mut self) -> Result<(Vec<TypeExpr>, Option<Span>), Error> {
        if *self.peek() == Tok::ColonColon && *self.peek2() == Tok::Lt {
            self.bump();
            self.bump();
            let (args, end) = self.comma_list(Tok::Gt, Self::type_arg)?;
            Ok((args, Some(end)))
        } else {
            Ok((Vec::new(), None))
        }
    }

    /// A name followed by `::`: `Shape::Rect(2, 3)`, `Shape::Empty`,
    /// `Option::<i64>::None`, `id::<i64>`, `Pair::<i64, bool> { .. }`, or a
    /// path that starts with a module: `geometry::origin()`,
    /// `crate::a::Point { .. }`. Which of a path's names are modules isn't
    /// known yet: `a::f(1)` is read as a variant `f` of an `a`, until
    /// resolving names says `a` is a module (see `check/modules.rs`).
    fn path_expr(&mut self) -> Result<Expr, Error> {
        let start = self.span();
        let segs = self.segments()?;
        let (last, targs, end) = segs[segs.len() - 1].clone();
        let all: Vec<&Ident> = segs.iter().map(|s| &s.0).collect();
        if *self.peek() == Tok::LBrace && !self.no_struct {
            no_targs(&segs[..segs.len() - 1])?;
            let name = self.join_all(&all);
            return self.struct_lit(name, targs);
        }
        if segs.len() == 1 || end.is_some() {
            no_targs(&segs[..segs.len() - 1])?;
            return Ok(Expr {
                span: start.to(end.unwrap_or(last.span)),
                kind: ExprKind::Var(self.join_all(&all).name, targs),
                id: self.id(),
            });
        }
        let ty_segs = &segs[..segs.len() - 1];
        no_targs(&ty_segs[..ty_segs.len() - 1])?;
        let ty = self.join_all(&all[..all.len() - 1]);
        let ty_targs = ty_segs[ty_segs.len() - 1].1.clone();
        let parens = self.eat(&Tok::LParen);
        let (args, end) = match parens {
            true => self.args()?,
            false => (Vec::new(), last.span),
        };
        Ok(Expr {
            kind: ExprKind::Variant(ty, ty_targs, last, args, parens),
            span: start.to(end),
            id: self.id(),
        })
    }

    /// `Point { x: 1, y }`, from the `{`, after the name and any type
    /// arguments.
    fn struct_lit(&mut self, name: Ident, targs: Vec<TypeExpr>) -> Result<Expr, Error> {
        self.expect(Tok::LBrace)?;
        let (fields, end) = self.structs(true, |p| {
            p.comma_list(Tok::RBrace, |p| {
                let name = p.ident()?;
                let value = if p.eat(&Tok::Colon) {
                    p.expr()?
                } else {
                    Expr {
                        kind: ExprKind::Var(name.name.clone(), Vec::new()),
                        span: name.span,
                        id: p.id(),
                    }
                };
                Ok(FieldInit { name, value })
            })
        })?;
        let span = name.span.to(end);
        Ok(Expr {
            kind: ExprKind::StructLit(name, targs, fields),
            span,
            id: self.id(),
        })
    }

    /// `|x, y: i64| x + y`, `|x| -> i64 { x }` or `|| 1`. As in Rust, the body
    /// runs as far right as an expression can, and a return type needs a
    /// block after it.
    fn lambda(&mut self) -> Result<Expr, Error> {
        let start = self.span();
        let moves = (*self.peek() == Tok::Move).then(|| self.bump().span);
        let params = if self.eat(&Tok::OrOr) {
            Vec::new()
        } else {
            self.expect(Tok::Pipe)?;
            self.comma_list(Tok::Pipe, |p| {
                let mutable = (*p.peek() == Tok::Mut).then(|| p.bump().span);
                let name = p.binder()?;
                let ty = if p.eat(&Tok::Colon) {
                    Some(p.ty()?)
                } else {
                    None
                };
                Ok(LambdaParam { mutable, name, ty })
            })?
            .0
        };
        let (ret, body) = if self.eat(&Tok::Arrow) {
            let ret = self.ty()?;
            if *self.peek() != Tok::LBrace {
                return Err(self.unexpected("`{` (a closure with a return type needs a block)"));
            }
            (Some(ret), self.block()?)
        } else {
            (None, self.expr()?)
        };
        let span = start.to(body.span);
        let mut lambda = Lambda::new(self.id(), params, ret, body);
        lambda.moves = moves;
        Ok(Expr {
            kind: ExprKind::Lambda(Arc::new(lambda)),
            span,
            id: self.id(),
        })
    }

    fn pattern(&mut self) -> Result<Pattern, Error> {
        let first = self.pat_atom()?;
        if *self.peek() == Tok::OrOr {
            let span = self.span();
            return Err(
                Error::syntax(span, "use `|` between pattern alternatives, not `||`").with_fix(
                    Fix::new("replace `||` with `|`", vec![Edit::replace(span, "|")]),
                ),
            );
        }
        if *self.peek() != Tok::Pipe {
            return Ok(first);
        }
        let mut alts = vec![first];
        while self.eat(&Tok::Pipe) {
            alts.push(self.pat_atom()?);
        }
        let span = alts[0].span.to(alts[alts.len() - 1].span);
        Ok(Pattern {
            kind: PatternKind::Or(alts),
            span,
            id: self.id(),
        })
    }

    fn pat_atom(&mut self) -> Result<Pattern, Error> {
        let start = self.span();
        let kind = match self.peek().clone() {
            Tok::Str(_) | Tok::ByteStr(_) => {
                return Err(Error::syntax(
                    start,
                    "a string literal can't be a pattern yet; compare with `==`",
                ));
            }
            Tok::Int(..) | Tok::Minus | Tok::Byte(_) | Tok::Char(_) => {
                let (lo, a, mut end) = self.pat_int()?;
                let kind = match self.peek() {
                    Tok::DotDot => {
                        end = self.bump().span;
                        if starts_pat_int(self.peek()) {
                            let (hi, b, e) = self.pat_int()?;
                            end = e;
                            // Can't overflow: a literal is at most `u64::MAX`.
                            PatternKind::Range(Some(lo), Some(hi - 1), false, a.with(b, end)?)
                        } else {
                            PatternKind::Range(Some(lo), None, false, a)
                        }
                    }
                    Tok::DotDotEq => {
                        self.bump();
                        let (hi, b, e) = self.pat_int()?;
                        end = e;
                        PatternKind::Range(Some(lo), Some(hi), true, a.with(b, end)?)
                    }
                    _ => PatternKind::Int(lo, a),
                };
                return Ok(Pattern {
                    kind,
                    span: start.to(end),
                    id: self.id(),
                });
            }
            Tok::DotDot | Tok::DotDotEq => {
                let inclusive = self.bump().tok == Tok::DotDotEq;
                let (hi, lit, end) = self.pat_int()?;
                let hi = if inclusive { hi } else { hi - 1 };
                return Ok(Pattern {
                    kind: PatternKind::Range(None, Some(hi), inclusive, lit),
                    span: start.to(end),
                    id: self.id(),
                });
            }
            Tok::True => PatternKind::Bool(true),
            Tok::False => PatternKind::Bool(false),
            Tok::Mut => {
                self.bump();
                let name = self.binder()?;
                if name.name == "_" {
                    return Err(Error::syntax(
                        name.span,
                        "`_` binds nothing, so it can't be `mut`",
                    ));
                }
                return Ok(Pattern {
                    kind: PatternKind::Bind(name.name, true),
                    span: start.to(name.span),
                    id: self.id(),
                });
            }
            Tok::LParen => {
                self.bump();
                let end = self.expect(Tok::RParen)?;
                return Ok(Pattern {
                    kind: PatternKind::Unit,
                    span: start.to(end),
                    id: self.id(),
                });
            }
            Tok::LBracket => {
                self.bump();
                let (pats, end) = self.comma_list(Tok::RBracket, Self::pattern)?;
                return Ok(Pattern {
                    kind: PatternKind::Array(pats),
                    span: start.to(end),
                    id: self.id(),
                });
            }
            Tok::Crate | Tok::Super => return self.path_pattern(),
            Tok::Ident(name) => match self.peek2() {
                Tok::ColonColon => return self.path_pattern(),
                Tok::LBrace => {
                    let ty = self.ident()?;
                    return self.struct_pattern(ty, Vec::new());
                }
                _ if name == "_" => PatternKind::Wild,
                _ => {
                    let name = self.binder()?;
                    return Ok(Pattern {
                        kind: PatternKind::Bind(name.name, false),
                        span: name.span,
                        id: self.id(),
                    });
                }
            },
            _ => return Err(self.unexpected("a pattern")),
        };
        self.bump();
        Ok(Pattern {
            kind,
            span: start,
            id: self.id(),
        })
    }

    /// `Shape::Dot(p)`, `a::Shape::Empty` or `a::P::<i64> { .. }`: a
    /// pattern that starts with a path.
    fn path_pattern(&mut self) -> Result<Pattern, Error> {
        let start = self.span();
        let segs = self.segments()?;
        let all: Vec<&Ident> = segs.iter().map(|s| &s.0).collect();
        let (last, last_targs, end) = segs[segs.len() - 1].clone();
        if *self.peek() == Tok::LBrace {
            no_targs(&segs[..segs.len() - 1])?;
            let name = self.join_all(&all);
            return self.struct_pattern(name, last_targs);
        }
        if segs.len() == 1 {
            return Err(self.unexpected("`::` or `{`"));
        }
        if let Some(end) = end {
            return Err(Error::syntax(
                end,
                "a variant has no type arguments of its own",
            ));
        }
        let ty_segs = &segs[..segs.len() - 1];
        no_targs(&ty_segs[..ty_segs.len() - 1])?;
        let ty = self.join_all(&all[..all.len() - 1]);
        let targs = ty_segs[ty_segs.len() - 1].1.clone();
        let (args, end) = if self.eat(&Tok::LParen) {
            self.comma_list(Tok::RParen, Self::pattern)?
        } else {
            (Vec::new(), last.span)
        };
        Ok(Pattern {
            kind: PatternKind::Variant(ty, targs, last, args),
            span: start.to(end),
            id: self.id(),
        })
    }

    /// `5` or `-5` in a pattern, or `b'a'` or `'a'`, by its value.
    fn pat_int(&mut self) -> Result<(i128, Lit, Span), Error> {
        let start = self.span();
        match *self.peek() {
            Tok::Byte(b) => return Ok((i128::from(b), Lit::Byte, self.bump().span)),
            Tok::Char(c) => return Ok((i128::from(u32::from(c)), Lit::Char, self.bump().span)),
            _ => {}
        }
        let neg = self.eat(&Tok::Minus);
        let Tok::Int(n, suffix) = *self.peek() else {
            return Err(self.unexpected("an integer"));
        };
        let end = self.bump().span;
        if suffix.is_some() {
            return Err(Error::syntax(
                end,
                "an integer in a pattern can't have a suffix: its type is that of what it matches",
            ));
        }
        let n = i128::from(n);
        Ok((if neg { -n } else { n }, Lit::Int, start.to(end)))
    }

    /// `Point { x, y: 0, .. }`
    fn struct_pattern(&mut self, ty: Ident, targs: Vec<TypeExpr>) -> Result<Pattern, Error> {
        self.expect(Tok::LBrace)?;
        let mut fields = Vec::new();
        let mut rest = false;
        loop {
            if *self.peek() == Tok::RBrace {
                break;
            }
            if self.eat(&Tok::DotDot) {
                rest = true;
                break;
            }
            if *self.peek() == Tok::Mut {
                let message = match self.peek2() {
                    Tok::Ident(x) => format!("write `{x}: mut {x}` to bind field `{x}` mutably"),
                    _ => "expected a field name, found `mut`".to_string(),
                };
                return Err(Error::syntax(self.span(), message));
            }
            let name = self.ident()?;
            let pat = if self.eat(&Tok::Colon) {
                self.pattern()?
            } else {
                Pattern {
                    kind: PatternKind::Bind(name.name.clone(), false),
                    span: name.span,
                    id: self.id(),
                }
            };
            fields.push(FieldPat { name, pat });
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        let span = ty.span.to(self.expect(Tok::RBrace)?);
        Ok(Pattern {
            kind: PatternKind::Struct(ty, targs, fields, rest),
            span,
            id: self.id(),
        })
    }
}

/// Can a literal in a pattern start with `tok`?
fn starts_pat_int(tok: &Tok) -> bool {
    matches!(tok, Tok::Int(..) | Tok::Minus | Tok::Byte(_) | Tok::Char(_))
}

impl Lit {
    /// The kind of a range from a literal of this kind to one of kind
    /// `other`, which ends at `end`: characters go only with characters,
    /// and a byte makes it a range of bytes.
    fn with(self, other: Lit, end: Span) -> Result<Lit, Error> {
        match (self, other) {
            (a, b) if a == b => Ok(a),
            (Lit::Char, _) | (_, Lit::Char) => Err(Error::syntax(
                end,
                "a range of characters goes from a character to a character",
            )),
            _ => Ok(Lit::Byte),
        }
    }
}

/// Can an expression start with `tok`?
fn starts_expr(tok: &Tok) -> bool {
    matches!(
        tok,
        Tok::Int(..)
            | Tok::Byte(_)
            | Tok::Char(_)
            | Tok::Str(_)
            | Tok::ByteStr(_)
            | Tok::Ident(_)
            | Tok::True
            | Tok::False
            | Tok::LParen
            | Tok::LBrace
            | Tok::If
            | Tok::Match
            | Tok::While
            | Tok::Loop
            | Tok::Unsafe
            | Tok::For
            | Tok::Lifetime(_)
            | Tok::Return
            | Tok::Break
            | Tok::Continue
            | Tok::Move
            | Tok::Pipe
            | Tok::OrOr
            | Tok::Minus
            | Tok::Bang
            | Tok::Star
            | Tok::Amp
            | Tok::AndAnd
    )
}

const CMP_PREC: u8 = 3;

/// How tightly `&`, `|`, `^`, `<<` and `>>` bind: more than comparisons,
/// and as `+`, though which of them comes first, or of them and
/// arithmetic, needs parentheses (`BinOp::nests_in`).
const BITS_PREC: u8 = 4;

fn binop(tok: &Tok) -> Option<(BinOp, u8)> {
    Some(match tok {
        Tok::OrOr => (BinOp::Or, 1),
        Tok::AndAnd => (BinOp::And, 2),
        Tok::EqEq => (BinOp::Eq, CMP_PREC),
        Tok::NotEq => (BinOp::Ne, CMP_PREC),
        Tok::Lt => (BinOp::Lt, CMP_PREC),
        Tok::Le => (BinOp::Le, CMP_PREC),
        Tok::Gt => (BinOp::Gt, CMP_PREC),
        Tok::Ge => (BinOp::Ge, CMP_PREC),
        Tok::Plus => (BinOp::Add, 4),
        Tok::Minus => (BinOp::Sub, 4),
        Tok::Star => (BinOp::Mul, 5),
        Tok::Slash => (BinOp::Div, 5),
        Tok::Percent => (BinOp::Rem, 5),
        Tok::Amp => (BinOp::BitAnd, BITS_PREC),
        Tok::Pipe => (BinOp::BitOr, BITS_PREC),
        Tok::Caret => (BinOp::BitXor, BITS_PREC),
        Tok::Shl => (BinOp::Shl, BITS_PREC),
        _ => return None,
    })
}

/// What can come before a `fn`, `struct`, `enum`, field or variant.
/// A segment of a path in an expression or pattern, with its type
/// arguments and where they end, if it has any.
type Segment = (Ident, Vec<TypeExpr>, Option<Span>);

/// Why a `#[measure]` can't be where it is.
const NOT_METHOD: &str = "only a method of `&self` can be a measure";
const TRAIT_MEASURE: &str =
    "a trait's method can't be a measure: each impl's body would have to be the same";

struct Head {
    doc: Option<DocComment>,
    allows: Vec<Allow>,
    derives: Vec<Derive>,
    /// Where its `#[intrinsic]` is, if it has one: only a function's can.
    intrinsic: Option<Span>,
    /// Where its `#[measure]` is, if it has one: only a function's can.
    measure: Option<Span>,
}

impl Head {
    /// Checks this head, for an associated type, has no `#[allow(..)]`.
    fn no_allows(&self) -> Result<(), Error> {
        self.no_allows_on("an associated type")
    }

    /// Checks this head, for `what`, has no `#[allow(..)]`.
    fn no_allows_on(&self, what: &str) -> Result<(), Error> {
        match self.allows.first() {
            Some(a) => Err(Error::syntax(
                a.span,
                format!("`#[allow(..)]` can't be on {what}"),
            )),
            None => Ok(()),
        }
    }

    /// This head, for something that isn't a function, so can't have an
    /// `#[intrinsic]`.
    fn no_intrinsic(self) -> Result<Head, Error> {
        match self.intrinsic {
            Some(at) => Err(Error::syntax(
                at,
                "`#[intrinsic]` can only be on a function, a struct, a trait, or a method of an \
                 impl that isn't a trait's",
            )),
            None => Ok(self),
        }
    }

    /// This head, for something that isn't a function, so can't have a
    /// `#[measure]`.
    fn no_measure(self, why: &str) -> Result<Head, Error> {
        match self.measure {
            Some(at) => Err(Error::syntax(at, why)),
            None => Ok(self),
        }
    }

    /// This head, for something that isn't a struct or enum, so can't
    /// have a `#[derive(..)]`.
    fn no_derives(self) -> Result<Head, Error> {
        match self.derives.first() {
            Some(d) => Err(Error::syntax(
                d.span,
                "`#[derive(..)]` can only be on a struct or enum",
            )),
            None => Ok(self),
        }
    }
}

/// Marks each `x[i]` place `e` is, or is inside of, through fields and
/// `*`s, as written to: it's `index_mut`'s, not `index`'s.
fn written(e: &mut Expr) {
    match &mut e.kind {
        ExprKind::Index(x, _, _, w) => {
            *w = true;
            written(x);
        }
        ExprKind::Field(x, _) | ExprKind::Unary(UnOp::Deref, x) => written(x),
        _ => {}
    }
}

/// One doc comment from a run of `///` lines.
fn doc_comment(lines: &[Token]) -> DocComment {
    let text = |t: &Token| match &t.tok {
        Tok::Doc(text) | Tok::InnerDoc(text) => text.clone(),
        _ => unreachable!("only doc comments are set aside"),
    };
    DocComment {
        inner: matches!(lines[0].tok, Tok::InnerDoc(_)),
        lines: lines
            .iter()
            .map(|t| DocLine {
                text: text(t),
                span: t.span,
            })
            .collect(),
    }
}

/// Checks that none of `segs`, the start of a path, has type arguments:
/// they go after the type's name, or a function's.
fn no_targs(segs: &[Segment]) -> Result<(), Error> {
    match segs.iter().find_map(|(_, _, end)| *end) {
        Some(end) => Err(Error::syntax(
            end,
            "type arguments can only go after the type or function they're for",
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod generated_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn editor_keeps_items_around_errors() {
        for bad in [
            "fn broken( -> i64 {}",
            "fn broken( fn(i64)) {}",
            "fn broken() { let x = ; fn local() {} struct Local {} }",
            "struct Broken { x: }",
            "enum Broken { A(, bool) }",
            "trait Broken { fn missing( ; }",
            "impl Before { fn missing( ; }",
            "use missing::;",
            "#[unknown] fn broken() {}",
            "#[unknown] pub fn broken() {}",
            "#[unknown] #[allow(dead_code)] pub fn broken() {}",
            "#[derive(Copy)] fn broken() {}",
            "pub impl Before {}",
            "unexpected tokens;",
            "}",
        ] {
            let src = format!(
                "struct Before {{}}\n{bad}\n/// Kept.\n#[allow(dead_code)] pub fn after() {{}}"
            );
            let tokens = lex(&src).unwrap();
            let strict = parse(tokens.clone()).unwrap_err();
            let parsed = parse_for_editor(tokens);
            assert!(!parsed.errors.is_empty(), "{bad}");
            assert_eq!(parsed.errors[0].span, strict.span, "{bad}");
            assert_eq!(parsed.errors[0].message, strict.message, "{bad}");
            assert_eq!(parsed.program.types.len(), 1, "{bad}");
            assert_eq!(parsed.program.types[0].name().name, "Before", "{bad}");
            assert_eq!(parsed.program.fns.len(), 1, "{bad}");
            let after = &parsed.program.fns[0];
            assert_eq!(after.name.name, "after", "{bad}");
            assert_eq!(after.module, ROOT, "{bad}");
            assert_eq!(after.doc.as_ref().unwrap().markdown(), "Kept.", "{bad}");
            assert!(parsed.program.traits.is_empty(), "{bad}");
            assert!(parsed.program.impls.is_empty(), "{bad}");
            assert!(parsed.program.uses.is_empty(), "{bad}");
        }
    }

    #[test]
    fn editor_recovers_inside_modules_and_reports_each_item() {
        let src = "mod m { fn before() {} fn broken(x:) {} struct S {} use x::; fn after() {} } fn main() {}";
        let parsed = parse_for_editor(lex(src).unwrap());
        assert_eq!(parsed.errors.len(), 2);
        assert_eq!(parsed.program.mods.len(), 2);
        assert_eq!(
            parsed.program.mods[1].span.end,
            src.find(" fn main").unwrap()
        );
        let fns: Vec<_> = parsed
            .program
            .fns
            .iter()
            .map(|f| (f.name.name.as_str(), f.module))
            .collect();
        assert_eq!(fns, [("before", 1), ("after", 1), ("main", ROOT)]);
        assert_eq!(parsed.program.types[0].module(), 1);
    }

    #[test]
    fn editor_discards_unclosed_modules_without_orphaning_children() {
        let src = "fn before() {} mod unfinished { fn child() {} mod nested { struct S {} }";
        let parsed = parse_for_editor(lex(src).unwrap());
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].span, Span::new(src.len(), src.len()));
        assert_eq!(parsed.program.mods.len(), 1);
        assert!(parsed.program.types.is_empty());
        assert_eq!(parsed.program.fns.len(), 1);
        assert_eq!(parsed.program.fns[0].name.name, "before");
    }

    #[test]
    fn editor_accepts_valid_input_and_terminates_at_truncated_input() {
        let src = "//! Docs.\nmod m { pub struct S {} } fn main() { fn local() {} }";
        let tokens = lex(src).unwrap();
        let strict = parse(tokens.clone()).unwrap();
        let parsed = parse_for_editor(tokens);
        assert!(parsed.errors.is_empty());
        assert_eq!(
            crate::fmt::shape(&strict),
            crate::fmt::shape(&parsed.program)
        );

        for src in ["", "fn", "#[", "pub", "struct S {", "mod m { fn", ";;}"] {
            let parsed = parse_for_editor(lex(src).unwrap());
            assert_eq!(parsed.errors.is_empty(), src.is_empty(), "{src}");
        }
    }
}
