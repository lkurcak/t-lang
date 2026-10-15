//! The abstract syntax tree: what the parser produces and the checker reads.
//!
//! Note that `TypeExpr` (a type as *written* in the source) is separate from
//! `types::Type` (a type as the checker *understands* it). The checker turns
//! one into the other and rejects names that don't mean anything.

use crate::error::Span;
use crate::types::comma_list;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Program {
    pub types: Vec<TypeDecl>,
    pub fns: Vec<FnDecl>,
    /// Doc comments that aren't before anything they can document, in
    /// order. The checker warns about each.
    pub stray_docs: Vec<DocComment>,
}

/// A `///` doc comment, of one or more lines in a row, before the `fn`,
/// `struct`, `enum`, field or variant it documents.
#[derive(Debug, Clone)]
pub struct DocComment {
    pub lines: Vec<DocLine>,
}

#[derive(Debug, Clone)]
pub struct DocLine {
    /// What's after the `///`, as written: `/// Adds one.` is `" Adds one."`.
    pub text: String,
    /// The whole line, `///` and all.
    pub span: Span,
}

impl DocComment {
    /// From the start of its first line to the end of its last.
    pub fn span(&self) -> Span {
        let (first, last) = (&self.lines[0], &self.lines[self.lines.len() - 1]);
        first.span.to(last.span)
    }

    /// The text as Markdown: the lines, without the indentation they all
    /// share, as rustdoc does.
    pub fn markdown(&self) -> String {
        let indent = self
            .lines
            .iter()
            .map(|l| &l.text)
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let lines: Vec<&str> = self
            .lines
            .iter()
            .map(|l| l.text.get(indent..).unwrap_or(""))
            .collect();
        lines.join("\n")
    }
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

    pub fn doc(&self) -> Option<&DocComment> {
        match self {
            TypeDecl::Struct(s) => s.doc.as_ref(),
            TypeDecl::Enum(e) => e.doc.as_ref(),
        }
    }

    /// From the `struct` or `enum` to the `}`.
    pub fn span(&self) -> Span {
        match self {
            TypeDecl::Struct(s) => s.span,
            TypeDecl::Enum(e) => e.span,
        }
    }

    pub fn type_params(&self) -> &[Ident] {
        match self {
            TypeDecl::Struct(s) => &s.type_params,
            TypeDecl::Enum(e) => &e.type_params,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub doc: Option<DocComment>,
    /// From the `struct` to the `}`, without the doc comment.
    pub span: Span,
    pub name: Ident,
    /// `<A, B>` in `struct Pair<A, B> { .. }`; empty if it isn't generic.
    pub type_params: Vec<Ident>,
    pub fields: Vec<Param>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub doc: Option<DocComment>,
    /// From the `enum` to the `}`, without the doc comment.
    pub span: Span,
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub variants: Vec<Variant>,
}

/// One case of an enum: `Rect(int, int)`, or `Empty` with no fields.
#[derive(Debug, Clone)]
pub struct Variant {
    pub doc: Option<DocComment>,
    /// From the name to the `)` after the fields, if it has any.
    pub span: Span,
    pub name: Ident,
    pub fields: Vec<TypeExpr>,
}

/// A function: at the top level, or in a block, where it's *nested*. A
/// nested `fn` means what it would at the top level, so it can't use the
/// locals around it; only its name is scoped, to the block it's in.
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub doc: Option<DocComment>,
    /// From the `fn` to the end of the body, without the doc comment.
    pub span: Span,
    pub name: Ident,
    /// `<T>` in `fn id<T>(x: T) -> T`; empty if it isn't generic.
    pub type_params: Vec<Ident>,
    pub params: Vec<Param>,
    /// From the `(` before the parameters to the `)` after them.
    pub parens: Span,
    pub ret: TypeExpr,
    pub body: Expr,
}

impl Program {
    /// Every function, nested ones too: the top-level ones in order, then
    /// the ones nested in those, and so on. Each comes with the index in
    /// this list of the function it's nested in, if it's nested.
    pub fn all_fns(&self) -> Vec<(&FnDecl, Option<usize>)> {
        let mut out: Vec<(&FnDecl, Option<usize>)> = self.fns.iter().map(|f| (f, None)).collect();
        let mut i = 0;
        while i < out.len() {
            let mut fns = Vec::new();
            nested(&out[i].0.body, Stmt::as_fn, &mut fns);
            out.extend(fns.into_iter().map(|f| (f, Some(i))));
            i += 1;
        }
        out
    }

    /// Every struct and enum, local ones too: the top-level ones in order,
    /// then the ones declared in each function in `all_fns`, in that order.
    /// Each comes with the index in `all_fns` of the function it's in, if
    /// it's local.
    pub fn all_types(&self) -> Vec<(&TypeDecl, Option<usize>)> {
        let mut out: Vec<(&TypeDecl, Option<usize>)> =
            self.types.iter().map(|d| (d, None)).collect();
        for (i, (f, _)) in self.all_fns().into_iter().enumerate() {
            let mut local = Vec::new();
            nested(&f.body, Stmt::as_type, &mut local);
            out.extend(local.into_iter().map(|d| (d, Some(i))));
        }
        out
    }
}

/// Adds what `item` finds among the statements of the blocks in `e` to
/// `out`, in order, but not in the functions nested in them.
fn nested<'a, T>(e: &'a Expr, item: fn(&'a Stmt) -> Option<&'a T>, out: &mut Vec<&'a T>) {
    if let ExprKind::Block(stmts, _) = &e.kind {
        out.extend(stmts.iter().filter_map(item));
    }
    for c in e.children() {
        nested(c, item, out);
    }
}

impl Expr {
    /// The expressions directly inside this one, in order. Not the bodies
    /// of the functions declared in it, which are separate functions.
    pub fn children(&self) -> Vec<&Expr> {
        match &self.kind {
            ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit | ExprKind::Var(..) => Vec::new(),
            ExprKind::Unary(_, x) | ExprKind::Field(x, _) => vec![x],
            ExprKind::Binary(_, l, r) => vec![l, r],
            ExprKind::Call(f, args) => std::iter::once(&**f).chain(args).collect(),
            ExprKind::If(c, then, els) => [&**c, then].into_iter().chain(els.as_deref()).collect(),
            ExprKind::Block(stmts, tail) => stmts
                .iter()
                .filter_map(|s| match s {
                    Stmt::Let { init: e, .. } | Stmt::Expr(e) | Stmt::BlockLike(e) => Some(e),
                    Stmt::Fn(_) | Stmt::Type(_) => None,
                })
                .chain(tail.as_deref())
                .collect(),
            ExprKind::StructLit(_, _, inits) => inits.iter().map(|i| &i.value).collect(),
            ExprKind::Variant(_, _, _, args) => args.iter().collect(),
            ExprKind::Match(scrutinee, arms) => std::iter::once(&**scrutinee)
                .chain(arms.iter().map(|a| &a.body))
                .collect(),
            ExprKind::Lambda(l) => vec![&l.body],
        }
    }

    /// Does `name` appear anywhere in this, in the bodies of functions
    /// declared in it too? Whatever it refers to there.
    pub fn mentions(&self, name: &str) -> bool {
        match &self.kind {
            ExprKind::Var(n, _) => n == name,
            ExprKind::Block(stmts, _)
                if stmts
                    .iter()
                    .filter_map(Stmt::as_fn)
                    .any(|f| f.body.mentions(name)) =>
            {
                true
            }
            _ => self.children().iter().any(|c| c.mentions(name)),
        }
    }
}

#[derive(Debug, Clone)]
/// `name: type`. Used for function parameters and struct fields.
pub struct Param {
    /// Only a struct field can have one.
    pub doc: Option<DocComment>,
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
    /// `int`, `bool`, a type parameter, a struct or enum with its type
    /// arguments (`Option<int>`), or a name that doesn't exist (caught by the
    /// checker).
    Named(String, Vec<TypeExpr>),
    /// `()`
    Unit,
    /// `fn(int, bool) -> int`
    Fn(Vec<TypeExpr>, Box<TypeExpr>),
}

/// As written: `Option<int>`, `fn(int) -> bool`, `()`.
impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            TypeExprKind::Named(name, args) if args.is_empty() => f.write_str(name),
            TypeExprKind::Named(name, args) => write!(f, "{name}<{}>", comma_list(args)),
            TypeExprKind::Unit => f.write_str("()"),
            TypeExprKind::Fn(params, ret) => write!(f, "fn({}) -> {ret}", comma_list(params)),
        }
    }
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
    /// `x`, or `id::<int>` with type arguments (a *turbofish*) for a generic
    /// function. Every list of type arguments here and below is empty if none
    /// were written, and then the checker infers them.
    Var(String, Vec<TypeExpr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    /// `if c { .. } else { .. }`; the `else` is optional.
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// `{ stmt; stmt; tail }`; without a tail the block's value is `()`.
    Block(Vec<Stmt>, Option<Box<Expr>>),
    /// `Point { x: 1, y: 2 }`, or `Pair::<int, bool> { .. }`
    StructLit(Ident, Vec<TypeExpr>, Vec<FieldInit>),
    /// `p.x`
    Field(Box<Expr>, Ident),
    /// `Shape::Rect(2, 3)`, `Shape::Empty` with no arguments, or
    /// `Option::<int>::None` with type arguments.
    Variant(Ident, Vec<TypeExpr>, Ident, Vec<Expr>),
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
    /// The names the body uses but doesn't bind itself: its *free
    /// variables*, which a closure made from it has to capture. Some may be
    /// functions, which need no capturing.
    pub captures: Vec<String>,
}

impl Lambda {
    pub fn new(params: Vec<LambdaParam>, ret: Option<TypeExpr>, body: Expr) -> Lambda {
        let mut bound: Vec<String> = params.iter().map(|p| p.name.name.clone()).collect();
        let mut captures = Vec::new();
        free_vars(&body, &mut bound, &mut captures);
        Lambda {
            params,
            ret,
            body,
            captures,
        }
    }
}

/// Adds the variables `e` uses that aren't in `bound` to `out`, once each.
/// `bound` grows and shrinks with the scopes inside `e`, so a `let` or
/// pattern in there shadows an outer name only where it's in scope.
fn free_vars(e: &Expr, bound: &mut Vec<String>, out: &mut Vec<String>) {
    let mut go = |e: &Expr, bound: &mut Vec<String>| free_vars(e, bound, out);
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Unit => {}
        ExprKind::Var(name, _) => {
            if !bound.contains(name) && !out.contains(name) {
                out.push(name.clone());
            }
        }
        ExprKind::Unary(_, x) | ExprKind::Field(x, _) => go(x, bound),
        ExprKind::Binary(_, l, r) => {
            go(l, bound);
            go(r, bound);
        }
        ExprKind::Call(f, args) => {
            go(f, bound);
            args.iter().for_each(|a| go(a, bound));
        }
        ExprKind::If(c, then, els) => {
            go(c, bound);
            go(then, bound);
            els.iter().for_each(|e| go(e, bound));
        }
        ExprKind::Block(stmts, tail) => {
            let scope = bound.len();
            // A nested `fn`'s name is in scope in the whole block, and its
            // body is a function of its own, which can't capture anything.
            let fns = stmts.iter().filter_map(Stmt::as_fn);
            bound.extend(fns.map(|f| f.name.name.clone()));
            for stmt in stmts {
                match stmt {
                    Stmt::Let { pat, init, .. } => {
                        go(init, bound);
                        pattern_binds(pat, bound);
                    }
                    Stmt::Expr(e) | Stmt::BlockLike(e) => go(e, bound),
                    Stmt::Fn(_) | Stmt::Type(_) => {}
                }
            }
            tail.iter().for_each(|t| go(t, bound));
            bound.truncate(scope);
        }
        ExprKind::StructLit(_, _, inits) => inits.iter().for_each(|i| go(&i.value, bound)),
        ExprKind::Variant(_, _, _, args) => args.iter().for_each(|a| go(a, bound)),
        ExprKind::Match(scrutinee, arms) => {
            go(scrutinee, bound);
            for arm in arms {
                let scope = bound.len();
                pattern_binds(&arm.pat, bound);
                go(&arm.body, bound);
                bound.truncate(scope);
            }
        }
        ExprKind::Lambda(l) => {
            let scope = bound.len();
            bound.extend(l.params.iter().map(|p| p.name.name.clone()));
            go(&l.body, bound);
            bound.truncate(scope);
        }
    }
}

/// Adds the variables `p` binds to `out`.
fn pattern_binds(p: &Pattern, out: &mut Vec<String>) {
    match &p.kind {
        PatternKind::Bind(name) => out.push(name.clone()),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
            pats.iter().for_each(|p| pattern_binds(p, out));
        }
        PatternKind::Struct(_, _, fields, _) => {
            fields.iter().for_each(|f| pattern_binds(&f.pat, out))
        }
        PatternKind::Wild
        | PatternKind::Int(_)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
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
        /// Boxed, since it's rare and would make every `Stmt` bigger.
        ty: Option<Box<TypeExpr>>,
        init: Expr,
    },
    /// `e;`: any type, value discarded.
    Expr(Expr),
    /// An `if`, `match` or block statement with no `;` after it, like
    /// `if c { print(1) }`. As in Rust, it must have type `()`.
    BlockLike(Expr),
    /// `fn helper() { .. }`: a nested function. Its name is in scope in the
    /// whole block, before the `fn` as well as after. Boxed, like `Let`'s
    /// type.
    Fn(Box<FnDecl>),
    /// `struct Point { .. }` or `enum Shape { .. }`: a local type. Like a
    /// nested function, it means what it would at the top level, and its
    /// name is in scope in the whole block.
    Type(Box<TypeDecl>),
}

impl Stmt {
    pub fn as_fn(&self) -> Option<&FnDecl> {
        match self {
            Stmt::Fn(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_type(&self) -> Option<&TypeDecl> {
        match self {
            Stmt::Type(d) => Some(d),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    /// The variables it binds.
    pub fn binds(&self) -> Vec<String> {
        let mut out = Vec::new();
        pattern_binds(self, &mut out);
        out
    }
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
    /// `Shape::Rect(w, _)`, or `Option::<int>::Some(x)` with type arguments.
    Variant(Ident, Vec<TypeExpr>, Ident, Vec<Pattern>),
    /// `Point { x, y: 0 }`, or `Point { x, .. }` to ignore the other fields
    /// (the `bool`).
    Struct(Ident, Vec<TypeExpr>, Vec<FieldPat>, bool),
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
