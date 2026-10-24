//! What the checker learns about a function body that the borrow checker
//! needs and the AST doesn't say: the type of every expression, what each
//! variable refers to, how each pattern binds, and where references were
//! looked through. Recorded as the body is checked, by the id of the node
//! in the AST it's about (`NodeId`).
//!
//! The types may still have type variables in them while the body is being
//! checked; the borrow checker runs once they're all solved, and asks the
//! checker for their solutions.

use crate::ast::{Capture, NodeId};
use crate::error::Span;
use crate::traits::FnKind;
use crate::types::Type;
use std::collections::HashMap;

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
    /// For each field expression, how many references and boxes it looked
    /// through to find the struct (`r.x` for `r: &P` looks through one),
    /// and which field it is.
    pub(crate) fields: HashMap<NodeId, (usize, usize)>,
    /// The variables each closure captures, by id.
    pub(crate) captures: HashMap<NodeId, Vec<NodeId>>,
    /// How each closure that captures captures each of those, once worked
    /// out (`Checker::settle_closures`), with its type's symbol.
    pub(crate) modes: HashMap<NodeId, Vec<(NodeId, Capture)>>,
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
