//! Recursive-descent parser: tokens -> AST.
//!
//! Grammar (lowest to highest precedence for operators):
//!
//! ```text
//! program  = fn_decl*
//! fn_decl  = "fn" IDENT "(" (param ("," param)* ","?)? ")" ("->" type)? block
//! param    = IDENT ":" type
//! type     = IDENT | "(" ")" | "fn" "(" (type ("," type)*)? ")" ("->" type)?
//! block    = "{" (let | expr ";" | block_like_expr)* expr? "}"
//! let      = "let" IDENT (":" type)? "=" expr ";"
//! expr     = or
//! or       = and ("||" and)*
//! and      = cmp ("&&" cmp)*
//! cmp      = add (("==" | "!=" | "<" | "<=" | ">" | ">=") add)?   -- no chaining
//! add      = mul (("+" | "-") mul)*
//! mul      = unary (("*" | "/" | "%") unary)*
//! unary    = ("-" | "!") unary | call
//! call     = primary ("(" args ")")*
//! primary  = INT | "true" | "false" | IDENT | "(" ")" | "(" expr ")" | block | if
//! if       = "if" expr block ("else" (block | if))?
//! ```

use crate::ast::*;
use crate::error::{Error, Span};
use crate::lexer::{Tok, Token};

pub fn parse(tokens: Vec<Token>) -> Result<Program, Error> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut fns = Vec::new();
    while *p.peek() != Tok::Eof {
        fns.push(p.fn_decl()?);
    }
    Ok(Program { fns })
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
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
        Error::syntax(self.span(), format!("expected {wanted}, found {}", self.peek().describe()))
    }

    fn ident(&mut self) -> Result<Ident, Error> {
        match self.peek().clone() {
            Tok::Ident(name) => Ok(Ident { name, span: self.bump().span }),
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
            return Err(self.unexpected("`fn`"));
        }
        self.bump();
        let name = self.ident()?;
        self.expect(Tok::LParen)?;
        let (params, close) = self.comma_list(Tok::RParen, |p| {
            let name = p.ident()?;
            p.expect(Tok::Colon)?;
            let ty = p.ty()?;
            Ok(Param { name, ty })
        })?;
        let ret = if self.eat(&Tok::Arrow) {
            self.ty()?
        } else {
            TypeExpr { kind: TypeExprKind::Unit, span: close }
        };
        let body = self.block()?;
        Ok(FnDecl { name, params, ret, body })
    }

    fn ty(&mut self) -> Result<TypeExpr, Error> {
        let start = self.span();
        match self.peek().clone() {
            Tok::Ident(name) => {
                self.bump();
                Ok(TypeExpr { kind: TypeExprKind::Named(name), span: start })
            }
            Tok::LParen => {
                self.bump();
                let end = self.expect(Tok::RParen)?;
                Ok(TypeExpr { kind: TypeExprKind::Unit, span: start.to(end) })
            }
            Tok::Fn => {
                self.bump();
                self.expect(Tok::LParen)?;
                let (params, close) = self.comma_list(Tok::RParen, Self::ty)?;
                let ret = if self.eat(&Tok::Arrow) {
                    self.ty()?
                } else {
                    TypeExpr { kind: TypeExprKind::Unit, span: close }
                };
                let span = start.to(ret.span);
                Ok(TypeExpr { kind: TypeExprKind::Fn(params, Box::new(ret)), span })
            }
            _ => Err(self.unexpected("a type")),
        }
    }

    fn block(&mut self) -> Result<Expr, Error> {
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
            let e = self.expr()?;
            if self.eat(&Tok::Semi) {
                stmts.push(Stmt::Expr(e));
            } else if *self.peek() == Tok::RBrace {
                break Some(Box::new(e));
            } else if matches!(e.kind, ExprKind::If(..) | ExprKind::Block(..)) {
                // Like Rust, `if`/blocks used as statements don't need a `;`.
                stmts.push(Stmt::Expr(e));
            } else {
                return Err(self.unexpected("`;` or `}`"));
            }
        };
        let end = self.expect(Tok::RBrace)?;
        Ok(Expr { kind: ExprKind::Block(stmts, tail), span: start.to(end) })
    }

    fn let_stmt(&mut self) -> Result<Stmt, Error> {
        self.expect(Tok::Let)?;
        let name = self.ident()?;
        let ty = if self.eat(&Tok::Colon) { Some(self.ty()?) } else { None };
        self.expect(Tok::Eq)?;
        let init = self.expr()?;
        self.expect(Tok::Semi)?;
        Ok(Stmt::Let { name, ty, init })
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
            lhs = Expr { kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)), span };

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
            _ => return self.call(),
        };
        let start = self.bump().span;
        let operand = self.unary()?;
        let span = start.to(operand.span);
        Ok(Expr { kind: ExprKind::Unary(op, Box::new(operand)), span })
    }

    fn call(&mut self) -> Result<Expr, Error> {
        let mut e = self.primary()?;
        while self.eat(&Tok::LParen) {
            let (args, end) = self.comma_list(Tok::RParen, Self::expr)?;
            let span = e.span.to(end);
            e = Expr { kind: ExprKind::Call(Box::new(e), args), span };
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let span = self.span();
        let kind = match self.peek().clone() {
            Tok::Int(n) => ExprKind::Int(n),
            Tok::True => ExprKind::Bool(true),
            Tok::False => ExprKind::Bool(false),
            Tok::Ident(name) => ExprKind::Var(name),
            Tok::LParen => {
                self.bump();
                if *self.peek() == Tok::RParen {
                    let end = self.bump().span;
                    return Ok(Expr { kind: ExprKind::Unit, span: span.to(end) });
                }
                let inner = self.expr()?;
                let end = self.expect(Tok::RParen)?;
                return Ok(Expr { kind: inner.kind, span: span.to(end) });
            }
            Tok::LBrace => return self.block(),
            Tok::If => return self.if_expr(),
            _ => return Err(self.unexpected("an expression")),
        };
        self.bump();
        Ok(Expr { kind, span })
    }

    fn if_expr(&mut self) -> Result<Expr, Error> {
        let start = self.expect(Tok::If)?;
        let cond = self.expr()?;
        let then = self.block()?;
        let els = if self.eat(&Tok::Else) {
            Some(Box::new(if *self.peek() == Tok::If { self.if_expr()? } else { self.block()? }))
        } else {
            None
        };
        let span = start.to(els.as_ref().map_or(then.span, |e| e.span));
        Ok(Expr { kind: ExprKind::If(Box::new(cond), Box::new(then), els), span })
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
