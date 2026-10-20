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
//! - `methods.rs`: method lookup, and `dispatch`

mod bounds;
mod collect;
mod derive;
mod impls;
mod methods;
mod prove;
mod termination;

pub(super) use bounds::has_proj;
pub(super) use collect::Outer;
pub(super) use methods::{Found, trait_method_symbol};
pub(super) use prove::{Obligation, Why};

use crate::types::{AdtKind, Type};

/// A stand-in for a projection, for `dispatch`: its name, the projection,
/// and which parameters of the function it's in the projection mentions.
type Hidden = (String, Type, Vec<usize>);

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

/// `t` with each projection in it a stand-in type parameter, `?p0`, `?p1`
/// and so on, added to `hidden` with the projection and the indices of the
/// parameters of `params` it mentions (see `dispatch`).
fn hide_projections(t: &Type, params: &[String], hidden: &mut Vec<Hidden>) -> Type {
    match t {
        Type::Proj(..) => {
            let name = format!("?p{}", hidden.len());
            let mentions = (0..params.len())
                .filter(|&i| mentions(t, &params[i]))
                .collect();
            hidden.push((name.clone(), t.clone(), mentions));
            Type::Param(name)
        }
        t => t.map(|p| hide_projections(p, params, hidden)),
    }
}

/// `t` with the stand-ins of `hide_projections` back as the projections
/// they stand for.
fn shown_projections(t: &Type, hidden: &[Hidden]) -> Type {
    match t {
        Type::Param(p) => match hidden.iter().find(|x| x.0 == *p) {
            Some((_, proj, _)) => proj.clone(),
            None => t.clone(),
        },
        t => t.map(|p| shown_projections(p, hidden)),
    }
}
