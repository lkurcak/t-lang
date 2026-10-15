//! What the checker learns about a function body that the borrow checker
//! needs and the AST doesn't say: the type of every expression, what each
//! variable refers to, how each pattern binds, and where references were
//! looked through. Recorded as the body is checked, by the address of the
//! node in the AST it's about, so it's only good while that AST is.
//!
//! The types may still have type variables in them while the body is being
//! checked; the borrow checker runs once they're all solved, and asks the
//! checker for their solutions.

use crate::ast::{Expr, Lambda, Pattern};
use crate::error::Span;
use crate::types::Type;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Typed {
    /// The type of every expression.
    pub(crate) exprs: HashMap<*const Expr, Type>,
    /// What each variable names.
    pub(crate) vars: HashMap<*const Expr, Res>,
    /// Every variable, by its *id*: where its name is first bound.
    pub(crate) binds: HashMap<Span, Bind>,
    /// Which variable is bound by the name at each binding site, by id. A
    /// variable of an or-pattern has a site in each alternative.
    pub(crate) sites: HashMap<Span, Span>,
    /// How many references each pattern looked through before matching
    /// (*match ergonomics*): `Option::Some(x)` matching a
    /// `&Option<int>` looks through one, and binds `x` to a `&int`.
    pub(crate) peels: HashMap<*const Pattern, usize>,
    /// References used as others (*coerced*): a `&mut T` where a `&T` is
    /// wanted, or a `&Box<T>` or `&&T` where a `&T` is. With how many more
    /// `*`s it takes to get to the `T`, and whether the result is `&mut`.
    pub(crate) coerced: HashMap<*const Expr, (usize, bool)>,
    /// For each field expression, how many references and boxes it looked
    /// through to find the struct (`r.x` for `r: &P` looks through one),
    /// and which field it is.
    pub(crate) fields: HashMap<*const Expr, (usize, usize)>,
    /// The variables each closure captures, by id.
    pub(crate) captures: HashMap<*const Lambda, Vec<Span>>,
}

/// What a variable names.
#[derive(Debug, Clone)]
pub(crate) enum Res {
    /// A local variable, by id.
    Local(Span),
    /// A function, by symbol, with its type arguments.
    Fn(String, Vec<Type>),
    Builtin,
}

/// A local variable.
#[derive(Debug, Clone)]
pub(crate) struct Bind {
    pub(crate) name: String,
    /// The type of the variable: for one bound by reference, a reference.
    pub(crate) ty: Type,
    /// Declared `mut`?
    pub(crate) mutable: bool,
    /// From the `mut` to the name, if it's `mut`.
    pub(crate) mut_span: Option<Span>,
    /// Bound by reference (match ergonomics), and if so, `&mut`?
    pub(crate) by_ref: Option<bool>,
}
