//! Prints a program back out as source, for `t mono` and `t fmt`. A
//! program without generics prints as T that parses back to the same
//! program; the names monomorphisation gives its copies, like `id::<int>`,
//! don't parse.
//!
//! For `t fmt` the printer also has the source's comments and blank lines
//! (`Trivia`), and puts them back as it goes.

use crate::ast::*;
use crate::lexer::Tok;
use crate::trivia::Trivia;
use std::fmt::{self, Write};

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut p = Printer::default();
        for d in &self.types {
            p.start_line(true);
            p.type_decl(d);
        }
        for func in &self.fns {
            p.start_line(true);
            p.fn_decl(func);
        }
        p.out.push('\n');
        f.write_str(&p.out)
    }
}

/// Prints `program` with the comments and blank lines in `trivia`, and its
/// items in the order they're written.
pub fn format(program: &Program, trivia: Trivia) -> String {
    enum Item<'a> {
        Type(&'a TypeDecl),
        Fn(&'a FnDecl),
    }
    let mut p = Printer {
        trivia: Some(trivia),
        ..Printer::default()
    };
    let mut items: Vec<(usize, Item)> = program
        .types
        .iter()
        .map(|d| (d.name().span.start, Item::Type(d)))
        .chain(program.fns.iter().map(|f| (f.name.span.start, Item::Fn(f))))
        .collect();
    items.sort_by_key(|(pos, _)| *pos);
    for (pos, item) in items {
        // Starts at the `struct`, `enum` or `fn` before the name.
        let start = p.before(pos);
        p.element(start, false);
        let end = match item {
            Item::Type(d) => {
                p.type_decl(d);
                p.find(pos, &Tok::RBrace) + 1
            }
            Item::Fn(f) => {
                p.fn_decl(f);
                f.body.span.end
            }
        };
        p.trailing(end);
    }
    p.dangling(usize::MAX, false);
    if !p.out.is_empty() {
        p.out.push('\n');
    }
    p.out
}

/// How tightly a binary operator binds, as in the parser.
fn prec(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 3,
        BinOp::Add | BinOp::Sub => 4,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 5,
    }
}

/// Binds tighter than any binary operator: unary operators, calls and
/// everything else.
const UNARY: u8 = 6;

fn block_like(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::If(..) | ExprKind::Match(..) | ExprKind::Block(..)
    )
}

/// Does `e` start with an `if`, `match` or block, without being one? At
/// the start of a statement, that would end the statement early.
fn starts_block_like(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Binary(_, l, _) => block_like(l) || starts_block_like(l),
        ExprKind::Call(f, _) | ExprKind::Field(f, _) => block_like(f) || starts_block_like(f),
        _ => false,
    }
}

#[derive(Default)]
struct Printer<'s> {
    out: String,
    indent: usize,
    /// The source's comments and blank lines, when formatting it.
    trivia: Option<Trivia<'s>>,
}

impl Printer<'_> {
    fn line(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }

    /// Starts a line, after a blank one if `blank`, unless nothing has been
    /// printed yet.
    fn start_line(&mut self, blank: bool) {
        if self.out.is_empty() {
            return;
        }
        if blank {
            self.out.push('\n');
        }
        self.line();
    }

    /// Starts the line for something that starts at `pos` in the source: an
    /// item, statement, arm or list element, after the comments before it.
    /// Keeps a blank line before it, unless it's `first` in its block or
    /// list.
    fn element(&mut self, pos: usize, first: bool) {
        let Some(t) = &mut self.trivia else {
            self.start_line(false);
            return;
        };
        let comments = t.leading(pos);
        let blank = t.blank_before(pos);
        let mut first = first;
        for c in comments {
            self.start_line(c.blank && !first);
            self.out.push_str(&c.text);
            first = false;
        }
        self.start_line(blank && !first);
    }

    /// The comment after something that ends at `end`, if it's on the same
    /// line.
    fn trailing(&mut self, end: usize) {
        let Some(t) = self.trivia.as_mut().and_then(|t| t.trailing(end)) else {
            return;
        };
        self.out.push(' ');
        let line_start = self.out.rfind('\n').map_or(0, |i| i + 1);
        let column = self.out[line_start..].chars().count();
        self.out.push_str(&t.text);
        for more in t.more {
            let _ = write!(self.out, "\n{:column$}{more}", "");
        }
    }

    /// The comments left before `close`, the end of a block or list, each on
    /// a line of its own. Returns whether there were any.
    fn dangling(&mut self, close: usize, first: bool) -> bool {
        let Some(t) = &mut self.trivia else {
            return false;
        };
        let comments = t.leading(close);
        let any = !comments.is_empty();
        let mut first = first;
        for c in comments {
            self.start_line(c.blank && !first);
            self.out.push_str(&c.text);
            first = false;
        }
        any
    }

    /// Are there comments to print before `pos`?
    fn comments_before(&self, pos: usize) -> bool {
        self.trivia.as_ref().is_some_and(|t| t.any_before(pos))
    }

    /// Where the first `tok` at or after `pos` starts in the source.
    fn find(&self, pos: usize, tok: &Tok) -> usize {
        self.trivia.as_ref().map_or(0, |t| t.find(pos, tok).start)
    }

    /// Where the token before the one at `pos` starts.
    fn before(&self, pos: usize) -> usize {
        self.trivia.as_ref().map_or(0, |t| t.before(pos).start)
    }

    /// Prints `items` between `open` and `close`, separated by commas: on one
    /// line, or one per line if `always_break` or there are comments among
    /// them. `close_pos` is where `close` is in the source, and `rest` goes
    /// after the last item (the `..` in a struct pattern).
    #[allow(clippy::too_many_arguments)]
    fn list<T>(
        &mut self,
        (open, close): (&str, &str),
        close_pos: usize,
        items: &[T],
        span: impl Fn(&T) -> (usize, usize),
        rest: Option<&str>,
        always_break: bool,
        mut item: impl FnMut(&mut Self, &T),
    ) {
        self.out.push_str(open);
        if !always_break && !self.comments_before(close_pos) {
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                item(self, x);
            }
            if let Some(rest) = rest {
                if !items.is_empty() {
                    self.out.push_str(", ");
                }
                self.out.push_str(rest);
            }
            self.out.push_str(close);
            return;
        }
        self.indent += 1;
        for (i, x) in items.iter().enumerate() {
            let (start, end) = span(x);
            self.element(start, i == 0);
            item(self, x);
            self.out.push(',');
            self.trailing(end);
        }
        let empty = items.is_empty() && rest.is_none();
        if let Some(rest) = rest {
            self.element(close_pos, items.is_empty());
            self.out.push_str(rest);
        }
        let comments = self.dangling(close_pos, items.is_empty());
        self.indent -= 1;
        if !empty || comments {
            self.line();
        }
        self.out.push_str(close);
    }

    fn generics(&mut self, params: &[Ident]) {
        if !params.is_empty() {
            let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            let _ = write!(self.out, "<{}>", names.join(", "));
        }
    }

    fn type_decl(&mut self, d: &TypeDecl) {
        let keyword = match d {
            TypeDecl::Struct(_) => "struct",
            TypeDecl::Enum(_) => "enum",
        };
        let _ = write!(self.out, "{keyword} {}", d.name().name);
        self.generics(d.type_params());
        let open = self.find(d.name().span.end, &Tok::LBrace);
        let close = self.find(open, &Tok::RBrace);
        let always_break = match d {
            TypeDecl::Struct(s) => !s.fields.is_empty(),
            TypeDecl::Enum(e) => !e.variants.is_empty(),
        };
        let delims = (" {", "}");
        match d {
            TypeDecl::Struct(s) => self.list(
                delims,
                close,
                &s.fields,
                |f| (f.name.span.start, f.ty.span.end),
                None,
                always_break,
                |p, f| {
                    let _ = write!(p.out, "{}: {}", f.name.name, ty(&f.ty));
                },
            ),
            TypeDecl::Enum(e) => {
                let ends: Vec<usize> = e
                    .variants
                    .iter()
                    .map(|v| match v.fields.last() {
                        Some(t) => self.find(t.span.end, &Tok::RParen) + 1,
                        None => v.name.span.end,
                    })
                    .collect();
                let variants: Vec<(&Variant, usize)> = e.variants.iter().zip(ends).collect();
                self.list(
                    delims,
                    close,
                    &variants,
                    |(v, end)| (v.name.span.start, *end),
                    None,
                    always_break,
                    |p, (v, _)| {
                        p.out.push_str(&v.name.name);
                        if !v.fields.is_empty() {
                            let _ = write!(p.out, "({})", types(&v.fields));
                        }
                    },
                )
            }
        }
    }

    fn fn_decl(&mut self, f: &FnDecl) {
        let _ = write!(self.out, "fn {}", f.name.name);
        self.generics(&f.type_params);
        let open = self.find(f.name.span.end, &Tok::LParen);
        let close = match f.params.last() {
            Some(p) => self.find(p.ty.span.end, &Tok::RParen),
            None => self.find(open, &Tok::RParen),
        };
        self.list(
            ("(", ")"),
            close,
            &f.params,
            |p| (p.name.span.start, p.ty.span.end),
            None,
            false,
            |p, param| {
                let _ = write!(p.out, "{}: {}", param.name.name, ty(&param.ty));
            },
        );
        if !matches!(f.ret.kind, TypeExprKind::Unit) {
            let _ = write!(self.out, " -> {}", ty(&f.ret));
        }
        self.out.push(' ');
        self.expr(&f.body, 0, false);
    }

    /// Prints `e`, in parentheses if it binds less tightly than `min`. If
    /// `no_struct`, `e` is the head of an `if` or `match`, where a struct
    /// literal needs parentheses.
    fn expr(&mut self, e: &Expr, min: u8, no_struct: bool) {
        let own = match &e.kind {
            ExprKind::Binary(op, ..) => prec(*op),
            // A closure's body runs as far right as it can.
            ExprKind::Lambda(_) => 0,
            _ => UNARY,
        };
        let parens = own < min || (no_struct && matches!(e.kind, ExprKind::StructLit(..)));
        if parens {
            self.out.push('(');
        }
        self.expr_inner(e, no_struct && !parens);
        if parens {
            self.out.push(')');
        }
    }

    fn expr_inner(&mut self, e: &Expr, no_struct: bool) {
        match &e.kind {
            ExprKind::Int(n) => {
                let _ = write!(self.out, "{n}");
            }
            ExprKind::Bool(b) => {
                let _ = write!(self.out, "{b}");
            }
            ExprKind::Unit => self.out.push_str("()"),
            ExprKind::Var(name, targs) => {
                self.out.push_str(name);
                self.out.push_str(&turbofish(targs));
            }
            ExprKind::Unary(op, x) => {
                let _ = write!(self.out, "{op}");
                self.expr(x, UNARY, no_struct);
            }
            ExprKind::Binary(op, l, r) => {
                let p = prec(*op);
                self.expr(l, p, no_struct);
                let _ = write!(self.out, " {op} ");
                // Left-associative, and comparisons don't chain: the right
                // side must bind tighter.
                self.expr(r, p + 1, no_struct);
            }
            ExprKind::Call(f, args) => {
                // Something more than a name or a field needs parentheses to
                // be called: `(|x| x)(1)`.
                let min = match f.kind {
                    ExprKind::Var(..) | ExprKind::Field(..) | ExprKind::Call(..) => UNARY,
                    _ => UNARY + 1,
                };
                self.expr(f, min, no_struct);
                self.args(f.span.end, args);
            }
            ExprKind::Field(x, field) => {
                let min = match x.kind {
                    ExprKind::Var(..) | ExprKind::Field(..) | ExprKind::Call(..) => UNARY,
                    _ => UNARY + 1,
                };
                self.expr(x, min, no_struct);
                let _ = write!(self.out, ".{}", field.name);
            }
            ExprKind::If(c, then, els) => {
                self.out.push_str("if ");
                self.expr(c, 0, true);
                self.out.push(' ');
                self.expr(then, 0, false);
                if let Some(els) = els {
                    self.out.push_str(" else ");
                    self.expr(els, 0, false);
                }
            }
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), e.span),
            ExprKind::StructLit(name, targs, inits) => {
                self.out.push_str(&name.name);
                self.out.push_str(&turbofish(targs));
                let close = e.span.end - 1;
                let flat = !self.comments_before(close);
                let delims = match (inits.is_empty(), flat) {
                    (true, true) => (" {", "}"),
                    (false, true) => (" { ", " }"),
                    (_, false) => (" {", "}"),
                };
                self.list(
                    delims,
                    close,
                    inits,
                    |i| (i.name.span.start, i.value.span.end),
                    None,
                    false,
                    |p, init| {
                        p.out.push_str(&init.name.name);
                        let shorthand = matches!(&init.value.kind, ExprKind::Var(v, t) if *v == init.name.name && t.is_empty());
                        if !shorthand {
                            p.out.push_str(": ");
                            p.expr(&init.value, 0, false);
                        }
                    },
                );
            }
            ExprKind::Variant(ty, targs, variant, args) => {
                let _ = write!(
                    self.out,
                    "{}{}::{}",
                    ty.name,
                    turbofish(targs),
                    variant.name
                );
                if !args.is_empty() {
                    self.args(variant.span.end, args);
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.out.push_str("match ");
                self.expr(scrutinee, 0, true);
                if arms.is_empty() && !self.comments_before(e.span.end) {
                    self.out.push_str(" {}");
                    return;
                }
                self.out.push_str(" {");
                self.indent += 1;
                let open = self.find(scrutinee.span.end, &Tok::LBrace);
                self.trailing(open + 1);
                for (i, arm) in arms.iter().enumerate() {
                    self.element(arm.pat.span.start, i == 0);
                    self.out.push_str(&pattern(&arm.pat));
                    self.out.push_str(" => ");
                    self.expr(&arm.body, 0, false);
                    self.out.push(',');
                    self.trailing(arm.body.span.end);
                }
                self.dangling(e.span.end - 1, false);
                self.indent -= 1;
                self.line();
                self.out.push('}');
            }
            ExprKind::Lambda(l) => {
                if l.params.is_empty() {
                    self.out.push_str("||");
                } else {
                    let last = l.params.last().unwrap();
                    let last_end = last.ty.as_ref().map_or(last.name.span.end, |t| t.span.end);
                    let close = self.find(last_end, &Tok::Pipe);
                    self.list(
                        ("|", "|"),
                        close,
                        &l.params,
                        |p| {
                            let end = p.ty.as_ref().map_or(p.name.span.end, |t| t.span.end);
                            (p.name.span.start, end)
                        },
                        None,
                        false,
                        |p, param| {
                            p.out.push_str(&param.name.name);
                            if let Some(t) = &param.ty {
                                let _ = write!(p.out, ": {}", ty(t));
                            }
                        },
                    );
                }
                self.out.push(' ');
                match &l.ret {
                    Some(t) => {
                        let _ = write!(self.out, "-> {} ", ty(t));
                        self.expr(&l.body, 0, false);
                    }
                    None => self.expr(&l.body, 0, no_struct),
                }
            }
        }
    }

    /// Call arguments, after the callee that ends at `after`.
    fn args(&mut self, after: usize, args: &[Expr]) {
        let close = match args.last() {
            Some(a) => self.find(a.span.end, &Tok::RParen),
            None => self.find(after, &Tok::RParen),
        };
        self.list(
            ("(", ")"),
            close,
            args,
            |a| (a.span.start, a.span.end),
            None,
            false,
            |p, a| p.expr(a, 0, false),
        );
    }

    fn block(&mut self, stmts: &[Stmt], tail: Option<&Expr>, span: crate::error::Span) {
        let close = span.end - 1;
        if stmts.is_empty() && tail.is_none() && !self.comments_before(close) {
            self.out.push_str("{}");
            return;
        }
        self.out.push('{');
        self.indent += 1;
        self.trailing(span.start + 1);
        for (i, stmt) in stmts.iter().enumerate() {
            let (start, end) = match stmt {
                Stmt::Let { pat, init, .. } => (self.before(pat.span.start), init.span.end),
                Stmt::Expr(e) | Stmt::BlockLike(e) => (e.span.start, e.span.end),
            };
            self.element(start, i == 0);
            match stmt {
                Stmt::Let { pat, ty: t, init } => {
                    let _ = write!(self.out, "let {}", pattern(pat));
                    if let Some(t) = t {
                        let _ = write!(self.out, ": {}", ty(t));
                    }
                    self.out.push_str(" = ");
                    self.expr(init, 0, false);
                    self.out.push(';');
                }
                Stmt::Expr(e) => {
                    self.statement(e);
                    self.out.push(';');
                }
                Stmt::BlockLike(e) => self.expr(e, 0, false),
            }
            self.trailing(end);
        }
        if let Some(tail) = tail {
            self.element(tail.span.start, stmts.is_empty());
            self.statement(tail);
            self.trailing(tail.span.end);
        }
        self.dangling(close, stmts.is_empty() && tail.is_none());
        self.indent -= 1;
        self.line();
        self.out.push('}');
    }

    /// `e` at the start of a statement, where an `if`, `match` or block
    /// ends the statement.
    fn statement(&mut self, e: &Expr) {
        let min = if starts_block_like(e) { UNARY + 1 } else { 0 };
        self.expr(e, min, false);
    }
}

/// `::<int, bool>`, or nothing if there are no type arguments.
fn turbofish(targs: &[TypeExpr]) -> String {
    match targs {
        [] => String::new(),
        targs => format!("::<{}>", types(targs)),
    }
}

fn types(ts: &[TypeExpr]) -> String {
    let ts: Vec<String> = ts.iter().map(ty).collect();
    ts.join(", ")
}

fn ty(t: &TypeExpr) -> String {
    match &t.kind {
        TypeExprKind::Named(name, args) if args.is_empty() => name.clone(),
        TypeExprKind::Named(name, args) => format!("{name}<{}>", types(args)),
        TypeExprKind::Unit => "()".into(),
        TypeExprKind::Fn(params, ret) => match ret.kind {
            TypeExprKind::Unit => format!("fn({})", types(params)),
            _ => format!("fn({}) -> {}", types(params), ty(ret)),
        },
    }
}

fn pattern(p: &Pattern) -> String {
    match &p.kind {
        PatternKind::Wild => "_".into(),
        PatternKind::Bind(name) => name.clone(),
        PatternKind::Int(n) => n.to_string(),
        PatternKind::Range(lo, hi, inclusive) => range(*lo, *hi, *inclusive),
        PatternKind::Bool(b) => b.to_string(),
        PatternKind::Unit => "()".into(),
        PatternKind::Variant(ty, targs, variant, args) => {
            let head = format!("{}{}::{}", ty.name, turbofish(targs), variant.name);
            if args.is_empty() {
                return head;
            }
            let args: Vec<String> = args.iter().map(pattern).collect();
            format!("{head}({})", args.join(", "))
        }
        PatternKind::Struct(ty, targs, fields, rest) => {
            let ty = format!("{}{}", ty.name, turbofish(targs));
            let mut items: Vec<String> = fields
                .iter()
                .map(|f| match &f.pat.kind {
                    PatternKind::Bind(b) if *b == f.name.name => b.clone(),
                    _ => format!("{}: {}", f.name.name, pattern(&f.pat)),
                })
                .collect();
            if *rest {
                items.push("..".into());
            }
            match items.is_empty() {
                true => format!("{ty} {{}}"),
                false => format!("{ty} {{ {} }}", items.join(", ")),
            }
        }
        PatternKind::Or(alts) => {
            let alts: Vec<String> = alts.iter().map(pattern).collect();
            alts.join(" | ")
        }
    }
}

/// A range pattern as written. Unlike `exhaust::show_range`, which tidies
/// ranges up for messages, this keeps `0..1` from turning into `0`.
fn range(lo: i64, hi: i64, inclusive: bool) -> String {
    let lo = match lo {
        i64::MIN => String::new(),
        lo => lo.to_string(),
    };
    match hi {
        _ if inclusive => format!("{lo}..={hi}"),
        i64::MAX => format!("{lo}.."),
        hi => format!("{lo}..{}", hi + 1),
    }
}
