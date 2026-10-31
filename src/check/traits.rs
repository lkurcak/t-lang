//! Collecting traits and impls, and checking the rules that keep trait
//! resolution simple (see `traits.rs`): every impl makes sense, no two
//! overlap (*coherence*), and proving that a type implements a trait always
//! ends (*termination*). Also proving obligations, and finding the method a
//! call `x.m()` or a path `T::m` names.

//!
//! - `collect.rs`: traits and impls, and the rules each impl header keeps
//! - `bounds.rs`: bounds and `where` clauses, and projections in them
//! - `derive.rs`: `#[derive(..)]`
//! - `impls.rs`: impls against their traits, the orphan rule, coherence
//! - `termination.rs`: resolution and normalising always end
//! - `prove.rs`: obligations and well-formedness
//! - `methods.rs`: method lookup
//! - `dicts.rs`: which impls a use picks, for polymorphic recursion

mod bounds;
mod collect;
mod derive;
mod dicts;
mod impls;
mod methods;
mod prove;
mod termination;

pub(super) use bounds::has_proj;
pub(super) use collect::Outer;
pub(super) use dicts::{Site, dict_names};
pub(super) use methods::{Found, trait_method_symbol};
pub(super) use prove::{Obligation, Why};

use crate::types::{AdtKind, Type};

/// Each field of struct or enum `d`, of kind `kind`, as "field `x`" or
/// "variant `V`", with its type.
pub(super) fn fields(kind: &AdtKind) -> Vec<(String, Type)> {
    match kind {
        AdtKind::Struct(fields) => fields
            .iter()
            .map(|(f, t)| (format!("field `{f}`"), t.clone()))
            .collect(),
        AdtKind::Enum(variants) => variants
            .iter()
            .flat_map(|(v, ts)| {
                ts.iter()
                    .map(move |t| (format!("variant `{v}`"), t.clone()))
            })
            .collect(),
    }
}

/// How many times `t` mentions type parameter `p`.
fn occurrences(t: &Type, p: &str) -> usize {
    match t {
        Type::Param(q) if q == p => 1,
        t => t.parts().map(|t| occurrences(t, p)).sum(),
    }
}

/// Does `t` mention type parameter `p`?
fn mentions(t: &Type, p: &str) -> bool {
    matches!(t, Type::Param(q) if q == p) || t.parts().any(|t| mentions(t, p))
}

/// Calls `f` with the type and trait of each projection in `t`.
fn each_proj<'t>(t: &'t Type, f: &mut impl FnMut(&'t Type, &'t str)) {
    if let Type::Proj(s, tr, _) = t {
        f(s, tr);
    }
    t.parts().for_each(|p| each_proj(p, f));
}
