//! Collecting structs and enums: their fields and variants, and the
//! rules each declaration must keep.

use super::generics::{self, Lifetimes, Use};
use super::{Checker, source_name};
use crate::ast::*;
use crate::error::Error;
use crate::types::{AdtDef, AdtKind, Adts, Type, all_fields};
use std::collections::{HashMap, HashSet};

impl Checker {
    /// Pass 0: builds the table of structs and enums, local ones too, and
    /// fills in `arities`. Rejects duplicate names, fields, variants and
    /// type parameters, type parameters no field uses, types that contain
    /// themselves, and polymorphic recursion.
    pub(super) fn collect_adts(&mut self, prog: &Program) -> Result<(), Error> {
        let decls = prog.all_types();
        // All names first, so declarations can mention each other in any
        // order.
        let mut declared = HashSet::new();
        for ((d, _), def) in decls.iter().zip(&self.types) {
            let name = d.name();
            if matches!(name.name.as_str(), "int" | "bool" | "Box" | "Self") {
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
            if let Some(e) = d
                .type_params()
                .iter()
                .find(|p| crate::parser::is_effect_name(&p.name))
            {
                let what = match d {
                    TypeDecl::Struct(_) => "a struct",
                    _ => "an enum",
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
            let arity = (d.generics().lifetimes.len(), d.type_params().len());
            self.arities.insert(def.symbol.clone(), arity);
        }

        let mut adts = Adts::new();
        adts.proj = std::mem::take(&mut self.adts.proj);
        // Every field's type, with the symbol of the type it's in and where
        // it was written, for the recursion checks.
        let mut field_types = Vec::new();
        let symbols: Vec<String> = self.types.iter().map(|t| t.symbol.clone()).collect();
        for ((d, _), symbol) in decls.iter().zip(&symbols) {
            let params = generics::declare(d.type_params())?;
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
                        let t = self.resolve_with(&f.ty, &params, &mut lts)?;
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

        // `struct S<T> { f: fn(S<Option<T>>) -> int }` would need `S<int>`,
        // so `S<Option<int>>`, so `S<Option<Option<int>>>`...
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
        generics::polymorphic_recursion(&uses, &params)?;

        // A struct that holds itself, even via other types, would need
        // infinite memory: `struct A { a: A }` has the size of an int
        // plus... itself. It can hold itself through a pointer, which has a
        // size of its own: `Box<A>` or `&A`.
        for (owner, span, t) in &field_types {
            if contains(&adts, t, owner, &mut HashSet::new()) {
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
        self.env.clear();
        adts.proj = std::mem::take(&mut self.adts.proj);
        adts.proj.local.clear();
        self.adts = adts;
        Ok(())
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
