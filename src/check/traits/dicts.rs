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

use super::{each_proj, fields, mentions};
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

/// The nodes of each impl, its dictionaries, by index in `traits.impls`:
/// for what code may call, and for what copies of types need.
pub(in crate::check) fn dict_names(impls: &[ImplDef]) -> (Vec<String>, Vec<String>) {
    let calls = call_dict_names(impls);
    let types = calls.iter().map(|n| format!("{n} (types)")).collect();
    (calls, types)
}

fn call_dict_names(impls: &[ImplDef]) -> Vec<String> {
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
    /// Each impl's dictionary, for what code may call: its methods.
    pub(in crate::check) dicts: &'a [String],
    /// Each impl's dictionary for what a copy of a type needs: only its
    /// associated types, as copying a type copies no method but a written
    /// `discard` (`Checker::discard_uses`).
    pub(in crate::check) type_dicts: &'a [String],
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
        (dicts, type_dicts): (&'a [String], &'a [String]),
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
            type_dicts,
            methods,
            schemes,
            derived_eq,
        }
    }

    /// The uses by which `site` proves `ty: tr`: an edge to the
    /// dictionary of each impl the proof picks, with what the impl's type
    /// parameters are, and on through what those impls need. Where `ty`
    /// has no head, a type parameter, a projection or an opaque type, the
    /// proof is an assumption: the impl was picked by whoever said what it
    /// is.
    pub(in crate::check) fn proof_uses<'a>(&self, site: &mut Site<'a, '_>, ty: &Type, tr: &str) {
        let ty = crate::traits::normalise(&self.traits.impls, &self.traits.anon(), &[], ty);
        if headless(&ty) || !site.seen.insert((ty.clone(), tr.to_string())) {
            return;
        }
        match (tr, &ty) {
            // The built-in impls that need the same of what's inside.
            (EQ, Type::Ref(_, _, inner) | Type::Box(inner) | Type::Slice(inner))
            | (DISCARD, Type::Box(inner))
            | (DISCARD | COPY | EQ, Type::Array(inner, _)) => {
                return self.proof_uses(site, inner, tr);
            }
            _ => {}
        }
        // A closure is what it captures.
        if let Type::Adt(symbol, ..) = &ty
            && let Some(c) = self.traits.closures.get(symbol)
        {
            if tr == DISCARD || tr == COPY {
                for f in c.fields_at(&ty) {
                    self.proof_uses(site, &f, tr);
                }
            }
            return;
        }
        let Some((k, args)) = self.traits.select(&ty, Some(tr), &[]) else {
            return;
        };
        let dicts = match site.types {
            true => site.ix.type_dicts,
            false => site.ix.dicts,
        };
        site.push(&dicts[k], args.clone(), None);
        let d = &self.traits.impls[k];
        let s = Subst::new(&d.params, &args);
        for p in &d.preds {
            self.proof_uses(site, &s.apply(&p.ty), &p.trait_name);
        }
    }

    /// The uses of a use in `from` of `to` at `args`, at `span`: of `to`
    /// itself, or, for a trait's method on a type with a head, of the
    /// method of the impl that's for it, or of the trait's default; and of
    /// the dictionaries of the impls its bounds are proved by. Where no
    /// impl is picked here, as on a type with no head, the impl's type
    /// parameters are the dictionary's business, and the method's own type
    /// arguments go to `UNKNOWN`.
    pub(in crate::check) fn call_uses<'a>(
        &self,
        ix: &Index<'a>,
        (from, span): (&'a str, Span),
        to: &'a str,
        args: &[Type],
        out: &mut Vec<Use<'a>>,
    ) {
        let mut site = Site::new(ix, from, span, out);
        let Some(&(tr, m)) = ix.methods.get(to) else {
            site.push(to, args.to_vec(), None);
            if let Some(scheme) = ix.schemes.get(to) {
                let s = Subst::new(&scheme.params, args);
                for p in &scheme.preds {
                    self.proof_uses(&mut site, &s.apply(&p.ty), &p.trait_name);
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
        let picked = match headless(self_ty) {
            true => None,
            false => self.traits.select(self_ty, Some(tr), &[]),
        };
        match picked {
            Some((k, impl_args)) => {
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
                    site.push(to, targs, None);
                }
            }
            None => {
                for t in own {
                    site.push(UNKNOWN, vec![t.clone()], Some(m.name.clone()));
                }
            }
        }
        // The dictionaries of what the impl needs, or what a built-in one
        // does: `Eq` of an array compares what's in it.
        self.proof_uses(&mut site, self_ty, tr);
        // The method's own bounds, at its own type arguments.
        let s = Subst::new(&m.scheme.params, args);
        for p in &m.scheme.preds {
            let t = s.apply(&p.ty);
            if p.trait_name != tr || t != args[0] {
                self.proof_uses(&mut site, &t, &p.trait_name);
            }
        }
    }

    /// The uses of each impl's dictionary: each of its associated types,
    /// named in `assoc`, at the impl's own type parameters, and, but for a
    /// type's dictionary (`types`), each of its methods, its derived `eq`,
    /// or the trait's default where it has none. And the dictionaries its
    /// own proofs pick, of its trait's supertraits and of what its
    /// associated types must implement, which code that has the dictionary
    /// may use.
    pub(in crate::check) fn dict_uses<'a>(
        &'a self,
        ix: &Index<'a>,
        assoc: &'a [(usize, String, String)],
        types: bool,
    ) -> Vec<Use<'a>> {
        let mut out = Vec::new();
        for (k, d) in self.traits.impls.iter().enumerate() {
            let Some(tr) = &d.trait_name else { continue };
            let mut site = match types {
                true => Site::types(ix, &ix.type_dicts[k], d.span, &mut out),
                false => Site::new(ix, &ix.dicts[k], d.span, &mut out),
            };
            let own: Vec<Type> = d.params.iter().map(|p| Type::Param(p.clone())).collect();
            let def = &self.traits.traits[tr];
            for (_, _, node) in assoc.iter().filter(|(j, ..)| *j == k) {
                site.push(node, own.clone(), None);
            }
            if !types {
                for m in &def.methods {
                    let written = d.methods.iter().find(|(n, _)| *n == m.name);
                    match (written, &ix.derived_eq[k]) {
                        (Some((_, symbol)), _) => {
                            if let Some(to) = ix.fn_symbol(symbol) {
                                site.push(to, own.clone(), None);
                            }
                        }
                        (None, Some(eq)) => site.push(eq, own.clone(), None),
                        (None, None) if !d.derived && m.provided => {
                            site.push(&m.symbol, vec![d.self_ty.clone()], None)
                        }
                        _ => {}
                    }
                }
            }
            for s in &def.supers {
                self.proof_uses(&mut site, &d.self_ty, s);
            }
            for (name, t) in &d.assoc {
                let decl = def.assoc.iter().find(|a| a.name == *name);
                for b in decl.map_or(&[][..], |a| &a.bounds) {
                    self.proof_uses(&mut site, t, b);
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
        let mut entry = |symbol: &str, skip: usize, span: Span| {
            let Some((&to, scheme)) = ix.schemes.get_key_value(symbol) else {
                return;
            };
            let n = scheme.params.as_slice().len();
            if n > skip {
                let mut args = vec![Type::Unit; skip];
                args.extend((skip..n).map(|_| Type::Param(UNKNOWN_PARAM.into())));
                Site::new(ix, UNKNOWN, span, &mut out).push(to, args, None);
            }
        };
        for d in &self.traits.impls {
            for (_, symbol) in &d.methods {
                entry(symbol, d.params.as_slice().len(), d.span);
            }
        }
        for def in self.traits.traits.values() {
            for m in def.methods.iter().filter(|m| m.provided) {
                entry(&m.symbol, 1, m.span.unwrap_or(Span::new(0, 0)));
            }
        }
        out
    }

    /// The uses by which `site`, a struct or enum type or an associated
    /// type, needs what's in type `t`: the dictionaries of the impls its
    /// projections with a head are worked out by, from which their
    /// associated types are reached, and of what the structs and enums in
    /// it need of their type arguments.
    pub(in crate::check) fn type_uses<'a>(&self, site: &mut Site<'a, '_>, t: &Type) {
        crate::check::each_adt(t, &mut |name, args| {
            if let Some(def) = self.adts.get(name) {
                let s = Subst::new(&def.params, args);
                for p in &def.preds {
                    self.proof_uses(site, &s.apply(&p.ty), &p.trait_name);
                }
            }
        });
        each_proj(t, &mut |s, tr| {
            self.proof_uses(site, s, tr);
        });
    }
}

impl Checker {
    /// A node for each associated type of each impl: the impl, the name,
    /// and the node's name.
    pub(in crate::check) fn assoc_nodes(&self) -> Vec<(usize, String, String)> {
        (self.traits.impls.iter().enumerate())
            .flat_map(|(k, d)| {
                let tr = d.trait_name.clone().unwrap_or_default();
                (d.assoc.iter())
                    .map(move |(n, _)| (k, n.clone(), format!("<{} as {tr}>::{n}", d.self_ty)))
            })
            .collect()
    }

    /// The uses of the graph of types: each struct or enum uses what its
    /// fields mention, each associated type what it says, and each type
    /// dictionary its associated types.
    pub(in crate::check) fn type_graph<'a>(
        &'a self,
        ix: &Index<'a>,
        nodes: &'a [(usize, String, String)],
    ) -> Vec<Use<'a>> {
        let mut uses = Vec::new();
        let mut names: Vec<&String> = self.adts.keys().collect();
        names.sort();
        for a in names {
            let def = &self.adts[a];
            // Its name, where it's declared.
            let at = self.defs.types.get(a).copied().unwrap_or(Span::new(0, 0));
            for (_, f) in fields(&def.kind) {
                self.mention_uses(&mut Site::types(ix, a, at, &mut uses), &f);
            }
        }
        for (k, n, node) in nodes {
            let d = &self.traits.impls[*k];
            let (_, t) = d.assoc.iter().find(|(m, _)| m == n).expect("it");
            self.mention_uses(&mut Site::types(ix, node, d.span, &mut uses), t);
        }
        uses.extend(self.dict_uses(ix, nodes, true));
        uses
    }

    /// The uses by which `site` mentions type `t`: each struct and enum in
    /// it, which monomorphisation copies, and what `type_uses` says.
    pub(in crate::check) fn mention_uses<'a>(&'a self, site: &mut Site<'a, '_>, t: &Type) {
        crate::check::each_adt(t, &mut |name, args| {
            if let Some((to, _)) = self.adts.get_key_value(name) {
                site.push(to, args.to_vec(), None);
            }
        });
        self.type_uses(site, t);
    }

    /// The uses of each struct or enum with a `discard` of its own, which
    /// monomorphisation makes with each copy of the type, as it's called
    /// wherever a value of it is dropped, which nothing names. Each of the
    /// impl's type parameters is the type's parameter it's in: a part of
    /// it, so no bigger.
    pub(in crate::check) fn discard_uses<'a>(&'a self) -> Vec<Use<'a>> {
        let mut out = Vec::new();
        for d in &self.traits.impls {
            let (Some(DISCARD), false, Type::Adt(name, ..)) =
                (d.trait_name.as_deref(), d.derived, &d.self_ty)
            else {
                continue;
            };
            let (Some((from, def)), Some((_, discard))) =
                (self.adts.get_key_value(name), d.methods.first())
            else {
                continue;
            };
            let generic = def.generic(name);
            let renamed: Vec<Type> = (d.params.iter())
                .map(|p| Type::Param(format!("{p}'")))
                .collect();
            let header = Subst::new(&d.params, &renamed).apply(&d.self_ty);
            let mut vars: Vec<String> = def.params.iter().cloned().collect();
            vars.extend(d.params.iter().map(|p| format!("{p}'")));
            let Some(mgu) = crate::traits::unifier(&generic, &header, &vars) else {
                continue;
            };
            let args = (d.params.iter())
                .map(|p| {
                    let p = format!("{p}'");
                    let within = def.params.iter().find(|q| {
                        let t = crate::traits::unified(&Type::Param((*q).clone()), &mgu);
                        mentions(&t, &p)
                    });
                    within.map_or(Type::Unit, |q| Type::Param(q.clone()))
                })
                .collect();
            if let Some(to) = self.fns.iter().find(|f| f.symbol == *discard) {
                out.push(Use {
                    from,
                    to: &to.symbol,
                    args,
                    via: None,
                    span: d.span,
                });
            }
        }
        out
    }

    /// The type parameters of every node of the graph.
    pub(in crate::check) fn graph_params<'a>(
        &'a self,
        ix: &Index<'a>,
        nodes: &'a [(usize, String, String)],
    ) -> HashMap<&'a str, &'a [String]> {
        let mut params: HashMap<&str, &[String]> = (self.fns.iter())
            .map(|f| (f.symbol.as_str(), f.scheme.params.as_slice()))
            .collect();
        params.extend((self.adts.iter()).map(|(name, def)| (name.as_str(), def.params.as_slice())));
        for (k, d) in self.traits.impls.iter().enumerate() {
            params.insert(&ix.dicts[k], d.params.as_slice());
            params.insert(&ix.type_dicts[k], d.params.as_slice());
            if let Some(eq) = &ix.derived_eq[k] {
                params.insert(eq, d.params.as_slice());
            }
        }
        for (k, _, node) in nodes {
            params.insert(node, self.traits.impls[*k].params.as_slice());
        }
        params.insert(UNKNOWN, UNKNOWN_PARAMS);
        params
    }
}

/// Where uses are from: what, where, and those found so far.
pub(in crate::check) struct Site<'a, 'o> {
    ix: &'o Index<'a>,
    from: &'a str,
    span: Span,
    /// Is it a struct or enum type, or an associated type, or the
    /// mention of one, whose proofs pick type dictionaries?
    types: bool,
    /// What's been proved here, to prove it once.
    seen: HashSet<(Type, String)>,
    out: &'o mut Vec<Use<'a>>,
}

impl<'a, 'o> Site<'a, 'o> {
    pub(in crate::check) fn new(
        ix: &'o Index<'a>,
        from: &'a str,
        span: Span,
        out: &'o mut Vec<Use<'a>>,
    ) -> Site<'a, 'o> {
        Site {
            ix,
            from,
            span,
            types: false,
            seen: HashSet::new(),
            out,
        }
    }

    /// A site of a type's: of a struct or enum, an associated type, or a
    /// function's mention of a type, whose proofs pick what copies of the
    /// type need.
    pub(in crate::check) fn types(
        ix: &'o Index<'a>,
        from: &'a str,
        span: Span,
        out: &'o mut Vec<Use<'a>>,
    ) -> Site<'a, 'o> {
        Site {
            types: true,
            ..Site::new(ix, from, span, out)
        }
    }

    fn push(&mut self, to: &'a str, args: Vec<Type>, via: Option<String>) {
        self.out.push(Use {
            from: self.from,
            to,
            args,
            via,
            span: self.span,
        });
    }
}

/// Is `t` a type with no head, whose impls are picked elsewhere: a type
/// parameter, a projection, an opaque type, or one not known yet?
fn headless(t: &Type) -> bool {
    matches!(
        t,
        Type::Param(_) | Type::Proj(..) | Type::Opaque(..) | Type::Var(_)
    )
}
