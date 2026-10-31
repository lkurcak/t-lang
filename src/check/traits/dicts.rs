//! Where impls are picked, for the checks against polymorphic recursion
//! (`Checker::no_polymorphic_recursion`, `Checker::assoc_regular`).
//!
//! A call of a trait's method on a type parameter, `x.show()` with `x: T`
//! in `fn f<T: Show>`, runs the impl of whatever `T` is. Which one that is
//! was decided where `T` got a type with a head: where some caller proved
//! `Option<U>: Show`, say, by `impl<V: Show> Show for Option<V>`. That's
//! where the edge to the impl goes, *proof-directed*: from the place that
//! picks it, which can name the impl's type or trait, so is in its package
//! or one that depends on it. Monomorphisation passes the impl along, as a
//! dictionary would be in a dictionary-passing semantics: each impl is a
//! node of its own here, its *dictionary*, from which each of its methods
//! and associated types is reached, with the impl's type parameters.
//!
//! What can't be decided there are a method's own type arguments, `K` in
//! `x.two::<K>(..)`, given where it's called, to whatever impl the
//! dictionary is. Those go to one node for every impl in every package,
//! `UNKNOWN`, which passes them on to each method's own type parameters,
//! at the same size. So no package can pass a bigger one on to an impl it
//! doesn't know: each keeps every path from an impl's method's own type
//! parameters to such a call from getting bigger, and then no cycle made
//! of such paths can (lemma R2 of docs/29-closed-and-sealed-traits.md).
//!
//! Every edge goes from a package to one it depends on, or to `UNKNOWN`,
//! so what a package's check finds depends on it and its dependencies
//! alone, and adding a dependency to a program can't make it fail.

use super::each_proj;
use crate::check::Checker;
use crate::check::generics::Use;
use crate::error::Span;
use crate::traits::{COPY, DISCARD, EQ, ImplDef, eq_subject};
use crate::types::{Subst, Type};
use std::collections::{HashMap, HashSet};

/// The node of every impl's methods' own type parameters, which a call of
/// a generic method of a trait on a type parameter passes its own type
/// arguments to.
pub(in crate::check) const UNKNOWN: &str = "(any impl)";

/// The one type parameter of `UNKNOWN`, which no written one can be.
const UNKNOWN_PARAM: &str = "";

/// `UNKNOWN`'s type parameters, for `generics::polymorphic_recursion`.
pub(in crate::check) const UNKNOWN_PARAMS: &[String] = &[String::new()];

/// The node of each impl, its dictionary, by index in `traits.impls`.
pub(in crate::check) fn dict_names(impls: &[ImplDef]) -> Vec<String> {
    let mut seen = HashSet::new();
    (impls.iter().enumerate())
        .map(|(k, d)| {
            let tr = d.trait_name.as_deref().unwrap_or("");
            let name = format!("impl {} for {}", tr, d.self_ty.symbols());
            match seen.insert(name.clone()) {
                true => name,
                false => format!("{name} #{k}"),
            }
        })
        .collect()
}

/// What the uses need to know of the program, indexed once.
pub(in crate::check) struct Index<'a> {
    pub(in crate::check) dicts: &'a [String],
    /// Each trait's method by symbol: the trait and its method.
    methods: HashMap<&'a str, (&'a str, &'a crate::traits::MethodDef)>,
    /// Each function's type scheme, by symbol.
    schemes: HashMap<&'a str, &'a crate::types::Scheme>,
    /// The name of each derived `Eq`'s `eq`, by impl.
    derived_eq: &'a [Option<String>],
}

impl<'a> Index<'a> {
    /// The symbol `symbol` is of a trait's method, as `Index` keeps it.
    pub(in crate::check) fn method_symbol(&self, symbol: &str) -> &'a str {
        self.methods
            .get_key_value(symbol)
            .expect("a trait's method")
            .0
    }

    /// The symbol `symbol` is of a function, as `Index` keeps it.
    pub(in crate::check) fn fn_symbol(&self, symbol: &str) -> Option<&'a str> {
        self.schemes.get_key_value(symbol).map(|(s, _)| *s)
    }
}

impl Checker {
    pub(in crate::check) fn index<'a>(
        &'a self,
        dicts: &'a [String],
        derived_eq: &'a [Option<String>],
    ) -> Index<'a> {
        let methods = (self.traits.traits.iter())
            .flat_map(|(t, d)| {
                d.methods
                    .iter()
                    .map(move |m| (m.symbol.as_str(), (t.as_str(), m)))
            })
            .collect();
        let schemes = (self.fns.iter())
            .map(|f| (f.symbol.as_str(), &f.scheme))
            .collect();
        Index {
            dicts,
            methods,
            schemes,
            derived_eq,
        }
    }

    /// The uses by which `from`, with type parameters `params`, proves
    /// `ty: tr`: an edge to the dictionary of each impl the proof picks,
    /// with what the impl's type parameters are, and on through what those
    /// impls need. Where `ty` has no head, a type parameter, a projection
    /// or an opaque type, the proof is an assumption: the impl was picked
    /// by whoever said what it is.
    pub(in crate::check) fn proof_uses<'a>(
        &self,
        ix: &Index<'a>,
        from: &'a str,
        ty: &Type,
        tr: &str,
        span: Span,
        seen: &mut HashSet<(Type, String)>,
        out: &mut Vec<Use<'a>>,
    ) {
        let ty = crate::traits::normalise(&self.traits.impls, &self.traits.anon(), &[], ty);
        if !seen.insert((ty.clone(), tr.to_string())) {
            return;
        }
        let mut more = |t: &Type, tr: &str, out: &mut Vec<Use<'a>>| {
            self.proof_uses(ix, from, t, tr, span, seen, out)
        };
        match (tr, &ty) {
            (_, Type::Param(_) | Type::Proj(..) | Type::Opaque(..) | Type::Var(_)) => return,
            // The built-in impls that need the same of what's inside.
            (EQ, Type::Ref(_, _, inner) | Type::Box(inner) | Type::Slice(inner))
            | (DISCARD, Type::Box(inner))
            | (DISCARD | COPY | EQ, Type::Array(inner, _)) => {
                more(inner, tr, out);
                return;
            }
            _ => {}
        }
        // A closure is what it captures.
        if let Type::Adt(symbol, ..) = &ty
            && let Some(c) = self.traits.closures.get(symbol)
        {
            if tr == DISCARD || tr == COPY {
                for f in c.fields_at(&ty) {
                    more(&f, tr, out);
                }
            }
            return;
        }
        let Some((k, args)) = self.traits.select(&ty, Some(tr), &[]) else {
            return;
        };
        out.push(Use {
            from,
            to: &ix.dicts[k],
            args: args.clone(),
            via: None,
            span,
        });
        let d = &self.traits.impls[k];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            more(&s.apply(&p.ty), &p.trait_name, out);
        }
    }

    /// The uses of a use in `from` of `to` at `args`, at `span`: of `to`
    /// itself, or, for a trait's method on a type with a head, of the
    /// method of the impl that's for it, or of the trait's default; and of
    /// the dictionaries of the impls its bounds are proved by. On a type
    /// with no head, the impl's type parameters are the dictionary's
    /// business, and the method's own type arguments go to `UNKNOWN`.
    pub(in crate::check) fn call_uses<'a>(
        &self,
        ix: &Index<'a>,
        from: &'a str,
        to: &'a str,
        args: &[Type],
        span: Span,
        out: &mut Vec<Use<'a>>,
    ) {
        let mut seen = HashSet::new();
        let Some(&(tr, m)) = ix.methods.get(to) else {
            out.push(Use {
                from,
                to,
                args: args.to_vec(),
                via: None,
                span,
            });
            if let Some(scheme) = ix.schemes.get(to) {
                let s = Subst::new(&scheme.params, args);
                for p in &scheme.preds {
                    self.proof_uses(
                        ix,
                        from,
                        &s.apply(&p.ty),
                        &p.trait_name,
                        span,
                        &mut seen,
                        out,
                    );
                }
            }
            return;
        };
        let (self_ty, own) = args.split_first().expect("`Self` first");
        let self_ty = match tr {
            EQ => eq_subject(self_ty),
            _ => self_ty,
        };
        let self_ty =
            &crate::traits::normalise(&self.traits.impls, &self.traits.anon(), &[], self_ty);
        let headless = matches!(
            self_ty,
            Type::Param(_) | Type::Proj(..) | Type::Opaque(..) | Type::Var(_)
        );
        match self.traits.select(self_ty, Some(tr), &[]) {
            Some((k, impl_args)) if !headless => {
                let d = &self.traits.impls[k];
                let derived: Option<&'a str> = ix.derived_eq[k].as_deref();
                let target: Option<(&'a str, Vec<Type>)> =
                    match (d.methods.iter().find(|(n, _)| *n == m.name), derived) {
                        (Some((_, symbol)), _) => {
                            ix.fn_symbol(symbol).map(|to| (to, impl_args.clone()))
                        }
                        (None, Some(eq)) => Some((eq, impl_args.clone())),
                        (None, None) if !d.derived && m.provided => {
                            Some((m.symbol.as_str(), vec![self_ty.clone()]))
                        }
                        _ => None,
                    };
                if let Some((to, mut targs)) = target {
                    targs.extend(own.iter().cloned());
                    out.push(Use {
                        from,
                        to,
                        args: targs,
                        via: None,
                        span,
                    });
                }
                // The dictionaries of what the impl needs.
                self.proof_uses(ix, from, self_ty, tr, span, &mut seen, out);
            }
            _ if headless && !own.is_empty() => {
                for t in own {
                    out.push(Use {
                        from,
                        to: UNKNOWN,
                        args: vec![t.clone()],
                        via: Some(m.name.clone()),
                        span,
                    });
                }
            }
            _ => {
                // A built-in impl: of `Eq` on an array or slice, say, which
                // compares what's in it.
                self.proof_uses(ix, from, self_ty, tr, span, &mut seen, out);
            }
        }
        // The method's own bounds, at its own type arguments.
        let s = Subst::new(&m.scheme.params, args);
        for p in &m.scheme.preds {
            let t = s.apply(&p.ty);
            if p.trait_name == tr && t == *args.first().expect("`Self`") {
                continue;
            }
            self.proof_uses(ix, from, &t, &p.trait_name, span, &mut seen, out);
        }
    }

    /// The uses of each impl's dictionary: each of its methods, its derived
    /// `eq`, or the trait's default where it has none, at the impl's own
    /// type parameters; and the dictionaries its own proofs pick, of its
    /// trait's supertraits and of what its associated types must implement,
    /// which code that has the dictionary may use. With `assoc`, those of
    /// the types graph instead: to each of its associated types.
    pub(in crate::check) fn dict_uses<'a>(
        &'a self,
        ix: &Index<'a>,
        assoc: &'a [(usize, String, String)],
        types: bool,
    ) -> Vec<Use<'a>> {
        let mut out = Vec::new();
        for (k, d) in self.traits.impls.iter().enumerate() {
            let Some(tr) = &d.trait_name else { continue };
            let from = ix.dicts[k].as_str();
            let own: Vec<Type> = d.params.iter().map(|p| Type::Param(p.clone())).collect();
            let to = |name: &'a str, args: Vec<Type>, out: &mut Vec<Use<'a>>| {
                out.push(Use {
                    from,
                    to: name,
                    args,
                    via: None,
                    span: d.span,
                })
            };
            if types {
                for (_, _, node) in assoc.iter().filter(|(j, ..)| *j == k) {
                    to(node, own.clone(), &mut out);
                }
            } else {
                let def = &self.traits.traits[tr];
                for m in &def.methods {
                    match (
                        d.methods.iter().find(|(n, _)| *n == m.name),
                        &ix.derived_eq[k],
                    ) {
                        (Some((_, symbol)), _) => {
                            if let Some((&name, _)) = ix.schemes.get_key_value(symbol.as_str()) {
                                to(name, own.clone(), &mut out);
                            }
                        }
                        (None, Some(eq)) => to(eq, own.clone(), &mut out),
                        (None, None) if !d.derived && m.provided => {
                            to(&m.symbol, vec![d.self_ty.clone()], &mut out)
                        }
                        _ => {}
                    }
                }
            }
            let mut seen = HashSet::new();
            for s in &self.traits.traits[tr].supers {
                self.proof_uses(ix, from, &d.self_ty, s, d.span, &mut seen, &mut out);
            }
            for (name, t) in &d.assoc {
                let decl = self.traits.traits[tr]
                    .assoc
                    .iter()
                    .find(|a| a.name == *name);
                for b in decl.map_or(&[][..], |a| &a.bounds) {
                    self.proof_uses(ix, from, t, b, d.span, &mut seen, &mut out);
                }
            }
        }
        out
    }

    /// The uses of `UNKNOWN`: each method of an impl, and each default of a
    /// trait, with type parameters of its own gets them from it, as they
    /// were, whoever called it.
    pub(in crate::check) fn unknown_uses<'a>(&'a self, ix: &Index<'a>) -> Vec<Use<'a>> {
        let mut out = Vec::new();
        let entry = |symbol: &str, skip: usize, span: Span, out: &mut Vec<Use<'a>>| {
            let Some((&to, scheme)) = ix.schemes.get_key_value(symbol) else {
                return;
            };
            let n = scheme.params.as_slice().len();
            if n <= skip {
                return;
            }
            let mut args = vec![Type::Unit; skip];
            args.extend((skip..n).map(|_| Type::Param(UNKNOWN_PARAM.into())));
            out.push(Use {
                from: UNKNOWN,
                to,
                args,
                via: None,
                span,
            });
        };
        for d in &self.traits.impls {
            for (_, symbol) in &d.methods {
                entry(symbol, d.params.as_slice().len(), d.span, &mut out);
            }
        }
        for def in self.traits.traits.values() {
            for m in def.methods.iter().filter(|m| m.provided) {
                entry(&m.symbol, 1, m.span.unwrap_or(Span::new(0, 0)), &mut out);
            }
        }
        out
    }

    /// The uses by which struct or enum type, or associated type, `from`,
    /// with type parameters `params`, needs what's in type `t`: the impls
    /// its projections with a head are worked out by, and the dictionaries
    /// of what the structs and enums in it need of their type arguments.
    pub(in crate::check) fn type_proof_uses<'a>(
        &self,
        ix: &Index<'a>,
        assoc: &'a [(usize, String, String)],
        from: &'a str,
        t: &Type,
        span: Span,
        out: &mut Vec<Use<'a>>,
    ) {
        let mut seen = HashSet::new();
        crate::check::each_adt(t, &mut |name, args| {
            if let Some(def) = self.adts.get(name) {
                let s = Subst::new(&def.params, args);
                for p in &def.preds {
                    self.proof_uses(
                        ix,
                        from,
                        &s.apply(&p.ty),
                        &p.trait_name,
                        span,
                        &mut seen,
                        out,
                    );
                }
            }
        });
        each_proj(t, &mut |s, tr| {
            let headless = matches!(s, Type::Param(_) | Type::Proj(..) | Type::Opaque(..));
            if headless {
                return;
            }
            let Some(name) = super::impls::proj_name(t, s, tr) else {
                return;
            };
            if let Some((k, args)) = self.traits.select(s, Some(tr), &[])
                && let Some((_, _, node)) = assoc.iter().find(|(j, n, _)| *j == k && *n == name)
            {
                out.push(Use {
                    from,
                    to: node,
                    args,
                    via: None,
                    span,
                });
            }
            self.proof_uses(ix, from, s, tr, span, &mut seen, out);
        });
    }
}
