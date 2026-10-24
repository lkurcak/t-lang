//! Variables and paths in expressions: what each refers to, its type
//! there, instantiated, and struct literals, variants and `Box::new`.

use super::generics::{self, Lifetimes};
use super::info::{Instance, NoteKind, Resolved};
use super::scope::{Binding, FnId};
use super::typed::Res;
use super::{Checker, builtin_scheme, modules, plural, traits};
use crate::ast::*;
use crate::error::{Error, Span};
use crate::traits::Pred;
use crate::types::{AdtKind, Region, Subst, Type, variant_fields};

impl Checker {
    /// Finds a variable's type, and notes that it was used and what it
    /// refers to. A generic function is instantiated: its type parameters
    /// are replaced by the type arguments `targs`, if any were written, or
    /// else by a fresh variable each. So each use is separate, and `id(1)`
    /// and `id(true)` can both be fine.
    pub(super) fn lookup(
        &mut self,
        name: &str,
        targs: &[TypeExpr],
        span: Span,
        id: NodeId,
    ) -> Result<Option<(Type, Res)>, Error> {
        // Notes are for the name, not any `::<..>` after it.
        let at = Span::new(span.start, span.start + name.len());
        // A path, `a::f`, names what resolving it said.
        let binding = match name.contains("::") {
            true => match self.modules.paths.get(&id) {
                Some(modules::Item::Fn(i)) => Some(Binding::Fn(FnId(*i))),
                _ => None,
            },
            false => self.binding(name),
        };
        let (scheme, def, symbol, builtin) = match binding {
            None => return Ok(None),
            Some(Binding::Local(i)) => {
                if i < self.barrier {
                    return Err(self.capture(i, span));
                }
                if !targs.is_empty() {
                    return Err(Error::ty(
                        span,
                        format!(
                            "`{name}` is a variable, and only functions, structs and enums take \
                             type arguments"
                        ),
                    ));
                }
                let local = &mut self.locals[i];
                local.used = true;
                let (ty, def, id) = (local.ty.clone(), local.def(), local.id());
                self.note(
                    at,
                    NoteKind::Var(name.to_string()),
                    Some(ty.clone()),
                    Some(def),
                );
                return Ok(Some((ty, Res::Local(id))));
            }
            Some(Binding::Fn(id)) => {
                self.uses().fns.insert(id);
                let f = &self.fns[id.0];
                let scheme = f.scheme.clone();
                (scheme, Some(f.name.span), Some(f.symbol.clone()), None)
            }
            Some(Binding::Builtin(b)) => (builtin_scheme(b), None, None, Some(b)),
        };
        let args = self.type_args(name, &scheme.params, targs, span, "(..)")?;
        let s = Subst::new(&scheme.params, &args);
        let ty = s.apply(&scheme.ty);
        // What it needs of its type arguments, by its bounds.
        for p in &scheme.preds {
            let pred = Pred {
                ty: self.adts.normalise(&s.apply(&p.ty)),
                trait_name: p.trait_name.clone(),
            };
            self.oblige(
                pred,
                at,
                traits::Why::Bound(name.to_string(), p.ty.to_string()),
            )?;
        }
        for (lhs, rhs) in &scheme.eqs {
            self.require_eq(s.apply(lhs), s.apply(rhs), at, name)?;
        }
        let res = match (&symbol, builtin) {
            (Some(symbol), _) => Res::Fn(symbol.clone(), args.clone()),
            (None, Some(b)) => Res::Builtin(b, args.clone()),
            (None, None) => unreachable!("a function or a built-in"),
        };
        let instance = symbol.map(|symbol| {
            self.info.symbols.insert(id, symbol.clone());
            (id, span, Instance::Fn(symbol, args))
        });
        let kind = NoteKind::Fn(name.to_string());
        self.note_use(at, kind, ty.clone(), def, instance, targs.is_empty());
        Ok(Some((ty, res)))
    }

    /// The type arguments for a use at `span` of `name`, which has type
    /// parameters `params`: those written in `targs`, or a fresh variable
    /// for each if none were. `rest` is what follows `name::<..>` in an
    /// example of writing them, for when they can't be inferred.
    pub(super) fn type_args(
        &mut self,
        name: &str,
        params: &[String],
        targs: &[TypeExpr],
        span: Span,
        rest: &str,
    ) -> Result<Vec<Type>, Error> {
        if targs.is_empty() {
            let example = vec!["i64"; params.len()].join(", ");
            return Ok(params
                .iter()
                .map(|p| {
                    self.fresh_param(
                        p,
                        span,
                        format!("`{p}` for this use of `{name}`"),
                        format!(
                            "nothing here says what it is, so write it, as in \
                             `{name}::<{example}>{rest}`, or annotate where the result goes"
                        ),
                    )
                })
                .collect());
        }
        if targs.len() != params.len() {
            return Err(generics::arity_error(
                &format!("`{name}`"),
                params.len(),
                targs.len(),
                span,
            ));
        }
        let mut args = Vec::new();
        for (p, t) in params.iter().zip(targs) {
            args.push(match crate::parser::is_effect_name(p) {
                true if matches!(&t.kind, TypeExprKind::Named(n, a) if n == "_" && a.is_empty()) => {
                    self.fresh_effects(t.span, format!("`{p}` for this use of `{name}`"))
                }
                true => {
                    return Err(Error::ty(
                        t.span,
                        format!(
                            "`{p}` is an effect parameter, which can't be written here yet; \
                             write `_` for it"
                        ),
                    ));
                }
                false => self.type_arg(t)?,
            });
        }
        Ok(args)
    }

    /// A type argument written in `::<..>`, where `_` may stand for any part
    /// of it, to be inferred: `pair::<i64, _>`, `id::<Option<_>>`.
    ///
    /// `resolve` can't make type variables, so each `_` is renamed to a
    /// type parameter no source program can name, resolved like any other,
    /// and then replaced by a fresh variable.
    fn type_arg(&mut self, t: &TypeExpr) -> Result<Type, Error> {
        let mut holes = Vec::new();
        let t = generics::name_holes(t, &mut holes);
        let mut params = self.type_params.clone();
        params.extend(holes.iter().map(|(name, _)| name.clone()));
        let mut lts = Lifetimes::Body {
            declared: &self.lifetimes,
        };
        let ty = self.resolve_with(&t, &params, &mut lts)?;
        let (names, vars): (Vec<String>, Vec<Type>) = holes
            .into_iter()
            .map(|(name, span)| {
                let what = "the type `_` stands for here".to_string();
                let var = self.fresh(span, what, "write it out instead".into());
                (name, var)
            })
            .unzip();
        Ok(Subst::new(&names, &vars).apply(&ty))
    }

    /// The type a struct literal or pattern names, which must be a struct.
    /// If it's generic, each type parameter gets a fresh variable, to be
    /// worked out from the fields or from where it's used.
    pub(super) fn struct_type(&mut self, name: &Ident, targs: &[TypeExpr]) -> Result<Type, Error> {
        let def = self.adt_symbol(name.id).and_then(|s| self.adts.get(s));
        match def.map(|d| &d.kind) {
            Some(AdtKind::Struct(_)) => self.instantiate(name, targs, " { .. }", NoteKind::Struct),
            Some(AdtKind::Enum(_)) => Err(Error::ty(
                name.span,
                format!("`{}` is an enum, not a struct", name.name),
            )),
            None => Err(Error::ty(
                name.span,
                format!("unknown type `{}`", name.name),
            )),
        }
    }

    /// `name<A1, .., Ak>`, for struct or enum `name`, which must be one,
    /// with the type arguments `targs` or else a fresh variable for each
    /// type parameter, noted as a use of it at `name`'s span. `rest`
    /// follows `name::<..>` in an example of writing them.
    fn instantiate(
        &mut self,
        name: &Ident,
        targs: &[TypeExpr],
        rest: &str,
        kind: fn(String) -> NoteKind,
    ) -> Result<Type, Error> {
        let symbol = self.adt_symbol(name.id).expect("a struct or enum");
        let symbol = symbol.to_string();
        let params = self.adts[&symbol].params.clone();
        let args = self.type_args(&name.name, &params, targs, name.span, rest)?;
        let lifetimes = self.adts[&symbol].lifetimes.len();
        let ty = Type::Adt(
            symbol.clone(),
            vec![Region::Erased; lifetimes],
            args.clone(),
        );
        self.well_formed(&ty, name.span)?;
        let def = self.defs.types.get(&symbol).copied();
        let instance = Instance::Adt(symbol, args);
        let kind = kind(name.name.clone());
        self.note_use(
            name.span,
            kind,
            ty.clone(),
            def,
            Some((name.id, name.span, instance)),
            targs.is_empty(),
        );
        Ok(ty)
    }

    /// Looks up `E::V`, returning the enum's type, instantiated as in
    /// `instantiate`, and the variant's index.
    pub(super) fn variant(
        &mut self,
        ty: &Ident,
        targs: &[TypeExpr],
        variant: &Ident,
    ) -> Result<(Type, usize), Error> {
        let symbol = self.adt_symbol(ty.id).map(str::to_string);
        let def = symbol.as_ref().and_then(|s| self.adts.get(s));
        if def.is_none() && ty.name == "Box" {
            return Err(Error::ty(
                ty.span,
                "`Box` isn't an enum: `Box::new(x)` makes one, and `*b` is what box `b` holds",
            ));
        }
        let variants = match def.map(|d| &d.kind) {
            Some(AdtKind::Enum(variants)) => variants,
            Some(AdtKind::Struct(_)) => {
                return Err(Error::ty(
                    ty.span,
                    format!("`{}` is a struct, not an enum", ty.name),
                ));
            }
            None => return Err(Error::ty(ty.span, format!("unknown type `{}`", ty.name))),
        };
        let Some(i) = variants.iter().position(|(v, _)| *v == variant.name) else {
            return Err(Error::ty(
                variant.span,
                format!("`{}` has no variant `{}`", ty.name, variant.name),
            ));
        };
        let rest = format!("::{}", variant.name);
        let t = self.instantiate(ty, targs, &rest, NoteKind::Enum)?;
        let fields = variant_fields(&self.adts, &t, i);
        let as_fn = match fields.is_empty() {
            true => t.clone(),
            false => Type::func(fields, t.clone()),
        };
        let key = (symbol.expect("an enum"), variant.name.clone());
        let def = self.defs.variants.get(&key).copied();
        let kind = NoteKind::Variant(ty.name.clone(), variant.name.clone());
        self.note(variant.span, kind, Some(as_fn), def);
        Ok((t, i))
    }

    /// Notes a use of field `field` of struct `s`, of type `ty`.
    pub(super) fn note_field(&mut self, s: &str, field: &Ident, ty: &Type) {
        let def = self.defs.fields.get(&(s.to_string(), field.name.clone()));
        let kind = NoteKind::Field(field.name.clone());
        self.note(field.span, kind, Some(ty.clone()), def.copied());
    }

    /// Notes the use of method `found`, named at `name`, for editors and for
    /// monomorphisation. Its type arguments were `inferred` unless they
    /// were written.
    pub(super) fn resolved(&mut self, name: &Ident, found: &traits::Found, inferred: bool) {
        let instance = Instance::Fn(found.symbol.clone(), found.args.clone());
        let kind = NoteKind::Fn(name.name.clone());
        let ty = found.ty.clone();
        self.note_use(
            name.span,
            kind,
            ty,
            found.def,
            Some((name.id, name.span, instance)),
            inferred,
        );
    }

    /// Is `ty::name` a variant of an enum, rather than a path to a method?
    pub(super) fn is_variant(&self, ty: &Ident, name: &Ident) -> bool {
        let def = self.adt_symbol(ty.id).and_then(|s| self.adts.get(s));
        match def.map(|d| &d.kind) {
            Some(AdtKind::Enum(variants)) => variants.iter().any(|(v, _)| *v == name.name),
            // Not a method either, but the variant rule says what's wrong.
            _ => def.is_none() && self.path_type(ty).is_none(),
        }
    }

    /// What `ty` in a path `ty::m` names, if it's a type: `Self`, a type
    /// parameter, an integer type, `bool` or `char`. Structs and enums are instantiated by
    /// `path`, as they may be generic.
    fn path_type(&self, ty: &Ident) -> Option<Type> {
        match ty.name.as_str() {
            "Self" if self.self_ty.is_some() => self.self_ty.clone(),
            n if crate::types::IntTy::named(n).is_some() => {
                crate::types::IntTy::named(n).map(Type::Int)
            }
            "bool" => Some(Type::Bool),
            "char" => Some(Type::Char),
            n if self.type_params.iter().any(|p| p == n) => Some(Type::Param(n.into())),
            n if self.traits.traits.contains_key(n) => Some(Type::Unit),
            _ => None,
        }
    }

    //   m is a method of T of type F
    //  ------------------------------ (T-Path)
    //          Γ ⊢ T::m : F
    //
    //   m is a method of trait Tr of type F, at Self = S    S: Tr
    //  ---------------------------------------------------------- (T-Path-Trait)
    //                    Γ ⊢ Tr::m : F
    //
    // A path names a method as a function, with its receiver, if it has
    // one, as its first parameter: `Point::len(&p)` is `p.len()`. It's
    // found as for a call, without going through references (see
    // `lookup_method`). In `Show::show(x)`, `Self` is worked out from how
    // it's used, here from `x`, as for any type parameter. With `(..)` it's
    // called (T-App).
    pub(super) fn path(
        &mut self,
        e: &Expr,
        ty: &Ident,
        targs: &[TypeExpr],
        name: &Ident,
        args: &[Expr],
        parens: bool,
    ) -> Result<Type, Error> {
        let found = match self.traits.traits.contains_key(&ty.name)
            && self.adt_symbol(ty.id).is_none()
            && !self.type_params.contains(&ty.name)
        {
            true => {
                let tr = self.trait_named(ty)?;
                if let Some(t) = targs.first() {
                    return Err(Error::ty(
                        t.span,
                        format!("trait `{tr}` takes no type arguments; its methods may"),
                    ));
                }
                let what = format!("`Self` for this use of `{tr}::{}`", name.name);
                let hint = "annotate the type of its argument".to_string();
                let s = self.fresh(name.span, what, hint);
                self.lookup_method(&s, Some(&tr), name, &[], "trait")?
            }
            false => {
                let t = match self.adt_symbol(ty.id) {
                    Some(_) => {
                        let rest = format!("::{}", name.name);
                        let kind = match self.adts[self.adt_symbol(ty.id).unwrap()].kind {
                            AdtKind::Struct(_) => NoteKind::Struct,
                            AdtKind::Enum(_) => NoteKind::Enum,
                        };
                        self.instantiate(ty, targs, &rest, kind)?
                    }
                    None => {
                        let t = self.path_type(ty).expect("checked by `is_variant`");
                        if let Some(t) = targs.first() {
                            return Err(Error::ty(
                                t.span,
                                format!("`{}` takes no type arguments", ty.name),
                            ));
                        }
                        t
                    }
                };
                let what = match self.adt_symbol(ty.id).map(|s| &self.adts[s].kind) {
                    Some(AdtKind::Enum(_)) => "enum",
                    _ => "type",
                };
                self.lookup_method(&t, None, name, &[], what)
                    .map_err(|err| match what {
                        "enum" => Error::ty(
                            name.span,
                            format!("`{}` has no variant or method `{}`", ty.name, name.name),
                        ),
                        _ => err,
                    })?
            }
        };
        self.resolved(name, &found, targs.is_empty());
        self.typed
            .vars
            .insert(e.id, Res::Fn(found.symbol.clone(), found.args.clone()));
        let resolved = Resolved {
            symbol: found.symbol.clone(),
            derefs: 0,
            autoref: None,
        };
        self.info.methods.insert(name.id, resolved);
        // What `Self` is may only be known from the arguments.
        // A call a `?` means (`modules::Rewrite::try_expr`) is the `?`'s.
        let why = match (self.is_try(name.span), name.name.as_str()) {
            (true, "branch") => traits::Why::TryBranch,
            (true, _) => traits::Why::TryReturn(self.returning_from()),
            (false, _) => traits::Why::Method(name.name.clone()),
        };
        let oblige = |cx: &mut Checker| match found.of_trait.clone() {
            Some(pred) => cx.oblige(pred, name.span, why.clone()),
            None => Ok(()),
        };
        if !parens {
            oblige(self)?;
            // Without `(..)`, it's this whole expression that names it.
            let instance = Instance::Fn(found.symbol.clone(), found.args.clone());
            self.pending.push((e.id, e.span, instance));
            return Ok(found.ty);
        }
        let Type::Fn(sig) = &found.ty else {
            unreachable!("a method has a function type")
        };
        if sig.params.len() != args.len() {
            return Err(Error::ty(
                e.span,
                format!(
                    "`{}::{}` takes {} but {} {} supplied",
                    ty.name,
                    name.name,
                    plural(sig.params.len(), "argument"),
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" },
                ),
            ));
        }
        for (arg, param) in args.iter().zip(&sig.params) {
            match self.is_try(name.span) && name.name == "from_residual" {
                // What a `?` returns early must be what the function's
                // result is made from: lemma Q2's equal residuals.
                // What's returned is the function's result, which `return`
                // already has: so it's known before its residual is, and
                // whether it's `Try` is said before how they differ.
                true => {
                    let ret = self.ret.clone().expect("a `?` is in a `return`");
                    // `Self` is still a variable here, so this can't clash.
                    let _ = self.unify(&sig.ret, &ret);
                    oblige(self)?;
                    let got = self.synth(arg)?;
                    let who = self.returning_from();
                    let ret = self.shown(&self.solved(&ret));
                    self.unify_or(&got, param, name.span, |got, want| {
                        format!(
                            "`?` can't return early here: {who} returns `{ret}`, so a `?` in it \
                             must stop with `{want}`, and this one stops with `{got}`"
                        )
                    })?;
                }
                false => self.check(arg, param)?,
            }
        }
        self.effect(e.span, format!("`{}`", name.name), (*sig.effects).clone());
        oblige(self)?;
        Ok((*sig.ret).clone())
    }

    /// Does `name`, written as the type in `name::new(..)`, mean `Box`?
    pub(super) fn is_box(&self, name: &Ident) -> bool {
        name.name == "Box" && self.adt_symbol(name.id).is_none()
    }

    //        Γ ⊢ e : T
    //  -------------------------- (T-Box)
    //   Γ ⊢ Box::new(e) : Box<T>
    //
    // Puts `e` on the heap: the one way a T program allocates memory.
    pub(super) fn box_new(
        &mut self,
        e: &Expr,
        targs: &[TypeExpr],
        new: &Ident,
        args: &[Expr],
    ) -> Result<Type, Error> {
        if new.name != "new" {
            return Err(Error::ty(
                new.span,
                format!(
                    "`Box` has no `{}`; `Box::new(x)` puts `x` in a box",
                    new.name
                ),
            ));
        }
        let [arg] = args else {
            return Err(Error::ty(
                e.span,
                format!(
                    "`Box::new` takes 1 argument but {} were supplied",
                    args.len()
                ),
            ));
        };
        let t = match targs {
            [] => self.synth(arg)?,
            [t] => {
                let t = self.type_arg(t)?;
                self.check(arg, &t)?;
                t
            }
            _ => return Err(generics::arity_error("`Box`", 1, targs.len(), e.span)),
        };
        // It allocates, and aborts if memory has run out, which is `alloc`
        // too (see docs/16-out-of-memory.md).
        let effects = Type::effects([Type::Alloc]);
        self.effect(e.span, "`Box::new`", effects);
        Ok(Type::Box(Box::new(t)))
    }
}
