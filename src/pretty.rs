//! Prints a program back out as source, for `t mono`. A program without
//! generics prints as T that parses back to the same program; the names
//! monomorphisation gives its copies, like `id::<int>`, don't parse.

use crate::ast::*;
use crate::exhaust::show_range;
use std::fmt::{self, Write};

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut p = Printer::default();
        let mut first = true;
        for d in &self.types {
            if !std::mem::take(&mut first) {
                p.out.push('\n');
            }
            p.type_decl(d);
        }
        for func in &self.fns {
            if !std::mem::take(&mut first) {
                p.out.push('\n');
            }
            p.fn_decl(func);
        }
        f.write_str(&p.out)
    }
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
struct Printer {
    out: String,
    indent: usize,
}

impl Printer {
    fn line(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }

    fn generics(&mut self, params: &[Ident]) {
        if !params.is_empty() {
            let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            let _ = write!(self.out, "<{}>", names.join(", "));
        }
    }

    fn type_decl(&mut self, d: &TypeDecl) {
        let (keyword, items): (&str, Vec<String>) = match d {
            TypeDecl::Struct(s) => (
                "struct",
                s.fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name.name, ty(&f.ty)))
                    .collect(),
            ),
            TypeDecl::Enum(e) => (
                "enum",
                e.variants
                    .iter()
                    .map(|v| match v.fields.is_empty() {
                        true => v.name.name.clone(),
                        false => format!("{}({})", v.name.name, types(&v.fields)),
                    })
                    .collect(),
            ),
        };
        let _ = write!(self.out, "{keyword} {}", d.name().name);
        self.generics(d.type_params());
        if items.is_empty() {
            self.out.push_str(" {}\n");
            return;
        }
        self.out.push_str(" {");
        for item in items {
            let _ = write!(self.out, "\n    {item},");
        }
        self.out.push_str("\n}\n");
    }

    fn fn_decl(&mut self, f: &FnDecl) {
        let _ = write!(self.out, "fn {}", f.name.name);
        self.generics(&f.type_params);
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name.name, ty(&p.ty)))
            .collect();
        let _ = write!(self.out, "({})", params.join(", "));
        if !matches!(f.ret.kind, TypeExprKind::Unit) {
            let _ = write!(self.out, " -> {}", ty(&f.ret));
        }
        self.out.push(' ');
        self.expr(&f.body, 0, false);
        self.out.push('\n');
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
            ExprKind::Var(name) => self.out.push_str(name),
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
                    ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Call(..) => UNARY,
                    _ => UNARY + 1,
                };
                self.expr(f, min, no_struct);
                self.args(args);
            }
            ExprKind::Field(x, field) => {
                let min = match x.kind {
                    ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Call(..) => UNARY,
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
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref()),
            ExprKind::StructLit(name, inits) => {
                self.out.push_str(&name.name);
                if inits.is_empty() {
                    self.out.push_str(" {}");
                    return;
                }
                self.out.push_str(" { ");
                for (i, init) in inits.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(&init.name.name);
                    let shorthand =
                        matches!(&init.value.kind, ExprKind::Var(v) if *v == init.name.name);
                    if !shorthand {
                        self.out.push_str(": ");
                        self.expr(&init.value, 0, false);
                    }
                }
                self.out.push_str(" }");
            }
            ExprKind::Variant(ty, variant, args) => {
                let _ = write!(self.out, "{}::{}", ty.name, variant.name);
                if !args.is_empty() {
                    self.args(args);
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.out.push_str("match ");
                self.expr(scrutinee, 0, true);
                if arms.is_empty() {
                    self.out.push_str(" {}");
                    return;
                }
                self.out.push_str(" {");
                self.indent += 1;
                for arm in arms {
                    self.line();
                    self.out.push_str(&pattern(&arm.pat));
                    self.out.push_str(" => ");
                    self.expr(&arm.body, 0, false);
                    self.out.push(',');
                }
                self.indent -= 1;
                self.line();
                self.out.push('}');
            }
            ExprKind::Lambda(l) => {
                if l.params.is_empty() {
                    self.out.push_str("||");
                } else {
                    let params: Vec<String> = l
                        .params
                        .iter()
                        .map(|p| match &p.ty {
                            Some(t) => format!("{}: {}", p.name.name, ty(t)),
                            None => p.name.name.clone(),
                        })
                        .collect();
                    let _ = write!(self.out, "|{}|", params.join(", "));
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

    fn args(&mut self, args: &[Expr]) {
        self.out.push('(');
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.expr(a, 0, false);
        }
        self.out.push(')');
    }

    fn block(&mut self, stmts: &[Stmt], tail: Option<&Expr>) {
        if stmts.is_empty() && tail.is_none() {
            self.out.push_str("{}");
            return;
        }
        self.out.push('{');
        self.indent += 1;
        for stmt in stmts {
            self.line();
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
        }
        if let Some(tail) = tail {
            self.line();
            self.statement(tail);
        }
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
        PatternKind::Range(lo, hi, inclusive) => show_range(*lo, *hi, *inclusive),
        PatternKind::Bool(b) => b.to_string(),
        PatternKind::Unit => "()".into(),
        PatternKind::Variant(ty, variant, args) => {
            let head = format!("{}::{}", ty.name, variant.name);
            if args.is_empty() {
                return head;
            }
            let args: Vec<String> = args.iter().map(pattern).collect();
            format!("{head}({})", args.join(", "))
        }
        PatternKind::Struct(ty, fields, rest) => {
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
                true => format!("{} {{}}", ty.name),
                false => format!("{} {{ {} }}", ty.name, items.join(", ")),
            }
        }
        PatternKind::Or(alts) => {
            let alts: Vec<String> = alts.iter().map(pattern).collect();
            alts.join(" | ")
        }
    }
}
