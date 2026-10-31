//! `match` and patterns: the types of what each pattern binds, and which
//! arms can never match or are missing (see docs/02-adts.md).

use super::lints::{irrefutable_let, mergeable, narrowable, unreachable_alt, unreachable_arm};
use super::modules::ElseOf;
use super::{Checker, Local, plural, result_span};
use crate::ast::*;
use crate::error::{Error, Span};
use crate::exhaust::{self, Ctor, Ends, Pat, Redundant};
use crate::types::{IntTy, Region, Type, struct_fields, variant_fields};

impl Checker {
    //   Γ ⊢ e : T    pi : T ⊣ Δi    Γ, Δi ⊢ ei : R  (each arm i)    p1..pn cover T
    //  ---------------------------------------------------------------------------- (T-Match)
    //                    Γ ⊢ match e { p1 => e1, .., pn => en } : R
    //
    // `p : T ⊣ Δ` reads "pattern p matches values of type T and binds the
    // variables Δ" (see `pattern`). The last premise is exhaustiveness: every
    // value of type T matches some pi. Without it, a value matching no arm
    // would leave the program stuck, so it's as much a part of soundness as
    // any other premise. Arms that can never match get a warning: they aren't
    // unsound, but they're always a mistake.
    pub(super) fn match_expr(
        &mut self,
        e: &Expr,
        scrutinee: &Expr,
        arms: &[Arm],
        want: Option<&Type>,
    ) -> Result<Type, Error> {
        // What an `if let`, `while let` or `let .. else` means, if it is:
        // its second arm is made up (`modules::Rewrite::if_let`).
        let made = arms.get(1).and_then(|a| self.else_of(a.pat.span));
        // With no `else`, the `if let` is a `()`, whatever's wanted, as an
        // `if`'s is.
        let want = match made {
            Some(ElseOf::IfLet(false)) => None,
            _ => want,
        };
        let t = self.synth(scrutinee)?;
        let mut pats = Vec::new();
        let mut binds = Vec::new();
        for arm in arms {
            let mut b = Vec::new();
            pats.push(self.pattern(&arm.pat, &t, None, &mut b)?);
            self.note_binds(&b, true);
            b.iter().for_each(|l| self.bind(l));
            binds.push(b);
        }
        // The patterns may have told us what the scrutinee is.
        let t = self.solved(&t);
        let written: Vec<Pattern> = arms.iter().map(|a| a.pat.clone()).collect();
        for r in exhaust::redundant(&self.adts, &t, &pats) {
            self.warnings.push(match r {
                Redundant::Arm(1) if let Some(of) = made => irrefutable_let(arms[0].pat.span, of),
                Redundant::Arm(i) => unreachable_arm(arms, i),
                Redundant::Alt(span) => unreachable_alt(span, &written),
                Redundant::Trim(span, lo, hi) => narrowable(span, lo, hi, &written),
            });
        }
        if let Some(missing) = exhaust::missing(&self.adts, &t, &pats) {
            return Err(Error::ty(
                scrutinee.span,
                format!("non-exhaustive `match`: `{missing}` not covered"),
            ));
        }

        // No arms: `match n {}`, on a value that can't exist, so it's
        // never run. It's a `!`, the type with no values, as it should be,
        // and diverges, as all of its arms do.
        let mut result = match arms.is_empty() {
            true => Some(Type::Never),
            false => want.cloned(),
        };
        let before = self.diverges.take();
        let mut all_diverge = true;
        for (arm, b) in arms.iter().zip(binds) {
            let scope = self.locals.len();
            self.locals.extend(b);
            match (&result, want) {
                (Some(r), Some(_)) => self.check(&arm.body, &r.clone())?,
                (Some(r), None) => {
                    let r = r.clone();
                    let u = self.synth(&arm.body)?;
                    let joined = self.join(&r, &u, result_span(&arm.body), |r, u| match made {
                        Some(ElseOf::IfLet(false)) => {
                            format!("`if let` without `else` must have type `()`, found `{r}`")
                        }
                        Some(ElseOf::IfLet(true)) => {
                            format!("`if let` and `else` have different types: `{r}` and `{u}`")
                        }
                        _ => format!("`match` arms have different types: `{r}` and `{u}`"),
                    })?;
                    result = Some(joined);
                }
                (None, _) => result = Some(self.synth(&arm.body)?),
            }
            all_diverge &= self.diverges.take().is_some();
            self.end_scope(scope);
        }
        self.diverges = before.or(all_diverge.then_some(e.span));
        Ok(result.expect("an arm, or none and `!`"))
    }

    /// Checks that pattern `p` can match values of type `ty`, and adds the
    /// variables it binds to `binds`. This is the judgment `p : T ⊣ Δ` from
    /// T-Match, with `binds` as Δ.
    ///
    /// `by_ref` says how variables bind: by value, moving or copying what
    /// they're bound to (`None`), or by reference to it (`Some`, with whether
    /// it's `&mut`). They bind by value until the pattern looks through a
    /// reference: that's *match ergonomics*, as in Rust. In
    /// `match r { Option::Some(x) => .. }` with `r: &Option<i64>`, the
    /// variant pattern looks through the `&` to the `Option<i64>` behind it,
    /// and `x` is a `&i64`, borrowed from inside it.
    ///
    /// Returns the pattern in the reduced form the exhaustiveness checker
    /// uses.
    pub(super) fn pattern(
        &mut self,
        p: &Pattern,
        ty: &Type,
        mut by_ref: Option<bool>,
        binds: &mut Vec<Local>,
    ) -> Result<Pat, Error> {
        let looks_inside = !matches!(
            p.kind,
            PatternKind::Wild | PatternKind::Bind(..) | PatternKind::Or(_)
        );
        let mut ty = ty.clone();
        if looks_inside {
            let mut peeled = 0;
            while let Type::Ref(_, m, inner) = self.shallow(&ty) {
                // Through a `&` to a `&mut`, it's still only shared.
                by_ref = Some(by_ref.unwrap_or(true) && m);
                ty = *inner;
                peeled += 1;
            }
            if peeled > 0 {
                self.typed.peels.insert(p.id, peeled);
            }
        }
        let ty = &ty;
        Ok(match &p.kind {
            PatternKind::Wild => Pat::Wild,
            PatternKind::Bind(name, mutable) => {
                if binds.iter().any(|l| l.name == *name) {
                    return Err(Error::ty(
                        p.span,
                        format!("`{name}` is bound more than once in the same pattern"),
                    ));
                }
                if *mutable && by_ref.is_some() {
                    return Err(Error::ty(
                        p.span,
                        format!(
                            "`{name}` can't be `mut` here: it's bound by reference, as the \
                             pattern looks through a reference to get to it"
                        ),
                    ));
                }
                let t = match by_ref {
                    Some(m) => Type::Ref(Region::Erased, m, Box::new(ty.clone())),
                    None => ty.clone(),
                };
                let mut local = Local::new(name, t, p.name_span(), p.id);
                local.mutable = *mutable;
                local.mut_span = mutable.then(|| Span::new(p.span.start, p.name_span().start));
                local.by_ref = by_ref;
                binds.push(local);
                Pat::Wild
            }
            PatternKind::Int(n, lit) => {
                self.int_pattern(p, ty, &[*n], *lit)?;
                Pat::Ctor(Ctor::Int(*n, *n), Vec::new())
            }
            PatternKind::Range(lo, hi, _, lit) => {
                let values = [*lo, *hi].into_iter().flatten().collect::<Vec<_>>();
                let ends = self.int_pattern(p, ty, &values, *lit)?;
                let (lo, hi) = (lo.unwrap_or(ends.0), hi.unwrap_or(ends.1));
                if lo > hi {
                    return Err(Error::ty(
                        p.span,
                        "this range is empty, so it can never match",
                    ));
                }
                Pat::Range(lo, hi, p.span, ends)
            }
            PatternKind::Bool(b) => {
                self.pattern_is(p, ty, Type::Bool)?;
                Pat::Ctor(Ctor::Bool(*b), Vec::new())
            }
            PatternKind::Unit => {
                self.pattern_is(p, ty, Type::Unit)?;
                Pat::Ctor(Ctor::Unit, Vec::new())
            }
            PatternKind::Variant(name, targs, variant, args) => {
                let (t, i) = self.variant(name, targs, variant)?;
                let fields = variant_fields(&self.adts, &t, i);
                if let Type::Adt(symbol, ..) = &t {
                    self.uses().types.insert(symbol.clone());
                }
                self.pattern_is(p, ty, t)?;
                if fields.len() != args.len() {
                    return Err(Error::ty(
                        p.span,
                        format!(
                            "`{}::{}` has {}, but this pattern has {}",
                            name.name,
                            variant.name,
                            plural(fields.len(), "field"),
                            args.len(),
                        ),
                    ));
                }
                let args = args
                    .iter()
                    .zip(&fields)
                    .map(|(a, t)| self.pattern(a, t, by_ref, binds))
                    .collect::<Result<_, _>>()?;
                Pat::Ctor(Ctor::Variant(i), args)
            }
            // Each alternative is checked separately, and they must all bind
            // the same Δ. Otherwise the arm's body could read a variable that
            // the alternative which actually matched never set.
            PatternKind::Or(alts) => {
                let outer = binds.len();
                let mut first: Option<Vec<Local>> = None;
                let mut pats = Vec::new();
                for alt in alts {
                    let mut b = binds.clone();
                    pats.push((self.pattern(alt, ty, by_ref, &mut b)?, alt.span));
                    let new = b.split_off(outer);
                    match &mut first {
                        None => first = Some(new),
                        Some(want) => {
                            if let Some(msg) = or_mismatch(want, &new) {
                                return Err(Error::ty(alt.span, msg));
                            }
                            // The same variable, bound in one more place, so
                            // with the same type.
                            for l in want.iter_mut() {
                                let m = new.iter().find(|m| m.name == l.name);
                                let m = m.expect("or_mismatch checked the names");
                                self.unify_or(&l.ty, &m.ty, alt.span, |t, u| {
                                    format!(
                                        "`{}` is `{t}` in the first alternative but `{u}` in this one",
                                        l.name
                                    )
                                })?;
                                l.sites.extend(m.sites.iter().copied());
                            }
                        }
                    }
                }
                self.warnings.extend(mergeable(p, alts));
                binds.extend(first.unwrap_or_default());
                Pat::Or(pats)
            }
            PatternKind::Struct(name, targs, field_pats, rest) => {
                let t = self.struct_type(name, targs)?;
                let fields = struct_fields(&self.adts, &t).expect("a struct");
                let Type::Adt(symbol, ..) = &t else {
                    unreachable!("a struct")
                };
                let symbol = symbol.clone();
                self.not_intrinsic(&symbol, name.span)?;
                self.uses().types.insert(symbol.clone());
                self.pattern_is(p, ty, t.clone())?;
                self.typed.struct_pats.insert(p.id, t);
                // Fields left out of the pattern match anything.
                let mut args = vec![Pat::Wild; fields.len()];
                let mut given = vec![false; fields.len()];
                for fp in field_pats {
                    let Some(i) = fields.iter().position(|(f, _)| *f == fp.name.name) else {
                        return Err(Error::ty(
                            fp.name.span,
                            format!("`{}` has no field `{}`", name.name, fp.name.name),
                        ));
                    };
                    if given[i] {
                        return Err(Error::ty(
                            fp.name.span,
                            format!("field `{}` is mentioned twice", fp.name.name),
                        ));
                    }
                    given[i] = true;
                    self.field_visible(&symbol, &fp.name)?;
                    self.note_field(&symbol, &fp.name, &fields[i].1);
                    args[i] = self.pattern(&fp.pat, &fields[i].1, by_ref, binds)?;
                    // `P { x }`: the parser made the pattern `x` out of the
                    // field name, so they're in the same place.
                    if fp.pat.span == fp.name.span
                        && let Some(site) = binds.last_mut().and_then(|l| l.sites.first_mut())
                    {
                        site.shorthand = true;
                    }
                }
                if let Some(i) = given.iter().position(|g| !g)
                    && !rest
                {
                    return Err(Error::ty(
                        p.span,
                        format!(
                            "pattern does not mention field `{}`; add it, or `..` to ignore the rest",
                            fields[i].0
                        ),
                    ));
                }
                Pat::Ctor(Ctor::Struct, args)
            }
            // An array of exactly as many values as there are patterns: a
            // length that's a const parameter isn't one (lemma A6 of
            // docs/22-arrays.md).
            PatternKind::Array(pats) => {
                let k = pats.len();
                let elem = self.fresh(
                    p.span,
                    "the type of this array's values".into(),
                    "say what it is".into(),
                );
                let found = Type::Array(Box::new(elem.clone()), Box::new(Type::Nat(k as u64)));
                self.unify_or(&found, ty, p.span, |_, ty| {
                    format!("this pattern is for an array of {k}, but the value is `{ty}`")
                })?;
                let mut args = Vec::new();
                for q in pats {
                    args.push(self.pattern(q, &elem, by_ref, binds)?);
                }
                Pat::Ctor(Ctor::Struct, args)
            }
        })
    }

    /// Unifies the type `ty` a pattern should match with the type `found` it
    /// does match.
    /// Requires `ty`, what pattern `p` matches, to be an integer type, as
    /// the integers `values` it's written with must fit in. Gives its
    /// least and greatest values, which an end of a range left open runs
    /// to. If the type isn't known yet, or is a type parameter, they're
    /// all an integer type can have, from the least `i64` to the greatest
    /// `u64`, as nothing narrower is sure to be covered.
    /// A literal pattern of `values`, written as `lit`s, matching a `ty`:
    /// the least and greatest values of its type.
    fn int_pattern(
        &mut self,
        p: &Pattern,
        ty: &Type,
        values: &[i128],
        lit: Lit,
    ) -> Result<Ends, Error> {
        // A character is one, and needs no checking: the lexer did.
        if lit == Lit::Char {
            self.pattern_is(p, ty, Type::Char)?;
            return Ok((0, 0x10FFFF));
        }
        if lit == Lit::Byte {
            self.pattern_is(p, ty, Type::Int(IntTy::U8))?;
        }
        let int = self.fresh_int(p.span, String::new(), String::new());
        self.unify_or(&int, ty, p.span, |found, ty| {
            format!("expected `{ty}`, found `{found}`")
        })?;
        for &value in values {
            self.literals.push(super::Literal {
                node: p.id,
                span: p.span,
                value,
                ty: ty.clone(),
            });
        }
        Ok(match self.solved(ty) {
            Type::Int(t) => t.range_64(),
            _ => (IntTy::I64.range_64().0, IntTy::U64.range_64().1),
        })
    }

    fn pattern_is(&mut self, p: &Pattern, ty: &Type, found: Type) -> Result<(), Error> {
        self.unify_or(&found, ty, p.span, |found, ty| {
            format!("expected `{ty}`, found `{found}`")
        })
    }
}

/// Why an or-pattern alternative's bindings `b` don't have the same names as
/// the first alternative's `a`, if they don't.
fn or_mismatch(a: &[Local], b: &[Local]) -> Option<String> {
    for Local { name, .. } in a {
        if !b.iter().any(|l| l.name == *name) {
            return Some(format!("`{name}` is not bound in this alternative"));
        }
    }
    let extra = b.iter().find(|l| !a.iter().any(|m| m.name == l.name))?;
    Some(format!(
        "`{}` is bound in this alternative but not in the first",
        extra.name
    ))
}
