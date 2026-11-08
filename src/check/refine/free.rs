//! Free theorems (docs/55-free-theorems.md): which type parameters of a
//! function it can't make a value of, so that what it gives of one is one
//! of the values of it it was given.
//!
//! A function generic in `T` can do with a `T` only what its bounds let
//! it. A marker trait (`Copy`, `Discard`, `Send`, `Sync`) lets it copy one,
//! drop one, or send one to another thread, never make one; nor can it make
//! one by `match`ing on `T` (there's no such thing), nor by an intrinsic:
//! each of those generic in a `T` with only markers moves, copies, drops
//! or stores the values it's given, and the one that reads a value from an
//! address, `Volatile::at`, needs `T: Atom`. So by Reynolds' abstraction
//! theorem, a value of `T` in the result is a value of `T` that was in an
//! argument (lemma P1). That's only something the checker can say of a
//! value it knows the parts of: where `T` is, in each parameter and the
//! result, is a *way* there, `T` itself or a variant's field of an enum
//! (lemma E1 of docs/53-refined-type-arguments.md), not behind a reference
//! (whose target may have changed), a struct (whose fields may be
//! private) or anything else.

use crate::ast::Free;
use crate::traits::{COPY, DISCARD, SEND, SYNC};
use crate::types::{AdtKind, Adts, Bound, Scheme, Type, variant_fields};

/// The traits that make no values of what they bound: they have no
/// methods, or only `Discard`'s, which uses its value up.
const MARKERS: [&str; 4] = [COPY, DISCARD, SEND, SYNC];

/// How deep in enums a way can go: deeper than any type written by hand.
const DEPTH: usize = 16;

/// The free type parameters of a function of scheme `s`, and the ways to
/// their values in its parameters and result: those that are in its result,
/// as only those say something. A parameter is free if each trait bound on
/// it is a marker, it's in no other bound (`U: Iterator<Item = T>` could
/// make one), and each of its uses in the parameters' and result's types
/// is a way.
///
/// For a method of an impl of a trait, `own` is the impl's type and trait:
/// what the impl says the trait's associated types are, `Option<T>`'s
/// `Output` is `T`, is in its methods' equalities, but is no bound. A value
/// of such a type is made only by a method of the impl, which can make a
/// `T` no more than this one can.
pub(crate) fn free_params(adts: &Adts, s: &Scheme, own: Option<(&Type, &str)>) -> Vec<Free> {
    let Type::Fn(sig) = &s.ty else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for name in s.params.as_slice() {
        if !s.params.is_type(name) {
            continue;
        }
        let this = Type::Param(name.clone());
        let bounded = s.preds.iter().all(|p| match p.ty == this {
            true => MARKERS.contains(&p.trait_name.as_str()),
            false => !mentions(&p.ty, name),
        });
        let is_own = |a: &Type| match (a, own) {
            (Type::Proj(base, tr, _), Some((ty, t))) => **base == *ty && tr == t,
            _ => false,
        };
        let equal = (s.eqs.iter())
            .filter(|(a, _)| !is_own(a))
            .any(|(a, b)| mentions(a, name) || mentions(b, name));
        let projected =
            (s.bounds.iter()).any(|b| matches!(b, Bound::Proj(t, _) if mentions(t, name)));
        if !bounded || equal || projected || mentions(&sig.effects, name) {
            continue;
        }
        let mut params = Vec::new();
        let mut all = true;
        for (i, p) in sig.params.iter().enumerate() {
            match ways(adts, p, name, 0) {
                Some(ws) => params.extend(ws.into_iter().map(|w| (i, w))),
                None => {
                    all = false;
                    break;
                }
            }
        }
        let Some(result) = ways(adts, &sig.ret, name, 0).filter(|_| all) else {
            continue;
        };
        if !result.is_empty() {
            out.push(Free {
                name: name.clone(),
                params,
                result,
            });
        }
    }
    out
}

/// Is type parameter `name` in `t`?
fn mentions(t: &Type, name: &str) -> bool {
    matches!(t, Type::Param(n) if n == name) || t.parts().any(|p| mentions(p, name))
}

/// The ways to the values of type parameter `name` in a value of type `t`,
/// or `None` if it's somewhere in `t` that isn't one: inside a reference, a
/// box, a struct, an array, a function or anything but an enum's field.
fn ways(adts: &Adts, t: &Type, name: &str, depth: usize) -> Option<Vec<Vec<(String, usize)>>> {
    if matches!(t, Type::Param(n) if n == name) {
        return Some(vec![Vec::new()]);
    }
    if !mentions(t, name) {
        return Some(Vec::new());
    }
    let Type::Adt(symbol, ..) = t else {
        return None;
    };
    let AdtKind::Enum(variants) = &adts.get(symbol)?.kind else {
        return None;
    };
    if depth == DEPTH {
        return None;
    }
    let mut out = Vec::new();
    for (v, (variant, _)) in variants.iter().enumerate() {
        for (f, ft) in variant_fields(adts, t, v).iter().enumerate() {
            for rest in ways(adts, ft, name, depth + 1)? {
                let mut way = vec![(variant.clone(), f)];
                way.extend(rest);
                out.push(way);
            }
        }
    }
    Some(out)
}
