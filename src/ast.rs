//! The abstract syntax tree: what the parser produces and the checker reads.
//!
//! Note that `TypeExpr` (a type as *written* in the source) is separate from
//! `types::Type` (a type as the checker *understands* it). The checker turns
//! one into the other and rejects names that don't mean anything.

use crate::error::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub fns: Vec<FnDecl>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: TypeExpr,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExprKind {
    /// `int`, `bool`, or a name that doesn't exist (caught by the checker).
    Named(String),
    /// `()`
    Unit,
    /// `fn(int, bool) -> int`
    Fn(Vec<TypeExpr>, Box<TypeExpr>),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Bool(bool),
    Unit,
    Var(String),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    /// `if c { .. } else { .. }`; the `else` is optional.
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// `{ stmt; stmt; tail }`; without a tail the block's value is `()`.
    Block(Vec<Stmt>, Option<Box<Expr>>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: Ident, ty: Option<TypeExpr>, init: Expr },
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
        })
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        })
    }
}
