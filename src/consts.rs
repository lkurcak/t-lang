//! Working out const items' values while compiling (see docs/36-consts.md).
//!
//! Once a program checks, each const's initializer is run by the
//! interpreter, as a program of its own: monomorphised from the const
//! alone (`mono::rooted`), with only what it needs, on a 32-bit machine and
//! on a 64-bit one. A const has one value on every machine, so the two must
//! agree (lemma C2); a panic, on either, is an error, so a use of the const
//! never panics (C1); and so is running for too long, as nothing says a
//! loop ends. The value is plain data (C4), which the const's MIR body is
//! then replaced by: what it builds, so neither the interpreter nor the C
//! backend runs the initializer again.
//!
//! What a const needs, another const's value, as a length or by a call, is
//! worked out first. A const that needs itself is an error (C7): a cycle
//! has no first.

use crate::ast::Program;
use crate::check::Info;
use crate::check::Known;
use crate::error::{Error, Span};
use crate::interp::Failed;
use crate::mir::{
    self, Agg, Bb, Block, Body, Key, Local, LocalDecl, LocalKind, Operand, Place, Rvalue,
    Statement, StmtKind, Term,
};
use crate::types::{Adts, Type, struct_fields, variant_fields};
use std::fmt;
use std::sync::Arc;

/// How many basic blocks working out one const's value may run.
pub(crate) const BUDGET: u64 = 1_000_000;

/// A const's value: plain data, as the interpreter gives it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Int(i128),
    Bool(bool),
    Char(char),
    Unit,
    /// What a `&str` or a `&[u8]` points to.
    Bytes(Vec<u8>),
    /// A struct's fields, in declaration order.
    Struct(Vec<Value>),
    /// Variant `.0` of an enum, and its fields.
    Variant(usize, Vec<Value>),
    Array(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let list = |f: &mut fmt::Formatter, open: &str, vs: &[Value], close: &str| {
            let vs: Vec<String> = vs.iter().map(Value::to_string).collect();
            write!(f, "{open}{}{close}", vs.join(", "))
        };
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Char(c) => write!(f, "{c:?}"),
            Value::Unit => f.write_str("()"),
            Value::Bytes(b) => match std::str::from_utf8(b) {
                Ok(s) => write!(f, "{s:?}"),
                Err(_) => write!(f, "{b:?}"),
            },
            Value::Struct(vs) => list(f, "{ ", vs, " }"),
            Value::Variant(i, vs) => list(f, &format!("variant {i}("), vs, ")"),
            Value::Array(vs) => list(f, "[", vs, "]"),
        }
    }
}

/// A const to work out the value of.
pub(crate) struct Const {
    pub(crate) symbol: String,
    /// Its name, as written, and where it is.
    pub(crate) name: String,
    pub(crate) span: Span,
    /// Its type, which is plain data.
    pub(crate) ty: Type,
    /// What the refinement logic took it to be, if anything: the value
    /// must be that (lemma C5).
    pub(crate) known: Option<Known>,
}

/// Works out the value of each of `consts`, of `prog`, which checked, as
/// far as `info` says, whose structs and enums are `adts`: into
/// `Info::const_values`, and their MIR bodies, which then give them.
pub(crate) fn evaluate(
    prog: &Program,
    info: &mut Info,
    adts: &Adts,
    consts: &[Const],
) -> Result<(), Error> {
    if consts
        .iter()
        .all(|c| info.const_values.contains_key(&c.symbol))
    {
        return Ok(());
    }
    let lifted = crate::lift::lift_program(prog, info);
    let mut cx = Cx {
        prog: &lifted,
        adts,
        consts,
        working: Vec::new(),
    };
    for c in consts {
        cx.value(info, &c.symbol)?;
    }
    Ok(())
}

struct Cx<'a> {
    prog: &'a Program,
    adts: &'a Adts,
    consts: &'a [Const],
    /// The consts being worked out, each needed by the one before it.
    working: Vec<String>,
}

impl Cx<'_> {
    fn decl(&self, symbol: &str) -> &Const {
        (self.consts.iter())
            .find(|c| c.symbol == symbol)
            .expect("every const is in `consts`")
    }

    /// Works out const `symbol`'s value, and first those it needs.
    fn value(&mut self, info: &mut Info, symbol: &str) -> Result<(), Error> {
        if info.const_values.contains_key(symbol) {
            return Ok(());
        }
        self.working.push(symbol.to_string());
        let exe = loop {
            let (exe, missing) = crate::mono::rooted(self.prog, info, symbol);
            if missing.is_empty() {
                break exe;
            }
            for m in missing {
                if self.working.contains(&m) {
                    let c = self.decl(&m);
                    return Err(Error::ty(
                        c.span,
                        format!(
                            "the value of const `{}` depends on itself: working it out needs it",
                            c.name
                        ),
                    ));
                }
                self.value(info, &m)?;
            }
        };
        self.working.pop();
        let c = self.decl(symbol);
        let at = |bits: u32| match crate::interp::evaluate(&exe, symbol, bits, BUDGET) {
            Ok(v) => Ok(v),
            Err(Failed::Budget) => Err(Error::ty(
                c.span,
                format!(
                    "evaluating `{}` took more than {BUDGET} steps; if it's in a loop that \
                     never ends, that's why",
                    c.name
                ),
            )),
            Err(Failed::Aborted(e)) => {
                let machine = match bits {
                    64 => String::new(),
                    _ => format!(" on a {bits}-bit machine"),
                };
                // The interpreter's message for a panic says it's one.
                let what = e.message.strip_prefix("panicked: ").unwrap_or(&e.message);
                let message = format!("evaluating `{}`{machine} panicked: {what}", c.name);
                let error = Error::ty(c.span, message);
                Err(match e.span == Span::new(0, 0) {
                    true => error,
                    false => error.with_note(e.span, "it panicked here"),
                })
            }
        };
        let wide = at(64)?;
        let narrow = at(32)?;
        if wide != narrow {
            return Err(Error::ty(
                c.span,
                format!(
                    "`{}` is {wide} on a 64-bit machine but {narrow} on a 32-bit one: a const \
                     has one value on every machine",
                    c.name
                ),
            ));
        }
        let agrees = match (c.known, &wide) {
            (None, _) => true,
            (Some(Known::Int(k)), Value::Int(n)) => k == *n,
            // The logic takes a `char` to be its scalar value.
            (Some(Known::Int(k)), Value::Char(c)) => k == i128::from(u32::from(*c)),
            (Some(Known::Bool(k)), Value::Bool(b)) => k == *b,
            _ => false,
        };
        if !agrees {
            unreachable!(
                "type soundness violated: the refinement logic took const `{}` to be {:?}, but \
                 it's {wide}. This is a bug in the type checker.",
                c.name, c.known
            );
        }
        let body = value_body(&wide, &c.ty, self.adts, c.span);
        info.mir.insert(Key::Fn(symbol.to_string()), Arc::new(body));
        info.const_values.insert(symbol.to_string(), wide);
        Ok(())
    }
}

/// The MIR body of a const whose value is `v`, of type `ty`: one block,
/// which builds it, its parts first, each into a temporary of its own.
fn value_body(v: &Value, ty: &Type, adts: &Adts, span: Span) -> Body {
    let decl = |ty: &Type, kind| LocalDecl {
        ty: ty.erased(),
        kind,
        mutable: false,
        name: None,
        span,
        mut_span: None,
        by_ref: None,
        deferred: false,
        of_static: None,
    };
    let mut b = Builder {
        adts,
        span,
        locals: vec![decl(ty, LocalKind::Return)],
        stmts: Vec::new(),
    };
    let value = b.rvalue(v, ty, &decl);
    let ret = Place::local(crate::mir::RETURN);
    b.assign(ret, value);
    for l in 1..b.locals.len() {
        b.stmts.push(Statement {
            kind: StmtKind::Dead(Local(l)),
            span,
        });
    }
    Body {
        locals: b.locals,
        blocks: vec![Block {
            stmts: b.stmts,
            term: Some(Term::Return),
        }],
        params: 0,
        env: Vec::new(),
        changed_by_closures: Vec::new(),
        unique_by_closures: Vec::new(),
        heads: vec![Bb(0)],
        drop_uses: Default::default(),
        refinements: Vec::new(),
        unsafe_blocks: Vec::new(),
    }
}

struct Builder<'a> {
    adts: &'a Adts,
    span: Span,
    locals: Vec<LocalDecl>,
    stmts: Vec<Statement>,
}

impl Builder<'_> {
    fn assign(&mut self, place: Place, value: Rvalue) {
        self.stmts.push(Statement {
            kind: StmtKind::Assign {
                place,
                value,
                declare: false,
            },
            span: self.span,
        });
    }

    /// What makes `v`, of type `ty`.
    fn rvalue(
        &mut self,
        v: &Value,
        ty: &Type,
        decl: &dyn Fn(&Type, LocalKind) -> LocalDecl,
    ) -> Rvalue {
        let agg = |b: &mut Self, agg, parts: &[Value], tys: Vec<Type>| {
            let ops = (parts.iter().zip(&tys))
                .map(|(p, t)| b.operand(p, t, decl))
                .collect();
            Rvalue::Aggregate(agg, ty.erased(), ops)
        };
        match (v, ty) {
            (Value::Struct(fs), _) => {
                let tys = (struct_fields(self.adts, ty).into_iter().flatten())
                    .map(|(_, t)| t)
                    .collect();
                agg(self, Agg::Struct, fs, tys)
            }
            (Value::Variant(i, fs), _) => agg(
                self,
                Agg::Variant(*i),
                fs,
                variant_fields(self.adts, ty, *i),
            ),
            (Value::Array(vs), Type::Array(elem, _)) => {
                agg(self, Agg::Array, vs, vec![(**elem).clone(); vs.len()])
            }
            _ => Rvalue::Use(self.operand(v, ty, decl)),
        }
    }

    /// `v`, of type `ty`, as an operand: a constant, or a temporary it's
    /// made in, moved out of.
    fn operand(
        &mut self,
        v: &Value,
        ty: &Type,
        decl: &dyn Fn(&Type, LocalKind) -> LocalDecl,
    ) -> Operand {
        let c = match (v, ty) {
            (Value::Int(n), Type::Int(t)) => mir::Const::Int(*n, Type::Int(*t)),
            (Value::Bool(b), _) => mir::Const::Bool(*b),
            (Value::Char(c), _) => mir::Const::Char(*c),
            (Value::Unit, _) => mir::Const::Unit,
            (Value::Bytes(b), Type::Ref(_, _, to)) if **to == Type::Str => {
                mir::Const::Str(String::from_utf8(b.clone()).expect("a `str` is UTF-8"))
            }
            (Value::Bytes(b), _) => mir::Const::Bytes(b.clone()),
            _ => {
                let tmp = Local(self.locals.len());
                self.locals.push(decl(ty, LocalKind::Temp));
                let value = self.rvalue(v, ty, decl);
                self.assign(Place::local(tmp), value);
                return Operand::Move(Place::local(tmp));
            }
        };
        Operand::Const(c)
    }
}
