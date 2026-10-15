//! Recursive-descent parser: tokens -> AST.
//!
//! Grammar (lowest to highest precedence for operators):
//!
//! ```text
//! program  = (fn_decl | struct | enum)*
//! fn_decl  = "fn" IDENT generics? "(" (param ("," param)* ","?)? ")" ("->" type)? block
//! generics = "<" (IDENT ("," IDENT)* ","?)? ">"
//! param    = IDENT ":" type
//! struct   = "struct" IDENT generics? "{" (param ("," param)* ","?)? "}"
//! enum     = "enum" IDENT generics? "{" (variant ("," variant)* ","?)? "}"
//! variant  = IDENT ("(" type ("," type)* ","? ")")?
//! type     = IDENT ("<" type ("," type)* ","? ">")? | "(" ")"
//!          | "fn" "(" (type ("," type)*)? ")" ("->" type)?
//! block    = "{" (let | fn_decl | expr ";" | block_like ";"?)* expr? "}"
//! block_like = if | match | block
//! let      = "let" pattern (":" type)? "=" expr ";"
//! expr     = or
//! or       = and ("||" and)*
//! and      = cmp ("&&" cmp)*
//! cmp      = add (("==" | "!=" | "<" | "<=" | ">" | ">=") add)?   -- no chaining
//! add      = mul (("+" | "-") mul)*
//! mul      = unary (("*" | "/" | "%") unary)*
//! unary    = ("-" | "!") unary | call
//! postfix  = primary ("(" args ")" | "." IDENT)*
//! primary  = INT | "true" | "false" | IDENT targs? | "(" ")" | "(" expr ")" | block
//!          | if | match | IDENT targs? "{" field_init,* "}"
//!          | IDENT targs? "::" IDENT ("(" args ")")? | lambda
//! targs    = "::" "<" type ("," type)* ","? ">"            -- the "turbofish"
//! lambda   = ("||" | "|" (lparam ("," lparam)* ","?)? "|") ("->" type block | expr)
//! lparam   = IDENT (":" type)?
//! if       = "if" expr block ("else" (block | if))?
//! match    = "match" expr "{" (pattern "=>" expr ","?)* "}"  -- `,` optional after a block/if/match
//! field_init = IDENT (":" expr)?                             -- `x` is short for `x: x`
//!
//! pattern  = pat_atom ("|" pat_atom)*
//! pat_atom = "_" | IDENT | int | range | "true" | "false" | "(" ")"
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
//! A `fn` in a block is a nested function (see `ast::FnDecl`). Only `fn`
//! can be declared there, for now: not `struct` or `enum`.
//!
//! A `///` doc comment documents the `fn`, `struct`, `enum`, field or
//! variant after it. The parser takes doc comments out of the tokens first,
//! and attaches each run of them to what starts right after it (`doc`). The
//! ones nothing takes are `Program::stray_docs`.
//!
//! One ambiguity: in `if x { .. }` the `x {` could start a struct literal. Like
//! Rust, struct literals aren't allowed directly in the head of an `if` or
//! `match` (write `if (P { .. }) == p` instead). `no_struct` tracks that.

use crate::ast::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::lexer::{Tok, Token};
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
    let mut fns = Vec::new();
    loop {
        match p.peek() {
            Tok::Eof => break,
            Tok::Struct => types.push(TypeDecl::Struct(p.struct_decl()?)),
            Tok::Enum => types.push(TypeDecl::Enum(p.enum_decl()?)),
            _ => fns.push(p.fn_decl()?),
        }
    }
    let stray_docs = p.stray_docs();
    Ok(Program {
        types,
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

    fn fn_decl(&mut self) -> Result<FnDecl, Error> {
        if *self.peek() != Tok::Fn {
            return Err(self.unexpected("`fn`, `struct` or `enum`"));
        }
        let doc = self.doc();
        let start = self.bump().span;
        let name = self.ident()?;
        let type_params = self.generics()?;
        let open = self.expect(Tok::LParen)?;
        let (params, close) = self.comma_list(Tok::RParen, Self::param)?;
        let ret = if self.eat(&Tok::Arrow) {
            self.ty()?
        } else {
            TypeExpr {
                kind: TypeExprKind::Unit,
                span: close,
            }
        };
        let body = self.block()?;
        Ok(FnDecl {
            doc,
            span: start.to(body.span),
            name,
            type_params,
            params,
            parens: open.to(close),
            ret,
            body,
        })
    }

    /// `<T, U>` after the name of a function, struct or enum, if it's there.
    fn generics(&mut self) -> Result<Vec<Ident>, Error> {
        if self.eat(&Tok::Lt) {
            Ok(self.comma_list(Tok::Gt, Self::ident)?.0)
        } else {
            Ok(Vec::new())
        }
    }

    fn param(&mut self) -> Result<Param, Error> {
        let name = self.ident()?;
        self.expect(Tok::Colon)?;
        let ty = self.ty()?;
        Ok(Param {
            doc: None,
            name,
            ty,
        })
    }

    fn struct_decl(&mut self) -> Result<StructDecl, Error> {
        let doc = self.doc();
        self.expect(Tok::Struct)?;
        let name = self.ident()?;
        let type_params = self.generics()?;
        self.expect(Tok::LBrace)?;
        let (fields, _) = self.comma_list(Tok::RBrace, |p| {
            let doc = p.doc();
            Ok(Param { doc, ..p.param()? })
        })?;
        Ok(StructDecl {
            doc,
            name,
            type_params,
            fields,
        })
    }

    fn enum_decl(&mut self) -> Result<EnumDecl, Error> {
        let doc = self.doc();
        self.expect(Tok::Enum)?;
        let name = self.ident()?;
        let type_params = self.generics()?;
        self.expect(Tok::LBrace)?;
        let (variants, _) = self.comma_list(Tok::RBrace, |p| {
            let doc = p.doc();
            let name = p.ident()?;
            let fields = if p.eat(&Tok::LParen) {
                p.comma_list(Tok::RParen, Self::ty)?.0
            } else {
                Vec::new()
            };
            Ok(Variant { doc, name, fields })
        })?;
        Ok(EnumDecl {
            doc,
            name,
            type_params,
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
                    let (args, end) = self.comma_list(Tok::Gt, Self::ty)?;
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
            _ => Err(self.unexpected("a type")),
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
            match self.peek() {
                Tok::Let => {
                    stmts.push(self.let_stmt()?);
                    continue;
                }
                Tok::Fn => {
                    stmts.push(Stmt::Fn(Box::new(self.fn_decl()?)));
                    continue;
                }
                Tok::Struct | Tok::Enum => {
                    let what = match self.peek() {
                        Tok::Struct => "a `struct`",
                        _ => "an `enum`",
                    };
                    return Err(Error::syntax(
                        self.span(),
                        format!(
                            "{what} can't be declared in a block, only at the top level; \
                             for now only `fn` can be nested"
                        ),
                    ));
                }
                _ => {}
            }
            let block_like = matches!(self.peek(), Tok::If | Tok::Match | Tok::LBrace);
            let e = match self.peek() {
                Tok::If => self.if_expr()?,
                Tok::Match => self.match_expr()?,
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
        self.expect(Tok::Let)?;
        let pat = self.pattern()?;
        let ty = if self.eat(&Tok::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(Tok::Eq)?;
        let init = self.expr()?;
        self.expect(Tok::Semi)?;
        Ok(Stmt::Let {
            pat,
            ty: ty.map(Box::new),
            init,
        })
    }

    fn expr(&mut self) -> Result<Expr, Error> {
        self.binary(0)
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
            _ => return self.postfix(),
        };
        let start = self.bump().span;
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
                let field = self.ident()?;
                let span = e.span.to(field.span);
                e = Expr {
                    kind: ExprKind::Field(Box::new(e), field),
                    span,
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
                ExprKind::Block(..) | ExprKind::If(..) | ExprKind::Match(..)
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
            let (args, end) = self.comma_list(Tok::Gt, Self::ty)?;
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
        let (args, end) = if self.eat(&Tok::LParen) {
            self.args()?
        } else {
            (Vec::new(), variant.span)
        };
        let span = name.span.to(end);
        Ok(Expr {
            kind: ExprKind::Variant(name, targs, variant, args),
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
                let name = p.ident()?;
                let ty = if p.eat(&Tok::Colon) {
                    Some(p.ty()?)
                } else {
                    None
                };
                Ok(LambdaParam { name, ty })
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
                _ => PatternKind::Bind(name),
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
            let name = self.ident()?;
            let pat = if self.eat(&Tok::Colon) {
                self.pattern()?
            } else {
                Pattern {
                    kind: PatternKind::Bind(name.name.clone()),
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
