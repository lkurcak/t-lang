//! Checking impls against their traits: signatures, associated types,
//! the orphan rule, and that no two impls overlap (*coherence*).

use super::methods::split;
use super::{Hidden, each_proj, fields, hide_projections, shown_projections};
use crate::ast::*;
use crate::check::generics;
use crate::check::{Checker, plural};
use crate::error::{Edit, Error, Fix, Span};
use crate::traits::{ImplDef, MethodDef, Pred, overlap};
use crate::types::{Bound, Region, Scheme, Subst, Type};
use std::collections::HashMap;

impl Checker {
    /// Pass 4, once every impl and method signature is known: the rules
    /// for impls. Each impl of a trait has the trait's methods at the
    /// trait's types, and the trait's supertraits; an impl of `Copy` or
    /// `Discard` is for a type whose fields all are; no two impls overlap;
    /// and proving
    /// that a type implements a trait always ends.
    pub(in crate::check) fn check_impls(&mut self, prog: &Program) -> Result<(), Error> {
        for (k, d) in prog.impls.iter().enumerate() {
            let def = self.traits.impls[self.impl_ids[k]].clone();
            self.env = def.preds.clone();
            self.well_formed_now(&def.self_ty, d.self_ty.span)?;
            let Some(tr) = &def.trait_name else {
                continue;
            };
            for f in &d.methods {
                let id = self.method_ids[&f.name.id];
                let got = self.fns[id.0].scheme.clone();
                let want = self
                    .traits
                    .method(tr, &f.name.name)
                    .expect("checked")
                    .clone();
                self.same_signature(f, &got, &want, &def)?;
            }
            for s in self.traits.traits[tr].supers.clone() {
                if self
                    .traits
                    .implements(&def.self_ty, &s, &def.preds, &def.eqs)
                    .is_err()
                {
                    return Err(Error::ty(
                        def.span,
                        format!(
                            "`{}` doesn't implement `{s}`, which `{tr}` needs, as it's \
                             `trait {tr}: {}`",
                            def.self_ty,
                            self.traits.traits[tr].supers.join(" + ")
                        ),
                    ));
                }
            }
            self.assoc_bounds(&def, d)?;
            if tr == "Copy" || tr == "Discard" {
                let Type::Adt(symbol, ..) = &def.self_ty else {
                    unreachable!("checked")
                };
                let adt = &self.adts[symbol];
                let s = Subst::new(&adt.params, &args_of(&def.self_ty));
                for (what, t) in fields(&adt.kind) {
                    let t = self.adts.normalise(&s.apply(&t));
                    if self
                        .traits
                        .implements(&t, tr, &def.preds, &def.eqs)
                        .is_err()
                    {
                        return Err(Error::ty(
                            def.span,
                            format!(
                                "`{}` can't be `{tr}`: its {what} holds a `{t}`, which isn't",
                                def.self_ty
                            ),
                        ));
                    }
                }
            }
        }
        self.env.clear();
        self.adts.proj.local.clear();
        self.coherence()?;
        self.termination()?;
        self.bound_termination()?;
        self.assoc_termination()?;
        self.assoc_sizes()?;
        self.assoc_regular()
    }

    /// Monomorphisation makes a copy of every struct and enum type a copy
    /// mentions, even through a box, and a field's projection is what an
    /// impl says it is, which can mention more: `struct S<T: Tr> { b:
    /// Box<T::Out> }` with `type Out = S<W<W<U>>>` for `W<U>` makes
    /// `S<W<int>>` need `S<W<W<int>>>`, and so on, with no end. So that's
    /// polymorphic recursion too (`generics::polymorphic_recursion`), with
    /// each impl's associated type a node of its own, generic over the
    /// impl's parameters, which a projection in a field gets to the way a
    /// method call gets to an impl's method (`dispatch`), and which uses
    /// the types and projections in what it says.
    fn assoc_regular(&self) -> Result<(), Error> {
        // A name for each associated type of each impl.
        let nodes: Vec<(usize, String, String)> = self
            .traits
            .impls
            .iter()
            .enumerate()
            .flat_map(|(k, d)| {
                let tr = d.trait_name.clone().unwrap_or_default();
                d.assoc
                    .iter()
                    .map(move |(n, _)| (k, n.clone(), format!("<{} as {tr}>::{n}", d.self_ty)))
            })
            .collect();
        let mut uses: Vec<generics::Use> = Vec::new();
        // To each impl a projection on `s` in `from` may be worked out by.
        let through = |from_params: &[String], s: &Type, tr: &str, name: &str, span: Span| {
            let mut out = Vec::new();
            let mut hidden: Vec<Hidden> = Vec::new();
            let s = &hide_projections(s, from_params, &mut hidden);
            for (k, n, node) in &nodes {
                let d = &self.traits.impls[*k];
                if n != name || d.trait_name.as_deref() != Some(tr) {
                    continue;
                }
                let renamed: Vec<Type> = d
                    .params
                    .iter()
                    .map(|p| Type::Param(format!("{p}'")))
                    .collect();
                let header = Subst::new(&d.params, &renamed).apply(&d.self_ty);
                let mut vars: Vec<String> = d.params.iter().map(|p| format!("{p}'")).collect();
                vars.extend(from_params.iter().cloned());
                vars.extend(hidden.iter().map(|h| h.0.clone()));
                if !overlap(&header, s, &vars) {
                    continue;
                }
                let mut found: Vec<Option<Type>> = vec![None; d.params.len()];
                let mut inside = Vec::new();
                split(
                    &d.self_ty,
                    s,
                    &d.params,
                    (from_params, &hidden),
                    &mut found,
                    &mut inside,
                );
                let args = found
                    .into_iter()
                    .map(|t| shown_projections(&t.unwrap_or(Type::Unit), &hidden))
                    .collect();
                out.push((node.as_str(), args, inside, span));
            }
            out
        };
        let mut edges: Vec<Edge> = Vec::new();
        let mut names: Vec<&String> = self.adts.keys().collect();
        names.sort();
        for a in &names {
            let def = &self.adts[*a];
            // Its name, where it's declared.
            let at = self.defs.types.get(*a).copied().unwrap_or(Span::new(0, 0));
            for (_, f) in fields(&def.kind) {
                crate::check::each_adt(&f, &mut |to, args| {
                    edges.push((
                        (*a).clone(),
                        self.adt_key(to),
                        args.to_vec(),
                        Vec::new(),
                        at,
                    ));
                });
                each_proj(&f, &mut |s, tr| {
                    if let Some(name) = proj_name(&f, s, tr) {
                        for (to, args, inside, span) in through(&def.params, s, tr, &name, at) {
                            edges.push(((*a).clone(), to, args, inside, span));
                        }
                    }
                });
            }
        }
        for (k, n, node) in &nodes {
            let d = &self.traits.impls[*k];
            let (_, t) = d.assoc.iter().find(|(m, _)| m == n).expect("it");
            crate::check::each_adt(t, &mut |to, args| {
                edges.push((
                    node.clone(),
                    self.adt_key(to),
                    args.to_vec(),
                    Vec::new(),
                    d.span,
                ));
            });
            each_proj(t, &mut |s, tr| {
                if let Some(name) = proj_name(t, s, tr) {
                    for (to, args, inside, span) in through(&d.params, s, tr, &name, d.span) {
                        edges.push((node.clone(), to, args, inside, span));
                    }
                }
            });
        }
        if nodes.is_empty() {
            return Ok(());
        }
        uses.extend(
            edges
                .iter()
                .map(|(from, to, args, inside, span)| generics::Use {
                    from,
                    to,
                    args: args.clone(),
                    inside: inside.clone(),
                    span: *span,
                }),
        );
        uses.sort_by_key(|u| u.span.start);
        let mut params: HashMap<&str, &[String]> = self
            .adts
            .iter()
            .map(|(name, def)| (name.as_str(), def.params.as_slice()))
            .collect();
        for (k, _, node) in &nodes {
            params.insert(node, &self.traits.impls[*k].params);
        }
        generics::polymorphic_recursion(&uses, &params)
    }

    /// The name a struct or enum's symbol is a node by, in `assoc_regular`.
    fn adt_key<'a>(&'a self, symbol: &str) -> &'a str {
        self.adts.get_key_value(symbol).expect("a type").0
    }

    /// What impl `def` (written as `d`) says its trait's associated types
    /// are must make sense where it's used: what's in them implements what
    /// it must, projections in them are worked out, and they implement what
    /// the trait declares of them, which generic code assumes.
    fn assoc_bounds(&mut self, def: &ImplDef, d: &ImplDecl) -> Result<(), Error> {
        let Some(tr) = &def.trait_name else {
            return Ok(());
        };
        self.env = def.preds.clone();
        self.adts.proj.local = def.eqs.clone();
        for (name, rhs) in &def.assoc {
            let span = d
                .assoc
                .iter()
                .find(|a| a.name.name == *name)
                .map_or(def.span, |a| a.ty.span);
            let t = self.adts.normalise(rhs);
            if let Some(p) = rigid_on_type(&t) {
                let Type::Proj(s, of, _) = &p else {
                    unreachable!("a projection")
                };
                return Err(Error::ty(
                    span,
                    format!("`{s}` doesn't implement `{of}`, so `{p}` means nothing"),
                ));
            }
            self.well_formed_now(&t, span)?;
            let decl = self.traits.traits[tr]
                .assoc
                .iter()
                .find(|a| a.name == *name)
                .expect("checked")
                .clone();
            for b in &decl.bounds {
                let local = &self.adts.proj.local;
                if let Err(bad) = self.traits.implements(&t, b, &self.env, local) {
                    let needs = match bad.ty == t {
                        true => String::new(),
                        false => format!(" (that needs `{bad}`)"),
                    };
                    return Err(Error::ty(
                        span,
                        format!(
                            "`{t}` doesn't implement `{b}`, which `{tr}` needs of `{name}`, as \
                             it declares `type {name}: {}`{needs}",
                            decl.bounds.join(" + ")
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// The orphan rule (lemma C2 of docs/06e-core.md): an impl of trait `tr`
    /// for `self_ty`, `d`, must be in the package of the trait, or of the
    /// type's head, its outermost constructor. Built-in traits and types
    /// are the core library's. So a library's impls are all there are for
    /// its own traits and types, whatever program it's in, and no program
    /// (nor the other library) can add one that overlaps them, or give a
    /// different one to another program: coherence holds for each package
    /// on its own. Looking only at the head is the strictest form of the
    /// rule, and can be relaxed.
    pub(super) fn orphan(
        &self,
        prog: &Program,
        d: &ImplDecl,
        tr: &str,
        self_ty: &Type,
    ) -> Result<(), Error> {
        let m = &self.modules;
        let package = m.package_root(d.module);
        let core = m.core.unwrap_or(package);
        let trait_package = prog
            .traits
            .iter()
            .find(|t| t.name.name == tr)
            .map_or(core, |t| m.package_root(t.module));
        // A type not declared anywhere is a closure's, which is its
        // function's package's.
        let head_package = match self_ty {
            Type::Adt(symbol, ..) => (self.types.iter())
                .find(|t| t.symbol == *symbol)
                .map_or(package, |t| m.package_root(t.module)),
            _ => core,
        };
        if trait_package == package || head_package == package {
            return Ok(());
        }
        let head = match self_ty {
            Type::Adt(symbol, ..) => symbol.rsplit("::").next().unwrap_or(symbol).to_string(),
            t => t.to_string(),
        };
        let tr = tr.rsplit("::").next().unwrap_or(tr);
        let (a, b) = (&m.mods[trait_package].path, &m.mods[head_package].path);
        let message = match a == b {
            true => format!("`{tr}` and `{head}` are both `{a}`'s, so only `{a}` can"),
            false => {
                format!("`{tr}` is `{a}`'s and `{head}` is `{b}`'s, so only `{a}` or `{b}` can")
            }
        };
        Err(Error::ty(
            Span::new(d.span.start, d.self_ty.span.end),
            format!(
                "{message} implement one for the other; to implement it for a type of your own, \
                 wrap it in a struct"
            ),
        ))
    }

    /// Checks that method `f`, of type `got`, of impl `d`, is method
    /// `want` of its trait, at the impl's type. Up to the names of their
    /// own type parameters and lifetimes: the impl's method can't take less
    /// or more than the trait says, or need more of its type parameters,
    /// or callers going by the trait's signature would be wrong.
    fn same_signature(
        &self,
        f: &FnDecl,
        got: &Scheme,
        want: &MethodDef,
        d: &ImplDef,
    ) -> Result<(), Error> {
        let tr = d.trait_name.as_deref().expect("a trait's impl");
        let mismatch = |what: String| {
            let decl = want.span;
            let e = Error::ty(
                f.name.span,
                format!(
                    "method `{}` doesn't match its declaration in trait `{tr}`: {what}",
                    f.name.name
                ),
            );
            match decl {
                Some(span) => e.with_note(span, "declared here".to_string()),
                None => e,
            }
        };
        let own = &got.params[d.params.len()..];
        let theirs = &want.scheme.params[1..];
        if own.len() != theirs.len() {
            return Err(mismatch(format!(
                "it has {} but should have {}",
                plural(own.len(), "type parameter"),
                theirs.len()
            )));
        }
        // The trait's, at `Self` = the impl's type and with this one's
        // type parameter names.
        let mut args = vec![d.self_ty.clone()];
        args.extend(own.iter().map(|p| crate::types::param(p)));
        let s = Subst::new(&want.scheme.params, &args);
        // `Self::Item` in the trait's is what this impl says it is, with
        // what its bounds say, which decide its effect parameters.
        let mut proj = self.adts.proj.clone();
        proj.local = d.eqs.clone();
        let wanted = proj.normalise(&s.apply(&want.scheme.ty));
        let (Type::Fn(g), Type::Fn(w)) = (&got.ty, &wanted) else {
            unreachable!("methods have function types")
        };
        if g.params.len() != w.params.len() {
            return Err(mismatch(format!(
                "it takes {} but should take {}",
                plural(g.params.len(), "parameter"),
                w.params.len()
            )));
        }
        let fixed = &d.lifetimes;
        let mut a = Canon::new(fixed);
        let mut b = Canon::new(fixed);
        for (i, (x, y)) in g.params.iter().zip(&w.params).enumerate() {
            if a.ty(x) != b.ty(y) {
                let which = match (i, want.receiver) {
                    (0, true) => "its receiver".to_string(),
                    (i, true) => format!("parameter {i}"),
                    (i, false) => format!("parameter {}", i + 1),
                };
                return Err(mismatch(format!("{which} should be `{y}`, not `{x}`")));
            }
        }
        if a.ty(&g.ret) != b.ty(&w.ret) {
            return Err(mismatch(format!(
                "it should return `{}`, not `{}`",
                w.ret, g.ret
            )));
        }
        // Its effects, at most the trait's: callers going by the trait's
        // signature may count on its calls having no others.
        let more = Type::effects(
            (g.effects.effect_items().iter())
                .filter(|e| !w.effects.effect_items().contains(e))
                .cloned(),
        );
        if !more.is_no_effects() {
            let e = Error::ty(
                f.name.span,
                format!(
                    "`{0}` {1}, but `{2}`'s `{0}` can't, so its impls can't either",
                    f.name.name,
                    crate::check::effects::does(&more),
                    crate::check::source_name(tr),
                ),
            );
            // Unless what's more is the impl's own, the trait's can say it;
            // or rather the impl's associated effect, if the trait says one.
            let own = more
                .effect_items()
                .iter()
                .all(|e| matches!(e, Type::Alloc | Type::Panic | Type::Unsafe))
                && !(want.effects_at.iter())
                    .flat_map(|(_, w)| w)
                    .any(|w| w.starts_with("Self::"));
            return Err(match (&want.effects_at, own) {
                (Some((span, written)), true) => {
                    let mut items = written.clone();
                    items.extend(more.effect_items().iter().map(Type::to_string));
                    let text = crate::check::effects::set_text(&items);
                    e.with_fix(Fix::new(
                        format!("say `with {text}` in `{}`", crate::check::source_name(tr)),
                        vec![Edit {
                            span: *span,
                            text: format!(" with {text}"),
                        }],
                    ))
                }
                _ => e,
            });
        }
        // Its own lifetime bounds, the same as the trait's.
        let show = |bounds: &[Bound], c: &mut Canon| -> Vec<String> {
            let mut v: Vec<String> = bounds
                .iter()
                .map(|b| match b {
                    Bound::Region(x, y) => format!("{}: {}", c.region(x), c.region(y)),
                    Bound::Param(p, y) => {
                        let p = s.apply(&Type::Param(p.clone()));
                        format!("{p}: {}", c.region(y))
                    }
                    Bound::Proj(p, y) => format!("{}: {}", s.apply(p), c.region(y)),
                })
                .collect();
            v.sort();
            v
        };
        let gb = show(&got.bounds[d.bounds.len()..], &mut a);
        let wb = show(&want.scheme.bounds, &mut b);
        if gb != wb {
            return Err(mismatch(format!(
                "its lifetime bounds should be {}",
                match wb.is_empty() {
                    true => "none".to_string(),
                    false => format!("`{}`", wb.join(", ")),
                }
            )));
        }
        // Its own trait bounds, which can't be more than the trait's.
        let mut gp: Vec<String> = got.preds[d.preds.len()..]
            .iter()
            .map(|p| p.to_string())
            .collect();
        let mut wp: Vec<String> = want.scheme.preds[1..]
            .iter()
            .map(|p| Pred {
                ty: self.adts.normalise(&s.apply(&p.ty)),
                trait_name: p.trait_name.clone(),
            })
            .map(|p| p.to_string())
            .collect();
        gp.sort();
        wp.sort();
        if gp != wp {
            return Err(mismatch(format!(
                "its bounds should be {}",
                match wp.is_empty() {
                    true => "none".to_string(),
                    false => format!("`{}`", wp.join(", ")),
                }
            )));
        }
        Ok(())
    }

    /// No two impls of a trait can be for the same type, whatever their
    /// type parameters are: then which one a use means would be a choice.
    /// Their bounds don't count. `impl<T: Show> Print for T` and `impl
    /// Print for int` overlap even if `int` isn't `Show`, because someone
    /// might make it so, and then an impl far away would change meaning.
    ///
    /// Inherent impls can overlap, as long as they don't both define a
    /// method of the same name.
    fn coherence(&self) -> Result<(), Error> {
        let impls = &self.traits.impls;
        for (i, a) in impls.iter().enumerate() {
            for b in &impls[..i] {
                if a.trait_name != b.trait_name {
                    continue;
                }
                // `b`'s parameters, renamed apart from `a`'s.
                let renamed: Vec<Type> = b
                    .params
                    .iter()
                    .map(|p| Type::Param(format!("{p}'")))
                    .collect();
                let b_ty = Subst::new(&b.params, &renamed).apply(&b.self_ty);
                let mut params = a.params.clone();
                params.extend(b.params.iter().map(|p| format!("{p}'")));
                if !overlap(&a.self_ty, &b_ty, &params) {
                    continue;
                }
                let Some(tr) = &a.trait_name else {
                    if let Some((m, _)) = a
                        .methods
                        .iter()
                        .find(|(m, _)| b.methods.iter().any(|(n, _)| n == m))
                    {
                        return Err(Error::ty(
                            a.span,
                            format!(
                                "method `{m}` is defined for `{}` here, and for `{}` in another \
                                 impl, and those can be the same type",
                                a.self_ty, b.self_ty
                            ),
                        )
                        .with_note(b.span, "the other impl".to_string()));
                    }
                    continue;
                };
                // Point at the one written, rather than a derive.
                let (a, b) = match a.derived && !b.derived {
                    true => (b, a),
                    false => (a, b),
                };
                let what = |d: &ImplDef| match d.derived {
                    true => format!("the `#[derive({tr})]`"),
                    false => format!("`impl {}`", d.header()),
                };
                let same = a.self_ty.erased() == b.self_ty.erased();
                let message = match same {
                    true => format!(
                        "`{}` implements `{tr}` twice: here and in {}",
                        a.self_ty,
                        what(b)
                    ),
                    false => format!(
                        "this impl of `{tr}` overlaps {}: a type can be both `{}` and `{}`, and \
                         then which impl it uses would be a choice",
                        what(b),
                        a.self_ty,
                        b.self_ty
                    ),
                };
                let note = "the other impl".to_string();
                return Err(Error::ty(a.span, message).with_note(b.span, note));
            }
        }
        Ok(())
    }
}

/// A use in `assoc_regular`: from what, to what, with what arguments, what
/// of `to`'s parameters is inside `from`'s, and where.
type Edge<'a> = (String, &'a str, Vec<Type>, Vec<(usize, usize, usize)>, Span);

/// The type arguments of struct or enum type `t`.
fn args_of(t: &Type) -> Vec<Type> {
    match t {
        Type::Adt(_, _, args) => args.clone(),
        _ => Vec::new(),
    }
}

/// Renames the lifetimes a method has of its own to `'0`, `'1`, .. in the
/// order they come up, for comparing two signatures up to their names.
/// Those in `fixed`, the impl's, keep theirs.
struct Canon<'a> {
    fixed: &'a [String],
    seen: Vec<Region>,
}

impl<'a> Canon<'a> {
    fn new(fixed: &'a [String]) -> Canon<'a> {
        Canon {
            fixed,
            seen: Vec::new(),
        }
    }

    fn region(&mut self, r: &Region) -> Region {
        match r {
            Region::Named(n) if self.fixed.contains(n) => r.clone(),
            Region::Named(n) if n.starts_with("'^") => r.clone(),
            Region::Named(_) | Region::Result(..) => {
                let i = match self.seen.iter().position(|s| s == r) {
                    Some(i) => i,
                    None => {
                        self.seen.push(r.clone());
                        self.seen.len() - 1
                    }
                };
                Region::Named(format!("'#{i}"))
            }
            r => r.clone(),
        }
    }

    fn ty(&mut self, t: &Type) -> Type {
        t.map_regions(&mut |r| self.region(r))
    }
}

/// A projection in `t` on a type that isn't a parameter, or a projection
/// on one: one with no impl to work it out by.
fn rigid_on_type(t: &Type) -> Option<Type> {
    match t {
        Type::Proj(s, ..) if !matches!(**s, Type::Param(_) | Type::Proj(..)) => Some(t.clone()),
        t => t.parts().find_map(rigid_on_type),
    }
}

/// The name of the associated type of the projection in `t` on `s` of
/// trait `tr`.
fn proj_name(t: &Type, s: &Type, tr: &str) -> Option<String> {
    match t {
        Type::Proj(x, y, name) if **x == *s && y == tr => Some(name.clone()),
        t => t.parts().find_map(|p| proj_name(p, s, tr)),
    }
}
