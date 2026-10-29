//! `#[derive(..)]`: the impls it writes, and the bounds they need.

use super::fields;
use crate::ast::*;
use crate::check::Checker;
use crate::error::{Edit, Error, Fix, Span};
use crate::traits::{COPY, DISCARD, EQ};
use crate::traits::{ImplDef, Pred};
use crate::types::{Subst, Type};
use std::collections::HashMap;

impl Checker {
    /// The impl of each `#[derive(..)]`. What it needs of the type's
    /// parameters is worked out from its fields: exactly the bounds that
    /// make every field implement the trait. `#[derive(Copy)] struct
    /// W<T> { f: fn(T) -> i64 }` is `Copy` for every `T`, and `struct P<A,
    /// B> { a: Option<A>, b: &B }` only needs `A: Copy`. A bound can be of
    /// another trait: with `impl<T: Show> Eq for Q<T>`, a derived `Eq` of
    /// `struct W<T> { q: Q<T> }` needs `T: Show`, and `T: Eq` wouldn't do,
    /// as it doesn't make `Q<T>: Eq`. Not for `Discard` and `Copy`, which
    /// are only derived (`collect_impls_in`): every impl of one is this or
    /// built in, so by induction each needs only the same trait, and so
    /// does any proof of one (lemma D1 of docs/17-retrospective-1.md).
    ///
    /// A field can be of a type whose impl is derived too, maybe the type
    /// itself, through a reference: `struct Node<'a, T> { v: T, next:
    /// Option<&'a Node<'a, T>> }`. So the bounds are worked out together,
    /// starting from none and adding what's missing until nothing is: the
    /// least *fixed point*. That ends, as each round adds a bound or stops,
    /// and there are only so many parameters and traits.
    ///
    /// A derived `Copy` needs the type to be `Discard` too, as `trait Copy:
    /// Discard`, and it's never assumed: it says so with a fix.
    ///
    /// What an exported struct with fields that aren't derives says what
    /// it needs in a `where`, or needs nothing, if it has none (lemma M5 of
    /// docs/06d-modules.md, at the package's boundary since
    /// docs/24-visibility.md): those bounds are part of its package's
    /// interface, which fields other packages can't see mustn't change.
    /// Inside the package, what's computed is used: code there that counts
    /// on it is checked again with any change. Whatever the fields need must
    /// follow from what's written, which may say more, to leave room.
    pub(in crate::check) fn derive(&mut self, prog: &Program) -> Result<(), Error> {
        let decls = prog.all_types();
        // (impl index, type, the derive, trait)
        let mut derived: Vec<(usize, usize, Span, &'static str)> = Vec::new();
        for (i, (d, _)) in decls.iter().enumerate().skip(self.from.all_types) {
            for tr in [COPY, DISCARD, EQ] {
                let Some(at) = d.derive_of(tr) else {
                    continue;
                };
                let symbol = &self.types[i].symbol;
                let def = &self.adts[symbol];
                derived.push((self.traits.impls.len(), i, at.span, tr));
                self.traits.impls.push(ImplDef {
                    span: at.span,
                    trait_name: Some(tr.to_string()),
                    params: def.params.clone(),
                    lifetimes: def.lifetimes.clone(),
                    bounds: def.bounds.clone(),
                    self_ty: def.generic(symbol),
                    preds: Vec::new(),
                    eqs: def.eqs.clone(),
                    assoc: Vec::new(),
                    methods: Vec::new(),
                    derived: true,
                });
            }
        }
        // The bounds written down, by impl.
        let mut written: HashMap<usize, Option<&Where>> = HashMap::new();
        for &(k, i, _, tr) in &derived {
            let (d, _) = decls[i];
            let boundary = match d {
                TypeDecl::Struct(s) => {
                    s.vis.is_export() && s.fields.iter().any(|f| !f.vis.is_export())
                }
                TypeDecl::Enum(_) => false,
            };
            let w = d.derive_where(tr);
            if w.is_none() && !boundary {
                continue;
            }
            if let Some(w) = w {
                let def = &self.adts[&self.types[i].symbol];
                let (params, lifetimes) = (def.params.clone(), def.lifetimes.clone());
                if let Some((p, _)) = w.bounds.iter().find(|(p, _)| !params.contains(&p.name)) {
                    return Err(Error::ty(
                        p.span,
                        format!("`{}` isn't a type parameter of `{}`", p.name, d.name().name),
                    ));
                }
                // A derived `Discard` or `Copy` needs only more of the same
                // (see `collect_impls_in`), so that's all it can say.
                if (tr == DISCARD || tr == COPY)
                    && let Some(b) = w
                        .bounds
                        .iter()
                        .flat_map(|(_, b)| b)
                        .find(|b| b.name.name != tr)
                {
                    return Err(Error::ty(
                        b.span,
                        format!(
                            "a derived `{tr}` can only need `{tr}` of type parameters: that's \
                             all its fields can need"
                        ),
                    ));
                }
                let g = Generics {
                    bounds: w.bounds.clone(),
                    ..Generics::default()
                };
                let (preds, eqs) = self.preds(&g, &params, &lifetimes, false)?;
                self.traits.impls[k].preds = preds;
                self.traits.impls[k].eqs.extend(eqs);
            }
            written.insert(k, w);
        }
        loop {
            let mut changed = false;
            for &(k, i, _, tr) in &derived {
                let (d, _) = decls[i];
                let symbol = &self.types[i].symbol;
                let def = &self.adts[symbol];
                let mut needed: Vec<Pred> = Vec::new();
                for (what, t) in fields(&def.kind) {
                    if let Err(bad) = self.needs(&t, tr, &def.params, &mut needed) {
                        // A derived impl can't have a bound on a projection
                        // (see `preds`): only the trait's own will do.
                        if let Type::Proj(_, of, a) = &bad.ty {
                            let p = &bad.ty;
                            let held = match bad.ty == t {
                                true => format!("holds a `{t}`"),
                                false => format!("holds a `{t}`, which has a `{p}` in it"),
                            };
                            return Err(Error::ty(
                                derived.iter().find(|x| x.0 == k).expect("it").2,
                                format!(
                                    "`{}` can't be `{tr}`: its {what} {held}, and `{of}` \
                                     doesn't say that's `{tr}`; declare it as `type {a}: {tr};`",
                                    d.name().name
                                ),
                            ));
                        }
                        let why = match (&bad.ty, tr) {
                            (Type::Fn(_), EQ) => " (functions can't be compared)".to_string(),
                            (b, _) if *b == t => String::new(),
                            (b, _) => format!(" (it needs `{b}: {tr}`, and that doesn't hold)"),
                        };
                        return Err(Error::ty(
                            derived.iter().find(|x| x.0 == k).expect("it").2,
                            format!(
                                "`{}` can't be `{tr}`: its {what} holds a `{t}`, which isn't{why}",
                                d.name().name
                            ),
                        ));
                    }
                }
                // In a fixed order, so that a round that adds none changes
                // nothing.
                let position = |p: &Pred| {
                    def.params
                        .iter()
                        .position(|q| p.ty == Type::Param(q.clone()))
                };
                needed.sort_by(|a, b| {
                    (position(a), &a.trait_name).cmp(&(position(b), &b.trait_name))
                });
                let preds = needed;
                if written.contains_key(&k) {
                    continue;
                }
                if preds != self.traits.impls[k].preds {
                    self.traits.impls[k].preds = preds;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // What's written must give what the fields need.
        for &(k, i, span, tr) in &derived {
            let Some(&w) = written.get(&k) else {
                continue;
            };
            let (d, _) = decls[i];
            let def = &self.adts[&self.types[i].symbol];
            let mut needed: Vec<Pred> = Vec::new();
            for (_, t) in fields(&def.kind) {
                // What can't hold at all was reported above.
                let _ = self.needs(&t, tr, &def.params, &mut needed);
            }
            let imp = &self.traits.impls[k];
            let missing: Vec<&Pred> = needed
                .iter()
                .filter(|p| {
                    self.traits
                        .implements(&p.ty, &p.trait_name, &imp.preds, &imp.eqs)
                        .is_err()
                })
                .collect();
            let Some(first) = missing.first() else {
                continue;
            };
            let name = &d.name().name;
            let bounds: Vec<String> = needed.iter().map(|p| format!("{p}")).collect();
            let at = d.derive_of(tr).expect("derived");
            return Err(match w {
                Some(w) => Error::ty(
                    w.span,
                    format!("`{name}`'s `{tr}` needs `{first}`, which its `where` doesn't say"),
                ),
                None => Error::ty(
                    span,
                    format!(
                        "`{name}` is exported with fields that aren't, so its `{tr}` must say \
                         what it needs: `where {}`; what those fields need can't be part of its \
                         package's interface",
                        bounds.join(", ")
                    ),
                )
                .with_fix(Fix::new(
                    "say what it needs",
                    vec![Edit::insert(
                        at.span.end,
                        format!(" where {}", bounds.join(", ")),
                    )],
                )),
            });
        }
        for &(k, i, span, tr) in &derived {
            let def = &self.traits.impls[k];
            if tr != COPY
                || self
                    .traits
                    .implements(&def.self_ty, DISCARD, &def.preds, &def.eqs)
                    .is_ok()
            {
                continue;
            }
            let (d, _) = decls[i];
            let name = &d.name().name;
            let e = Error::ty(
                span,
                format!(
                    "`{name}` can't be `Copy` unless it's `Discard`: `trait Copy: Discard`, as a \
                     value that's copied can't be used up exactly once"
                ),
            );
            let copy = d.derive_of(COPY).expect("derived");
            return Err(match d.derive_of(DISCARD) {
                Some(_) => e,
                None => e.with_fix(Fix::new(
                    "derive `Discard` too",
                    vec![Edit::insert(copy.span.start, "Discard, ")],
                )),
            });
        }
        Ok(())
    }

    /// What `t` needs of `params` to implement `tr`, by the impls found so
    /// far: bounds on them, added to `out`, each of whatever trait it is.
    /// Or the obligation that fails whatever they implement.
    fn needs(
        &self,
        t: &Type,
        tr: &str,
        params: &[String],
        out: &mut Vec<Pred>,
    ) -> Result<(), Pred> {
        match t {
            Type::Param(p) if params.contains(p) => {
                let pred = Pred {
                    ty: t.clone(),
                    trait_name: tr.to_string(),
                };
                if !out.contains(&pred) {
                    out.push(pred);
                }
                return Ok(());
            }
            Type::Ref(_, _, inner) | Type::Box(inner) if tr == EQ => {
                return self.needs(inner, tr, params, out);
            }
            Type::Box(inner) if tr == DISCARD => return self.needs(inner, tr, params, out),
            // An array is its values, for all three.
            Type::Array(inner, _) => return self.needs(inner, tr, params, out),
            Type::Adt(..) => {}
            t => {
                return self
                    .traits
                    .implements(t, tr, &[], &[])
                    .map(|_| ())
                    .map_err(|_| Pred {
                        ty: t.clone(),
                        trait_name: tr.to_string(),
                    });
            }
        }
        let fail = || Pred {
            ty: t.clone(),
            trait_name: tr.to_string(),
        };
        let (i, args) = self.traits.select(t, Some(tr), &[]).ok_or_else(fail)?;
        let d = &self.traits.impls[i];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            self.needs(&s.apply(&p.ty), &p.trait_name, params, out)?;
        }
        Ok(())
    }
}
