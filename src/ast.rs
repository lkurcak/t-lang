//! The abstract syntax tree: what the parser produces and the checker reads.
//!
//! Note that `TypeExpr` (a type as *written* in the source) is separate from
//! `types::Type` (a type as the checker *understands* it). The checker turns
//! one into the other and rejects names that don't mean anything.

use crate::error::Span;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Program {
    pub types: Vec<TypeDecl>,
    pub fns: Vec<FnDecl>,
}

/// `struct Point { x: int, y: int }` or `enum Shape { Circle(int), Empty }`.
#[derive(Debug, Clone)]
pub enum TypeDecl {
    Struct(StructDecl),
    Enum(EnumDecl),
}

impl TypeDecl {
    pub fn name(&self) -> &Ident {
        match self {
            TypeDecl::Struct(s) => &s.name,
            TypeDecl::Enum(e) => &e.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<Param>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: Ident,
    pub variants: Vec<Variant>,
}

/// One case of an enum: `Rect(int, int)`, or `Empty` with no fields.
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: Ident,
    pub fields: Vec<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: TypeExpr,
    pub body: Expr,
}

#[derive(Debug, Clone)]
/// `name: type`. Used for function parameters and struct fields.
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
    /// `int`, `bool`, a struct or enum, or a name that doesn't exist (caught
    /// by the checker).
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
    /// `Point { x: 1, y: 2 }`
    StructLit(Ident, Vec<FieldInit>),
    /// `p.x`
    Field(Box<Expr>, Ident),
    /// `Shape::Rect(2, 3)`, or `Shape::Empty` with no arguments.
    Variant(Ident, Ident, Vec<Expr>),
    /// `match e { pat => e, .. }`
    Match(Box<Expr>, Vec<Arm>),
    /// `|x, y: int| x + y`. Shared, because the closure values the
    /// interpreter makes from it point to it.
    Lambda(Arc<Lambda>),
}

/// An anonymous function: `|x| x + 1`, `|x: int| -> int { x + 1 }`, or `|| 1`
/// with no parameters. Any type left out is inferred.
#[derive(Debug)]
pub struct Lambda {
    pub params: Vec<LambdaParam>,
    pub ret: Option<TypeExpr>,
    pub body: Expr,
}

#[derive(Debug)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct Arm {
    pub pat: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let pat: ty = init;`. The pattern must be irrefutable: `let x = ..`,
    /// `let Point { x, y } = ..`, but not `let Shape::Empty = ..`.
    Let {
        pat: Pattern,
        ty: Option<TypeExpr>,
        init: Expr,
    },
    /// `e;`: any type, value discarded.
    Expr(Expr),
    /// An `if`, `match` or block statement with no `;` after it, like
    /// `if c { print(1) }`. As in Rust, it must have type `()`.
    BlockLike(Expr),
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    /// `_`: matches anything, binds nothing.
    Wild,
    /// `x`: matches anything and names it.
    Bind(String),
    Int(i64),
    /// An int range, with both bounds inclusive: `1..=9` is `Range(1, 9, true)`,
    /// `1..10` is `Range(1, 9, false)`, `10..` runs to the largest int and
    /// `..=-1` from the smallest. The `bool` says whether it was written with
    /// `..=`, so suggestions can keep the style. `lo > hi` means the range is
    /// empty, which the checker rejects.
    Range(i64, i64, bool),
    Bool(bool),
    Unit,
    /// `Shape::Rect(w, _)`
    Variant(Ident, Ident, Vec<Pattern>),
    /// `Point { x, y: 0 }`, or `Point { x, .. }` to ignore the other fields
    /// (the `bool`).
    Struct(Ident, Vec<FieldPat>, bool),
    /// `Shape::Circle(r) | Shape::Rect(r, _)`: matches if any alternative
    /// does. Every alternative must bind the same variables, at the same types.
    Or(Vec<Pattern>),
}

/// `y: 0` in a struct pattern. Plain `x` is short for `x: x`.
#[derive(Debug, Clone)]
pub struct FieldPat {
    pub name: Ident,
    pub pat: Pattern,
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
