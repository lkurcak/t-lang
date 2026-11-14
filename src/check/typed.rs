//! What the checker learns about a function body that the borrow checker
//! needs and the AST doesn't say: the type of every expression, what each
//! variable refers to, how each pattern binds, and where references were
//! looked through. Recorded as the body is checked, by the id of the node
//! in the AST it's about (`NodeId`).
//!
//! The types may still have type variables in them while the body is being
//! checked; the borrow checker runs once they're all solved, and asks the
//! checker for their solutions. Those of effects may still be variables
//! then, which it has no use for (see `Checker::keep_mir`).

use crate::ast::{Capture, Expr, ExprKind, NodeId, UnOp};
use crate::error::Span;
use crate::mir::Proj;
use crate::traits::FnKind;
use crate::types::Type;
use std::collections::HashMap;

/// A place a closure captures, and how (docs/76-widths-and-captures.md): a
/// variable around it, and the fields and `*`s from the variable to the
/// place, as the MIR's place for it has them (`Typed::place_path`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Captured {
    /// The variable, by id.
    pub(crate) var: NodeId,
    pub(crate) path: Vec<Proj>,
    /// How it's written: `p`, `p.x`, `r.x` for `(*r).x`, or `*r`.
    pub(crate) name: String,
    /// The place's type.
    pub(crate) ty: Type,
    pub(crate) mode: Capture,
}

#[derive(Clone, Default)]
pub(crate) struct Typed {
    /// The type of every expression.
    pub(crate) exprs: HashMap<NodeId, Type>,
    /// What each variable names, and each path `T::m` that names a method
    /// rather than a variant.
    pub(crate) vars: HashMap<NodeId, Res>,
    /// Every variable, by its *id*: the node where its name is first bound,
    /// a binding pattern or a parameter's name.
    pub(crate) binds: HashMap<NodeId, Bind>,
    /// Which variable is bound by the name at each binding site, by id. A
    /// variable of an or-pattern has a site in each alternative.
    pub(crate) sites: HashMap<NodeId, NodeId>,
    /// How many references each pattern looked through before matching
    /// (*match ergonomics*): `Option::Some(x)` matching a
    /// `&Option<i64>` looks through one, and binds `x` to a `&i64`.
    pub(crate) peels: HashMap<NodeId, usize>,
    /// The struct each struct pattern matches, by the pattern's id.
    pub(crate) struct_pats: HashMap<NodeId, Type>,
    /// References used as others (*coerced*): a `&mut T` where a `&T` is
    /// wanted, or a `&Box<T>` or `&&T` where a `&T` is. With how many more
    /// `*`s it takes to get to the `T`, and whether the result is `&mut`.
    pub(crate) coerced: HashMap<NodeId, (usize, bool)>,
    /// Of those, the references to arrays used as references to slices
    /// (*unsizing*), `&[T; N]` as `&[T]`: what's coerced is then given to
    /// the array's `as_slice`, or `as_mut_slice`, by symbol, with its type
    /// arguments, which says the slice is as long as the array (lemma C1
    /// of docs/73-for-over-collections.md).
    pub(crate) unsized_: HashMap<NodeId, (String, Vec<Type>)>,
    /// Closures used as functions (*reified*: see `Checker::reify`), by
    /// the node of what's used, with the closure's type's symbol.
    pub(crate) reified: HashMap<NodeId, String>,
    /// For each field expression, how many references and boxes it looked
    /// through to find the struct (`r.x` for `r: &P` looks through one),
    /// and which field it is.
    pub(crate) fields: HashMap<NodeId, (usize, usize)>,
    /// The variables each closure captures, by id.
    pub(crate) captures: HashMap<NodeId, Vec<NodeId>>,
    /// The places each closure that captures captures, and how, once
    /// worked out (`Checker::settle_closures`): its fields, in order.
    pub(crate) modes: HashMap<NodeId, Vec<Captured>>,
    /// For each call of what isn't a function type, how it gets at what it
    /// calls: through how many `*`s, and by which `Fn` trait. Through any,
    /// it borrows what's at the end, `&` for `Fn` and `&mut` for `FnMut`;
    /// or else, it borrows it that way, or for `FnOnce`, takes it.
    pub(crate) calls: HashMap<NodeId, (usize, FnKind)>,
    /// For each method call, the method it calls, and how its receiver is
    /// got from what's before the `.`: through how many `*`s, and then
    /// maybe a `&` (`Some(false)`) or `&mut` (`Some(true)`).
    pub(crate) methods: HashMap<NodeId, (Res, usize, Option<bool>)>,
}

/// What a variable names.
#[derive(Debug, Clone)]
pub(crate) enum Res {
    /// A local variable, by id.
    Local(NodeId),
    /// A function, by symbol, with its type arguments.
    Fn(String, Vec<Type>),
    /// A const parameter of the function, by name: a `usize` the caller
    /// picks, the same for the whole call.
    Const(String),
    /// A const item, by symbol: its value (see `check/consts.rs`).
    Item(String),
    /// A static item, by symbol: the place it is, which a `&'static` to it
    /// reaches (see docs/38-statics.md).
    Static(String),
}

/// A local variable.
#[derive(Debug, Clone)]
pub(crate) struct Bind {
    pub(crate) name: String,
    /// Where its name is first bound.
    pub(crate) span: Span,
    /// The type of the variable: for one bound by reference, a reference.
    pub(crate) ty: Type,
    /// Declared `mut`?
    pub(crate) mutable: bool,
    /// From the `mut` to the name, if it's `mut`.
    pub(crate) mut_span: Option<Span>,
    /// Bound by reference (match ergonomics), and if so, `&mut`?
    pub(crate) by_ref: Option<bool>,
}

impl Typed {
    /// The variable place expression `e` is in, and the fields and `*`s
    /// from it to `e`, as `Builder::place` makes them: a field through
    /// references or boxes looks through each first. `None` if `e` isn't a
    /// variable, or a field of one, or what one points to.
    pub(crate) fn place_path(&self, e: &Expr) -> Option<(NodeId, Vec<Proj>)> {
        match &e.kind {
            ExprKind::Var(..) => match self.vars.get(&e.id) {
                Some(Res::Local(id)) => Some((*id, Vec::new())),
                _ => None,
            },
            ExprKind::Field(x, _) => {
                let &(derefs, i) = self.fields.get(&e.id)?;
                let (id, mut path) = self.place_path(x)?;
                path.extend(std::iter::repeat_n(Proj::Deref, derefs));
                path.push(Proj::Field(i));
                Some((id, path))
            }
            ExprKind::Unary(UnOp::Deref, x) => {
                let (id, mut path) = self.place_path(x)?;
                path.push(Proj::Deref);
                Some((id, path))
            }
            _ => None,
        }
    }
}
