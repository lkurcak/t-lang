//! Collecting structs and enums: their fields and variants, and the
//! rules each declaration must keep.

use super::generics::{self, AliasDef, Kind, Lifetimes, Use, bare_projection};
use super::{Checker, Scoped, source_name};
use crate::ast::*;
use crate::error::Error;
use crate::types::{AdtDef, AdtKind, Adts, Type, all_fields};
use std::collections::{HashMap, HashSet};

impl Checker {
    /// Pass 0: builds the table of structs and enums, local ones too, and
    /// fills in `arities`. Rejects duplicate names, fields, variants and
    /// type parameters, type parameters no field uses, types that contain
    /// themselves, and polymorphic recursion.
    ///
    /// Those of the packages before are known already. Theirs can't
    /// mention this one's, so a cycle of types is in one package.
    pub(super) fn collect_adts(&mut self, prog: &Program) -> Result<(), Error> {
        let from = self.from.all_types;
        let decls: Vec<_> = prog.all_types().into_iter().skip(from).collect();
        // All names first, so declarations can mention each other in any
        // order.
        let mut declared = HashSet::new();
        for ((d, _), def) in decls.iter().zip(&self.types[from..]) {
            let name = d.name();
            if crate::types::is_builtin_type(&name.name) || name.name == "Self" {
                return Err(Error::ty(
                    name.span,
                    format!("`{}` is a built-in type", name.name),
                ));
            }
            if !declared.insert((def.block, d.module(), &name.name)) {
                return Err(Error::ty(
                    name.span,
                    format!("type `{}` is already defined", name.name),
                ));
            }
            // What a struct or enum holds has types, and a function type
            // among them says its effects itself.
            if let Some(e) = d.type_params().iter().find(|p| d.generics().is_effect(p)) {
                let what = match d {
                    TypeDecl::Struct(_) => "a struct",
                    TypeDecl::Enum(_) => "an enum",
                    TypeDecl::Alias(_) => {
                        return Err(Error::ty(
                            e.span,
                            format!(
                                "a type alias can't have effect parameters, as `{}` would be: \
                                 it takes lifetime, type and const parameters",
                                e.name
                            ),
                        ));
                    }
                };
                return Err(Error::ty(
                    e.span,
                    format!(
                        "{what} can't have effect parameters, as `{}` would be: what it holds \
                         has types, and a function type among them says its effects itself",
                        e.name
                    ),
                ));
            }
            if let Some(at) = def.intrinsic {
                if !self.modules.in_library(d.module()) {
                    return Err(super::not_library(at));
                }
                if !super::INTRINSIC_TYPES.contains(&def.symbol.as_str()) {
                    let message = format!("there's no intrinsic type `{}`", def.symbol);
                    return Err(Error::ty(name.span, message));
                }
            }
            if let TypeDecl::Alias(a) = d {
                let alias = std::sync::Arc::new(AliasDef::new(a));
                self.aliases.insert(def.symbol.clone(), alias);
                continue;
            }
            let arity = (d.generics().lifetimes.len(), d.generics().lengths());
            self.arities.insert(def.symbol.clone(), arity);
        }
        self.alias_cycles(&decls)?;

        // No projection's known yet: `projections_known` sees to them.
        self.adts.proj = Default::default();
        let mut adts = Adts::new();
        // Every field's type, with the symbol of the type it's in and where
        // it was written, for the recursion checks.
        let mut field_types = Vec::new();
        let symbols: Vec<String> = self.types[from..]
            .iter()
            .map(|t| t.symbol.clone())
            .collect();
        for ((d, _), symbol) in decls.iter().zip(&symbols) {
            if let TypeDecl::Alias(_) = d {
                continue;
            }
            let params = generics::declare(d.generics())?;
            let lifetimes = generics::declare_lifetimes(&d.generics().lifetimes)?;
            let bounds = generics::bounds(d.generics(), &lifetimes, &params)?;
            let (preds, eqs) = self.preds(d.generics(), &params, &lifetimes, false)?;
            self.env = preds.clone();
            self.adts.proj.local = eqs.clone();
            let mut lts = Lifetimes::Fields {
                declared: &lifetimes,
            };
            let kind = match d {
                TypeDecl::Struct(s) => {
                    let mut fields: Vec<(String, Type)> = Vec::new();
                    for f in &s.fields {
                        if fields.iter().any(|(n, _)| *n == f.name.name) {
                            return Err(Error::ty(
                                f.name.span,
                                format!("field `{}` is declared twice", f.name.name),
                            ));
                        }
                        // A refinement is checked once every type is known.
                        let t = self.resolve_with(f.ty.unrefined(), &params, &mut lts)?;
                        field_types.push((symbol.as_str(), f.ty.span, t.clone()));
                        fields.push((f.name.name.clone(), t));
                    }
                    AdtKind::Struct(fields)
                }
                TypeDecl::Enum(e) => {
                    let mut variants: Vec<(String, Vec<Type>)> = Vec::new();
                    for v in &e.variants {
                        if variants.iter().any(|(n, _)| *n == v.name.name) {
                            return Err(Error::ty(
                                v.name.span,
                                format!("variant `{}` is declared twice", v.name.name),
                            ));
                        }
                        let mut tys = Vec::new();
                        for f in &v.fields {
                            let t = self.resolve_with(f, &params, &mut lts)?;
                            field_types.push((symbol.as_str(), f.span, t.clone()));
                            tys.push(t);
                        }
                        variants.push((v.name.name.clone(), tys));
                    }
                    AdtKind::Enum(variants)
                }
                TypeDecl::Alias(_) => unreachable!("skipped above"),
            };
            let adt = AdtDef {
                params,
                lifetimes,
                bounds,
                preds,
                eqs,
                kind,
            };
            adts.insert(symbol.clone(), adt);
        }
        self.env.clear();
        self.adts.proj.local.clear();
        self.adts.extend(adts.drain());
        let adts = &self.adts;

        // `struct S<T> { f: fn(S<Option<T>>) -> i64 }` would need `S<i64>`,
        // so `S<Option<i64>>`, so `S<Option<Option<i64>>>`...
        let mut uses = Vec::new();
        for (owner, span, t) in &field_types {
            each_adt(t, &mut |name, args| {
                uses.push(Use {
                    from: owner,
                    to: name,
                    args: args.to_vec(),
                    inside: Vec::new(),
                    span: *span,
                })
            });
        }
        let params: HashMap<&str, &[String]> = adts
            .iter()
            .map(|(name, def)| (name.as_str(), def.params.as_slice()))
            .collect();
        generics::polymorphic_recursion(&uses, &params, &|_| true)?;

        // A struct that holds itself, even via other types, would need
        // infinite memory: `struct A { a: A }` has the size of an int
        // plus... itself. It can hold itself through a pointer, which has a
        // size of its own: `Box<A>` or `&A`.
        for (owner, span, t) in &field_types {
            if contains(adts, t, owner, &mut HashSet::new()) {
                let name = source_name(owner);
                return Err(Error::ty(
                    *span,
                    format!(
                        "`{name}` contains itself, so it would have infinite size; hold it \
                         through a pointer, like `Box<{name}>`, instead"
                    ),
                ));
            }
        }
        for ((d, _), symbol) in decls.iter().zip(&symbols) {
            if let TypeDecl::Struct(s) = d {
                self.field_refinements(s, symbol)?;
            }
        }
        Ok(())
    }
}

impl Checker {
    /// Rejects a type alias that stands for a type with itself in it, even
    /// through other aliases: expanding it would never end (rule R1 of
    /// docs/25-type-aliases.md). Those of the packages before can't mention
    /// this one's, so a cycle is in one package. After one, this package's
    /// aliases are forgotten, so what's done past the error (`type_notes`)
    /// never expands one.
    fn alias_cycles(&mut self, decls: &[(&TypeDecl, Option<usize>)]) -> Result<(), Error> {
        let Err(e) = self.no_alias_cycles(decls) else {
            return Ok(());
        };
        for def in &self.types[self.from.all_types..] {
            self.aliases.remove(&def.symbol);
        }
        Err(e)
    }

    fn no_alias_cycles(&self, decls: &[(&TypeDecl, Option<usize>)]) -> Result<(), Error> {
        // The aliases each one's body names, by symbol.
        let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut order = Vec::new();
        for ((d, _), def) in decls.iter().zip(&self.types[self.from.all_types..]) {
            let TypeDecl::Alias(a) = d else { continue };
            let mut named = Vec::new();
            a.ty.each_named(&mut |t| {
                if let Some(Scoped::Adt(symbol)) = self.type_names.get(&t.id)
                    && self.aliases.contains_key(symbol)
                {
                    named.push(symbol.as_str());
                }
            });
            edges.insert(def.symbol.as_str(), named);
            order.push((def.symbol.as_str(), &a.name));
        }
        // Depth first from each, in the order they're declared.
        fn path<'a>(
            at: &'a str,
            to: &str,
            edges: &HashMap<&str, Vec<&'a str>>,
            seen: &mut HashSet<&'a str>,
            out: &mut Vec<&'a str>,
        ) -> bool {
            for &next in edges.get(at).into_iter().flatten() {
                if next == to {
                    return true;
                }
                if seen.insert(next) {
                    out.push(next);
                    if path(next, to, edges, seen, out) {
                        return true;
                    }
                    out.pop();
                }
            }
            false
        }
        for (symbol, name) in order {
            let mut through = Vec::new();
            if !path(symbol, symbol, &edges, &mut HashSet::new(), &mut through) {
                continue;
            }
            let through = match through[..] {
                [] => String::new(),
                _ => {
                    let names: Vec<String> = (through.iter())
                        .map(|s| format!("`{}`", source_name(s)))
                        .collect();
                    format!(", through {}", names.join(", "))
                }
            };
            return Err(Error::ty(
                name.span,
                format!(
                    "type alias `{}` stands for a type that contains itself{through}; a \
                     recursive type needs a struct or enum",
                    name.name
                ),
            ));
        }
        Ok(())
    }

    /// Checks what each type alias stands for, once every type and trait
    /// is known: that it's a type, with a size, whose every name means
    /// something where it's declared, which uses every parameter and
    /// leaves out no lifetime (rules R3, R6 of docs/25-type-aliases.md).
    pub(super) fn check_aliases(&self, prog: &Program) -> Result<(), Error> {
        let decls = prog.all_types().into_iter().skip(self.from.all_types);
        for ((d, _), def) in decls.zip(&self.types[self.from.all_types..]) {
            let TypeDecl::Alias(a) = d else { continue };
            let body = &a.ty;
            let is_str = matches!(&body.kind, TypeExprKind::Named(n, args)
                if n == "str" && args.is_empty() && self.type_name(body).is_none());
            if matches!(body.kind, TypeExprKind::Slice(_)) || is_str {
                return Err(Error::ty(
                    body.span,
                    format!(
                        "a type alias can't stand for `{body}`, which has no size; alias a \
                         reference to it instead"
                    ),
                ));
            }
            let ty = self.alias_body(&self.aliases[&def.symbol])?;
            let mut used: HashSet<&str> = HashSet::new();
            fn params<'t>(t: &'t Type, used: &mut HashSet<&'t str>) {
                if let Type::Param(p) = t {
                    used.insert(p);
                }
                t.parts().for_each(|p| params(p, used));
            }
            params(&ty, &mut used);
            for r in ty.regions() {
                if let crate::types::Region::Named(l) = r {
                    used.insert(l);
                }
            }
            let g = &a.generics;
            if let Some(p) = g
                .lifetimes
                .iter()
                .chain(&g.types)
                .find(|p| !used.contains(p.name.as_str()))
            {
                return Err(Error::ty(
                    p.span,
                    format!(
                        "type alias `{}` doesn't use its parameter `{}`",
                        a.name.name, p.name
                    ),
                ));
            }
        }
        Ok(())
    }

    /// What alias `def` stands for, with its parameters as they are in it:
    /// a projection on one is left as it is, as the bounds that would say
    /// it means something are checked where it's used.
    pub(super) fn alias_body(&self, def: &AliasDef) -> Result<Type, Error> {
        let proj = |s: Type, tr: Option<&Ident>, name: &Ident, t: &TypeExpr, _: Kind| {
            let Some(tr) = tr else {
                return Err(bare_projection(t.span));
            };
            let Some(d) = self.traits.traits.get(&tr.name) else {
                return Err(Error::ty(tr.span, format!("unknown trait `{}`", tr.name)));
            };
            let owner = self.traits.assoc_owner(&tr.name, &name.name);
            match owner.first() {
                Some(o) if d.assoc.iter().any(|a| a.name == name.name) || owner.len() == 1 => {
                    let p = Type::Proj(Box::new(s), o.clone(), name.name.clone());
                    // As it is wherever it's written.
                    self.proj_traits.borrow_mut().insert(t.id, o.clone());
                    Ok(p)
                }
                _ => Err(Error::ty(
                    name.span,
                    format!("trait `{}` has no associated type `{}`", tr.name, name.name),
                )),
            }
        };
        let mut lts = Lifetimes::Alias {
            name: &def.name,
            declared: &def.lifetimes,
            args: None,
        };
        generics::resolve(
            &def.body,
            &|_, t| self.type_name(t),
            &proj,
            &def.params(),
            &mut lts,
        )
    }
}

/// Calls `f` with the name and type arguments of every struct and enum in
/// `t`, at any depth.
pub(super) fn each_adt<'t>(t: &'t Type, f: &mut impl FnMut(&'t str, &'t [Type])) {
    if let Type::Adt(name, _, args) = t {
        f(name, args);
    }
    for p in t.parts() {
        each_adt(p, f);
    }
}

/// Does a value of type `ty` hold a `target`, at any depth? Function types
/// don't count: a function value is a code address, not the values its
/// signature mentions. Nor do references and boxes, which hold an address.
/// `struct A { b: Wrap<A> }` counts, through `Wrap`'s field of type `T`.
///
/// This terminates because polymorphic recursion is ruled out first, so only
/// finitely many types can turn up in here.
fn contains(adts: &Adts, ty: &Type, target: &str, seen: &mut HashSet<Type>) -> bool {
    let Type::Adt(name, ..) = ty else {
        return false;
    };
    if name == target {
        return true;
    }
    if !seen.insert(ty.clone()) {
        return false;
    }
    all_fields(adts, ty)
        .iter()
        .any(|t| contains(adts, t, target, seen))
}

/// Adds the name of every struct and enum that `t` mentions to `out`.
pub(super) fn adt_names(t: &Type, out: &mut HashSet<String>) {
    each_adt(t, &mut |name, _| {
        out.insert(name.to_string());
    });
}
