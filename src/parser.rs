//! Recursive-descent parser: tokens -> AST.
//!
//! Grammar (lowest to highest precedence for operators):
//!
//! ```text
//! program  = (head (fn_decl | struct | enum | trait | impl))*
//! head     = attr*
//! attr     = "#" "[" ("allow" | "derive") "(" (IDENT ("," IDENT)* ","?)? ")" "]"
//! fn_decl  = "fn" IDENT generics? "(" (param ("," param)* ","?)? ")" ("->" type)? block
//! generics = "<" (gparam ("," gparam)* ","?)? ">"     -- lifetimes before types
//! gparam   = LIFETIME (":" LIFETIME ("+" LIFETIME)*)? | IDENT (":" bound ("+" bound)*)?
//! bound    = LIFETIME | IDENT                         -- a lifetime, or a trait
//! param    = "mut"? IDENT ":" type
//! trait    = "trait" IDENT (":" IDENT ("+" IDENT)*)? "{" (head method)* "}"
//! impl     = "impl" generics? (IDENT "for")? type "{" (head method)* "}"
//! method   = "fn" IDENT generics? "(" ((receiver | param) ("," param)* ","?)? ")"
//!            ("->" type)? (block | ";")               -- `;` only in a trait
//! receiver = "mut"? "self" | "&" LIFETIME? "mut"? "self"
//! struct   = "struct" IDENT generics? "{" (head param ("," head param)* ","?)? "}"
//! enum     = "enum" IDENT generics? "{" (head variant ("," head variant)* ","?)? "}"
//! variant  = IDENT ("(" type ("," type)* ","? ")")?
//! type     = IDENT ("<" targ ("," targ)* ","? ">")? | "(" ")"
//!          | "fn" "(" (type ("," type)*)? ")" ("->" type)?
//!          | "&" LIFETIME? "mut"? type | "!"
//! targ     = LIFETIME | type
//! block    = "{" (let | head (fn_decl | struct | enum) | expr ";" | block_like ";"?)* expr? "}"
//! block_like = if | match | label? (while | loop) | block
//! let      = "let" pattern (":" type)? "=" expr ";"
//! expr     = or (("=" | "+=" | "-=" | "*=" | "/=" | "%=") expr)?  -- assignment
//! or       = and ("||" and)*
//! and      = cmp ("&&" cmp)*
//! cmp      = add (("==" | "!=" | "<" | "<=" | ">" | ">=") add)?   -- no chaining
//! add      = mul (("+" | "-") mul)*
//! mul      = unary (("*" | "/" | "%") unary)*
//! unary    = ("-" | "!" | "*" | "&" "mut"?) unary | call
//! postfix  = primary ("(" args ")" | "." IDENT | "." IDENT targs? "(" args ")")*
//! primary  = INT | "true" | "false" | IDENT targs? | "(" ")" | "(" expr ")" | block
//!          | if | match | label? (while | loop) | IDENT targs? "{" field_init,* "}"
//!          | IDENT targs? "::" IDENT ("(" args ")")? | lambda
//!          | "return" expr? | "break" LIFETIME? expr? | "continue" LIFETIME?
//! label    = LIFETIME ":"
//! targs    = "::" "<" type ("," type)* ","? ">"            -- the "turbofish"
//! lambda   = ("||" | "|" (lparam ("," lparam)* ","?)? "|") ("->" type block | expr)
//! lparam   = "mut"? IDENT (":" type)?
//! if       = "if" expr block ("else" (block | if))?
//! while    = "while" expr block
//! loop     = "loop" block
//! match    = "match" expr "{" (pattern "=>" expr ","?)* "}"  -- `,` optional after a block/if/match
//! field_init = IDENT (":" expr)?                             -- `x` is short for `x: x`
//!
//! pattern  = pat_atom ("|" pat_atom)*
//! pat_atom = "_" | "mut"? IDENT | int | range | "true" | "false" | "(" ")"
//!          | IDENT targs? "::" IDENT ("(" pattern,* ")")?
//!          | IDENT targs? "{" (IDENT (":" pattern)?),* ".."? "}"
//! range    = int ".." int? | int "..=" int | ".." int | "..=" int
//! int      = "-"? INT
//! ```
//!
//! Like Rust, an `if`, `match` or block at the *start* of a statement is the
//! whole statement, so `if c { a } else { b } - x` is two statements, not a
//! subtraction. Without a `;` such a statement must have type `()` (checked in
//! `check.rs`), which turns that mistake into a type error. Parenthesize to
//! use one in a larger expression: `(if c { a } else { b }) - x`.
//!
//! `return` and `break` take an expression after them if one can start
//! there (after `break`'s label, if it has one), and it runs as far right as an expression can, as a closure's
//! body does: `return a + b` returns `a + b`.
//!
//! A `fn`, `struct` or `enum` in a block is local to it (see `ast::Stmt`).
//! A `trait` or `impl` can only be at the top level.
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

pub fn parse(tokens: Vec<Token>) -> Result<Program, Error> {
    let (docs, toks): (Vec<Token>, Vec<Token>) = tokens
        .into_iter()
        .partition(|t| matches!(t.tok, Tok::Doc(_)));
    let mut p = Parser {
        toks,
        attached: vec![false; docs.len()],
        docs,
        pos: 0,
        no_struct: false,
    };
    let mut types = Vec::new();
    let mut traits = Vec::new();
    let mut impls = Vec::new();
    let mut fns = Vec::new();
    loop {
        if *p.peek() == Tok::Eof {
            break;
        }
        let head = p.head()?;
        match p.peek() {
            Tok::Struct => types.push(TypeDecl::Struct(p.struct_decl(head)?)),
            Tok::Enum => types.push(TypeDecl::Enum(p.enum_decl(head)?)),
            Tok::Trait => traits.push(p.trait_decl(head.no_derives()?)?),
            Tok::Impl => impls.push(p.impl_decl(head.no_derives()?)?),
            _ => fns.push(p.fn_decl(head.no_derives()?)?),
        }
    }
    let stray_docs = p.stray_docs();
    Ok(Program {
        types,
        traits,
        impls,
        fns,
        stray_docs,
    })
}

struct Parser {
    /// Every token but doc comments.
    toks: Vec<Token>,
    /// The doc comments, and whether each is attached to something yet.
    docs: Vec<Token>,
    attached: Vec<bool>,
    pos: usize,
    /// Set while parsing the head of an `if` or `match`, where `x {` must not
    /// be read as a struct literal.
    no_struct: bool,
}

impl Parser {
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
        let from = match self.pos {
            0 => 0,
            i => self.toks[i - 1].span.end,
        };
        let to = self.span().start;
        let first = self.docs.partition_point(|d| d.span.start < from);
        let last = self.docs.partition_point(|d| d.span.start < to);
        if first == last {
            return None;
        }
        self.attached[first..last].fill(true);
        Some(doc_comment(&self.docs[first..last]))
    }

    /// The doc comment and attributes of what starts at the next token, or
    /// after the attributes there.
    fn head(&mut self) -> Result<Head, Error> {
        let mut doc = self.doc();
        let mut allows = Vec::new();
        let mut derives = Vec::new();
        while *self.peek() == Tok::Hash {
            let start = self.bump().span;
            self.expect(Tok::LBracket)?;
            let derive = match self.peek() {
                Tok::Ident(name) if name == "allow" || name == "derive" => name == "derive",
                Tok::Ident(name) => {
                    let message = format!(
                        "unknown attribute `{name}`; the only ones are `allow` and `derive`"
                    );
                    return Err(Error::syntax(self.span(), message));
                }
                _ => return Err(self.unexpected("`allow` or `derive`")),
            };
            self.bump();
            self.expect(Tok::LParen)?;
            let (names, _) = self.comma_list(Tok::RParen, Self::ident)?;
            let end = self.expect(Tok::RBracket)?;
            let span = start.to(end);
            if derive {
                if let Some(n) = names
                    .iter()
                    .find(|n| !matches!(n.name.as_str(), "Copy" | "Discard" | "Eq"))
                {
                    return Err(Error::syntax(
                        n.span,
                        format!(
                            "can't derive `{}`; only `Copy`, `Discard` and `Eq` can be derived",
                            n.name
                        ),
                    ));
                }
                derives.push(Derive { span, names });
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
        })
    }

    /// The doc comments nothing took, each run of lines with no other token
    /// between them as one.
    fn stray_docs(&self) -> Vec<DocComment> {
        let mut runs: Vec<Vec<Token>> = Vec::new();
        let mut prev_next_tok = None;
        for (d, _) in self.docs.iter().zip(&self.attached).filter(|(_, a)| !**a) {
            // The first token after it: the same for lines with nothing
            // between them.
            let next_tok = self.toks.partition_point(|t| t.span.start < d.span.start);
            match runs.last_mut() {
                Some(run) if prev_next_tok == Some(next_tok) => run.push(d.clone()),
                _ => runs.push(vec![d.clone()]),
            }
            prev_next_tok = Some(next_tok);
        }
        runs.iter().map(|run| doc_comment(run)).collect()
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

    fn fn_decl(&mut self, head: Head) -> Result<FnDecl, Error> {
        if *self.peek() != Tok::Fn {
            return Err(self.unexpected("`fn`, `struct`, `enum`, `trait` or `impl`"));
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
            }
        };
        let (body, provided) = match self.peek() {
            Tok::Semi if in_trait => {
                let span = self.bump().span;
                let body = Expr {
                    kind: ExprKind::Block(Vec::new(), None),
                    span,
                };
                (body, false)
            }
            Tok::Semi => {
                return Err(Error::syntax(
                    self.span(),
                    "a function needs a body; only a method in a `trait` can have none",
                ));
            }
            _ => (self.block()?, true),
        };
        let f = FnDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(body.span),
            name,
            generics,
            params,
            parens: open.to(close),
            ret,
            body,
        };
        Ok((f, provided))
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
        };
        let (mutable, ty) = match by_ref {
            false => (mut_span, self_ty),
            true => {
                let kind = TypeExprKind::Ref(lifetime, mut_span.is_some(), Box::new(self_ty));
                let span = start.to(name.span);
                (None, TypeExpr { kind, span })
            }
        };
        Ok(Param {
            doc: None,
            allows: Vec::new(),
            mutable,
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
            supers.push(self.ident()?);
            while self.eat(&Tok::Plus) {
                supers.push(self.ident()?);
            }
        }
        self.expect(Tok::LBrace)?;
        let mut methods = Vec::new();
        while *self.peek() != Tok::RBrace {
            let head = self.head()?.no_derives()?;
            if *self.peek() != Tok::Fn {
                return Err(self.unexpected("`fn` or `}`"));
            }
            let (decl, provided) = self.function(head, true, true)?;
            methods.push(TraitMethod { decl, provided });
        }
        let end = self.expect(Tok::RBrace)?;
        Ok(TraitDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            name,
            supers,
            methods,
        })
    }

    /// `impl<T> Show for Option<T> { .. }` or `impl Point { .. }`, after
    /// its head.
    fn impl_decl(&mut self, head: Head) -> Result<ImplDecl, Error> {
        let start = self.expect(Tok::Impl)?;
        let generics = self.generics()?;
        let first = self.ty()?;
        let (trait_name, self_ty) = match self.eat(&Tok::For) {
            false => (None, first),
            true => match first.kind {
                TypeExprKind::Named(name, args) if args.is_empty() => {
                    let name = Ident {
                        name,
                        span: first.span,
                    };
                    (Some(name), self.ty()?)
                }
                _ => {
                    return Err(Error::syntax(
                        first.span,
                        "expected the name of a trait before `for`",
                    ));
                }
            },
        };
        self.expect(Tok::LBrace)?;
        let mut methods = Vec::new();
        while *self.peek() != Tok::RBrace {
            let head = self.head()?.no_derives()?;
            if *self.peek() != Tok::Fn {
                return Err(self.unexpected("`fn` or `}`"));
            }
            methods.push(self.function(head, true, false)?.0);
        }
        let end = self.expect(Tok::RBrace)?;
        Ok(ImplDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            generics,
            trait_name,
            self_ty,
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
                        Tok::Ident(_) if !lifetime => p.ident()?,
                        _ if lifetime => p.lifetime()?,
                        Tok::Lifetime(_) => p.lifetime()?,
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

    /// `'a`
    fn lifetime(&mut self) -> Result<Ident, Error> {
        match self.peek().clone() {
            Tok::Lifetime(name) => Ok(Ident {
                name,
                span: self.bump().span,
            }),
            _ => Err(self.unexpected("a lifetime, like `'a`")),
        }
    }

    /// A function parameter: `x: int` or `mut x: int`.
    fn param(&mut self) -> Result<Param, Error> {
        let mutable = (*self.peek() == Tok::Mut).then(|| self.bump().span);
        let name = self.binder()?;
        self.expect(Tok::Colon)?;
        let ty = self.ty()?;
        Ok(Param {
            doc: None,
            allows: Vec::new(),
            mutable,
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
                ..p.param()?
            })
        })?;
        Ok(StructDecl {
            doc: head.doc,
            allows: head.allows,
            span: start.to(end),
            name,
            generics,
            derives: head.derives,
            fields,
        })
    }

    fn enum_decl(&mut self, head: Head) -> Result<EnumDecl, Error> {
        let start = self.expect(Tok::Enum)?;
        let name = self.ident()?;
        let generics = self.generics()?;
        self.expect(Tok::LBrace)?;
        let (variants, end) = self.comma_list(Tok::RBrace, |p| {
            let Head { doc, allows, .. } = p.head()?.no_derives()?;
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
            name,
            generics,
            derives: head.derives,
            variants,
        })
    }

    fn ty(&mut self) -> Result<TypeExpr, Error> {
        let start = self.span();
        match self.peek().clone() {
            Tok::Ident(name) => {
                self.bump();
                // There's no `<` operator in a type, so this can only start
                // type arguments.
                let (args, span) = if self.eat(&Tok::Lt) {
                    let (args, end) = self.comma_list(Tok::Gt, Self::type_arg)?;
                    (args, start.to(end))
                } else {
                    (Vec::new(), start)
                };
                Ok(TypeExpr {
                    kind: TypeExprKind::Named(name, args),
                    span,
                })
            }
            Tok::LParen => {
                self.bump();
                let end = self.expect(Tok::RParen)?;
                Ok(TypeExpr {
                    kind: TypeExprKind::Unit,
                    span: start.to(end),
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
                    }
                };
                let span = start.to(ret.span);
                Ok(TypeExpr {
                    kind: TypeExprKind::Fn(params, Box::new(ret)),
                    span,
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
                })
            }
            Tok::Bang => Ok(TypeExpr {
                kind: TypeExprKind::Never,
                span: self.bump().span,
            }),
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
        })
    }

    /// A type argument: a type, or a lifetime.
    fn type_arg(&mut self) -> Result<TypeExpr, Error> {
        match self.peek().clone() {
            Tok::Lifetime(name) => Ok(TypeExpr {
                kind: TypeExprKind::Lifetime(name),
                span: self.bump().span,
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
            if matches!(self.peek(), Tok::Fn | Tok::Struct | Tok::Enum | Tok::Hash) {
                let head = self.head()?;
                let stmt = match self.peek() {
                    Tok::Fn => Stmt::Fn(Box::new(self.fn_decl(head.no_derives()?)?)),
                    Tok::Struct => Stmt::Type(Box::new(TypeDecl::Struct(self.struct_decl(head)?))),
                    Tok::Enum => Stmt::Type(Box::new(TypeDecl::Enum(self.enum_decl(head)?))),
                    _ => {
                        return Err(self.unexpected("`fn`, `struct` or `enum` after an attribute"));
                    }
                };
                stmts.push(stmt);
                continue;
            }
            let block_like = matches!(
                self.peek(),
                Tok::If | Tok::Match | Tok::While | Tok::Loop | Tok::Lifetime(_) | Tok::LBrace
            );
            let e = match self.peek() {
                Tok::If => self.if_expr()?,
                Tok::Match => self.match_expr()?,
                Tok::While | Tok::Loop | Tok::Lifetime(_) => self.loop_expr()?,
                Tok::LBrace => self.block()?,
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
        let lhs = self.binary(0)?;
        let op = match self.peek() {
            Tok::Eq => None,
            Tok::OpEq(op) => Some(match op {
                OpEq::Add => BinOp::Add,
                OpEq::Sub => BinOp::Sub,
                OpEq::Mul => BinOp::Mul,
                OpEq::Div => BinOp::Div,
                OpEq::Rem => BinOp::Rem,
            }),
            _ => return Ok(lhs),
        };
        self.bump();
        let rhs = self.expr()?;
        let span = lhs.span.to(rhs.span);
        Ok(Expr {
            kind: ExprKind::Assign(op, Box::new(lhs), Box::new(rhs)),
            span,
        })
    }

    /// Precedence climbing: parses an expression whose operators all bind at
    /// least as tightly as `min_prec`.
    fn binary(&mut self, min_prec: u8) -> Result<Expr, Error> {
        let mut lhs = self.unary()?;
        while let Some((op, prec)) = binop(self.peek()) {
            if prec < min_prec {
                break;
            }
            self.bump();
            let rhs = self.binary(prec + 1)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
            };

            if prec == CMP_PREC && binop(self.peek()).is_some_and(|(_, p)| p == CMP_PREC) {
                return Err(Error::syntax(
                    self.span(),
                    "comparison operators cannot be chained; use `&&` (e.g. `a < b && b < c`)",
                ));
            }
        }
        Ok(lhs)
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
                });
            }
            _ => return self.postfix(),
        };
        let start = self.bump().span;
        if op == UnOp::Ref {
            return self.borrow(start);
        }
        let operand = self.unary()?;
        let span = start.to(operand.span);
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span,
        })
    }

    /// `&x` or `&mut x`, after the `&`, which is at `start`.
    fn borrow(&mut self, start: Span) -> Result<Expr, Error> {
        let op = match self.eat(&Tok::Mut) {
            true => UnOp::RefMut,
            false => UnOp::Ref,
        };
        let operand = self.unary()?;
        let span = start.to(operand.span);
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span,
        })
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut e = self.primary()?;
        loop {
            if self.eat(&Tok::LParen) {
                let (args, end) = self.args()?;
                let span = e.span.to(end);
                e = Expr {
                    kind: ExprKind::Call(Box::new(e), args),
                    span,
                };
            } else if self.eat(&Tok::Dot) {
                let name = self.ident()?;
                let (targs, _) = self.turbofish()?;
                e = match self.eat(&Tok::LParen) {
                    true => {
                        let (args, end) = self.args()?;
                        let span = e.span.to(end);
                        let kind = ExprKind::MethodCall(Box::new(e), name, targs, args);
                        Expr { kind, span }
                    }
                    false if !targs.is_empty() => {
                        return Err(self.unexpected("`(`: only a method call has type arguments"));
                    }
                    false => {
                        let span = e.span.to(name.span);
                        let kind = ExprKind::Field(Box::new(e), name);
                        Expr { kind, span }
                    }
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
            Tok::Int(n) => ExprKind::Int(n),
            Tok::True => ExprKind::Bool(true),
            Tok::False => ExprKind::Bool(false),
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
                    });
                }
                let inner = self.structs(true, Self::expr)?;
                let end = self.expect(Tok::RParen)?;
                return Ok(Expr {
                    kind: inner.kind,
                    span: span.to(end),
                });
            }
            Tok::LBrace => return self.block(),
            Tok::If => return self.if_expr(),
            Tok::Match => return self.match_expr(),
            Tok::While | Tok::Loop | Tok::Lifetime(_) => return self.loop_expr(),
            Tok::Return | Tok::Break | Tok::Continue => return self.jump(),
            Tok::Pipe | Tok::OrOr => return self.lambda(),
            _ => return Err(self.unexpected("an expression")),
        };
        self.bump();
        Ok(Expr { kind, span })
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
        let (cond, body) = match self.peek() {
            Tok::While => {
                self.bump();
                (Some(self.structs(false, Self::expr)?), self.block()?)
            }
            Tok::Loop => {
                self.bump();
                (None, self.block()?)
            }
            _ => return Err(self.unexpected("`loop` or `while` after a label")),
        };
        let span = start.to(body.span);
        let kind = match cond {
            Some(c) => ExprKind::While(label, Box::new(c), Box::new(body)),
            None => ExprKind::Loop(label, Box::new(body)),
        };
        Ok(Expr { kind, span })
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
        })
    }

    /// `::<int, bool>` after a name, if it's there: explicit type arguments.
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
    /// `Option::<int>::None`, `id::<int>` or `Pair::<int, bool> { .. }`.
    fn path_expr(&mut self) -> Result<Expr, Error> {
        let name = self.ident()?;
        let (targs, end) = self.turbofish()?;
        if let Some(end) = end
            && *self.peek() != Tok::ColonColon
        {
            if *self.peek() == Tok::LBrace && !self.no_struct {
                return self.struct_lit(name, targs);
            }
            return Ok(Expr {
                span: name.span.to(end),
                kind: ExprKind::Var(name.name, targs),
            });
        }
        self.expect(Tok::ColonColon)?;
        let variant = self.ident()?;
        let parens = self.eat(&Tok::LParen);
        let (args, end) = match parens {
            true => self.args()?,
            false => (Vec::new(), variant.span),
        };
        let span = name.span.to(end);
        Ok(Expr {
            kind: ExprKind::Variant(name, targs, variant, args, parens),
            span,
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
                    }
                };
                Ok(FieldInit { name, value })
            })
        })?;
        let span = name.span.to(end);
        Ok(Expr {
            kind: ExprKind::StructLit(name, targs, fields),
            span,
        })
    }

    /// `|x, y: int| x + y`, `|x| -> int { x }` or `|| 1`. As in Rust, the body
    /// runs as far right as an expression can, and a return type needs a
    /// block after it.
    fn lambda(&mut self) -> Result<Expr, Error> {
        let start = self.span();
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
        Ok(Expr {
            kind: ExprKind::Lambda(Arc::new(Lambda::new(params, ret, body))),
            span,
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
        })
    }

    fn pat_atom(&mut self) -> Result<Pattern, Error> {
        let start = self.span();
        let kind = match self.peek().clone() {
            Tok::Int(_) | Tok::Minus => {
                let (lo, mut end) = self.pat_int()?;
                let kind = match self.peek() {
                    Tok::DotDot => {
                        end = self.bump().span;
                        if matches!(self.peek(), Tok::Int(_) | Tok::Minus) {
                            let (hi, e) = self.pat_int()?;
                            end = e;
                            // Can't overflow: the smallest int literal is
                            // `-9223372036854775807`, one above `i64::MIN`.
                            PatternKind::Range(lo, hi - 1, false)
                        } else {
                            PatternKind::Range(lo, i64::MAX, false)
                        }
                    }
                    Tok::DotDotEq => {
                        self.bump();
                        let (hi, e) = self.pat_int()?;
                        end = e;
                        PatternKind::Range(lo, hi, true)
                    }
                    _ => PatternKind::Int(lo),
                };
                return Ok(Pattern {
                    kind,
                    span: start.to(end),
                });
            }
            Tok::DotDot | Tok::DotDotEq => {
                let inclusive = self.bump().tok == Tok::DotDotEq;
                let (hi, end) = self.pat_int()?;
                let hi = if inclusive { hi } else { hi - 1 };
                return Ok(Pattern {
                    kind: PatternKind::Range(i64::MIN, hi, inclusive),
                    span: start.to(end),
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
                });
            }
            Tok::LParen => {
                self.bump();
                let end = self.expect(Tok::RParen)?;
                return Ok(Pattern {
                    kind: PatternKind::Unit,
                    span: start.to(end),
                });
            }
            Tok::Ident(name) => match self.peek2() {
                Tok::ColonColon => {
                    let ty = self.ident()?;
                    let (targs, end) = self.turbofish()?;
                    if end.is_some() && *self.peek() == Tok::LBrace {
                        return self.struct_pattern(ty, targs);
                    }
                    self.expect(Tok::ColonColon)?;
                    let variant = self.ident()?;
                    let (args, end) = if self.eat(&Tok::LParen) {
                        self.comma_list(Tok::RParen, Self::pattern)?
                    } else {
                        (Vec::new(), variant.span)
                    };
                    let kind = PatternKind::Variant(ty, targs, variant, args);
                    return Ok(Pattern {
                        kind,
                        span: start.to(end),
                    });
                }
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
                    });
                }
            },
            _ => return Err(self.unexpected("a pattern")),
        };
        self.bump();
        Ok(Pattern { kind, span: start })
    }

    /// `5` or `-5` in a pattern.
    fn pat_int(&mut self) -> Result<(i64, Span), Error> {
        let start = self.span();
        let neg = self.eat(&Tok::Minus);
        let Tok::Int(n) = *self.peek() else {
            return Err(self.unexpected("an integer"));
        };
        let end = self.bump().span;
        Ok((if neg { -n } else { n }, start.to(end)))
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
        })
    }
}

/// Can an expression start with `tok`?
fn starts_expr(tok: &Tok) -> bool {
    matches!(
        tok,
        Tok::Int(_)
            | Tok::Ident(_)
            | Tok::True
            | Tok::False
            | Tok::LParen
            | Tok::LBrace
            | Tok::If
            | Tok::Match
            | Tok::While
            | Tok::Loop
            | Tok::Lifetime(_)
            | Tok::Return
            | Tok::Break
            | Tok::Continue
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
        _ => return None,
    })
}

/// What can come before a `fn`, `struct`, `enum`, field or variant.
struct Head {
    doc: Option<DocComment>,
    allows: Vec<Allow>,
    derives: Vec<Derive>,
}

impl Head {
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

/// One doc comment from a run of `///` lines.
fn doc_comment(lines: &[Token]) -> DocComment {
    let text = |t: &Token| match &t.tok {
        Tok::Doc(text) => text.clone(),
        _ => unreachable!("only doc comments are set aside"),
    };
    DocComment {
        lines: lines
            .iter()
            .map(|t| DocLine {
                text: text(t),
                span: t.span,
            })
            .collect(),
    }
}
