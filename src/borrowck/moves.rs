//! Moves and mutability: that nothing is used after it's been moved out of,
//! that nothing is moved out from behind a reference, and that only what's
//! declared `mut`, or reached through a `&mut`, is changed.
//!
//! Whether a place may have been moved out of at a point depends on the
//! path taken to get there, so it's a *dataflow* problem: the set of places
//! that *may* be moved out of flows forward through the body, growing at
//! each move and shrinking at each assignment, and where two paths meet it's
//! what either brings (`moved`). A value that's moved out of on one path
//! but not another can't be used after they meet: types are checked once,
//! not per path.

use super::build::{Builder, project};
use super::describe;
use super::mir::*;
use crate::error::{Edit, Error, Fix, Span};
use crate::types::Type;
use std::collections::BTreeMap;

/// The places moved out of, each with where, in order.
type Moved = BTreeMap<Place, Span>;

/// The part of `place` that the body owns: up to the first `*` of a
/// reference. What's behind a reference belongs to someone else, so moving
/// out of it isn't allowed, and assigning to it doesn't make anything the
/// body owns hold a value again.
fn owned(b: &Builder, place: &Place) -> (Place, bool) {
    let mut t = b.body.locals[place.local.0].ty.clone();
    for (i, p) in place.proj.iter().enumerate() {
        if let (Proj::Deref, Type::Ref(..)) = (p, &t) {
            let owned = Place {
                local: place.local,
                proj: place.proj[..i].to_vec(),
            };
            return (owned, true);
        }
        t = project(b.adts(), &t, *p);
    }
    (place.clone(), false)
}

/// Checks that nothing is moved out from behind a reference.
pub(crate) fn behind_references(b: &Builder) -> Result<(), Error> {
    for block in &b.body.blocks {
        for s in &block.stmts {
            let StmtKind::Assign { value, .. } = &s.kind else {
                continue;
            };
            for o in value.operands() {
                if let Operand::Move(p) = o
                    && owned(b, p).1
                {
                    let t = b.place_ty(p);
                    return Err(Error::ty(
                        s.span,
                        format!(
                            "cannot move out of `{}`, which is behind a reference: a `{t}` \
                             isn't `Copy`, and the reference doesn't own it",
                            describe(b, p)
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Checks that whatever's assigned to or borrowed `&mut` may be changed,
/// and returns the locals that had to be `mut` for that.
pub(crate) fn mutability(b: &Builder) -> Result<Vec<Local>, Error> {
    let mut needed = Vec::new();
    let changes = b
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.stmts)
        .flat_map(|s| {
            let mut out = Vec::new();
            if let StmtKind::Assign {
                place,
                value,
                declare,
            } = &s.kind
            {
                if let Rvalue::Ref(_, true, place) = value {
                    out.push((s, place, true));
                }
                if !declare {
                    out.push((s, place, false));
                }
            }
            out
        });
    {
        for (s, place, borrow) in changes {
            let act = |target: &str| match borrow {
                true => format!("cannot borrow `{target}` as mutable"),
                false => format!("cannot assign to `{target}`"),
            };
            // It may be changed if it's behind a `&mut`, whatever holds
            // that; behind a `&` it may not. Otherwise, it's up to the
            // local.
            let mut t = b.body.locals[place.local.0].ty.clone();
            let mut behind = None;
            for (i, p) in place.proj.iter().enumerate() {
                if let (Proj::Deref, Type::Ref(_, m, _)) = (p, &t) {
                    behind = Some((*m, i));
                }
                t = project(b.adts(), &t, *p);
            }
            match behind {
                Some((true, _)) => continue,
                Some((false, i)) => {
                    let through = Place {
                        local: place.local,
                        proj: place.proj[..i].to_vec(),
                    };
                    return Err(Error::ty(
                        s.span,
                        format!(
                            "{}: it's behind a `&` reference, `{}`, and only what's behind a \
                             `&mut` can be changed",
                            act(&describe(b, place)),
                            describe(b, &through)
                        ),
                    ));
                }
                None => {}
            }
            let decl = &b.body.locals[place.local.0];
            if decl.mutable {
                needed.push(place.local);
                continue;
            }
            let name = decl.name.clone().unwrap_or_default();
            let target = describe(b, place);
            let e = match decl.kind {
                LocalKind::Captured => Error::ty(
                    s.span,
                    format!(
                        "{}: `{name}` is captured by a closure, which has a copy of it that it \
                         can't change",
                        act(&target)
                    ),
                ),
                _ => {
                    let by_ref = b.cx.typed().binds.get(&decl.span).and_then(|b| b.by_ref);
                    let e = Error::ty(
                        s.span,
                        format!("{}, as `{name}` isn't declared `mut`", act(&target)),
                    )
                    .with_note(decl.span, format!("`{name}` is declared here"));
                    match by_ref {
                        Some(_) => e,
                        None => e.with_fix(Fix::new(
                            format!("make `{name}` mutable"),
                            vec![Edit::insert(decl.span.start, "mut ")],
                        )),
                    }
                }
            };
            return Err(e);
        }
    }
    Ok(needed)
}

/// Checks that nothing is used while it may have been moved out of.
pub(crate) fn moves(b: &Builder) -> Result<(), Error> {
    let body = &b.body;
    // At the start, only the parameters and captured variables have values.
    let mut start = Moved::new();
    for (i, l) in body.locals.iter().enumerate() {
        if !matches!(l.kind, LocalKind::Param | LocalKind::Captured) {
            start.insert(Place::local(Local(i)), l.span);
        }
    }
    let mut ins: Vec<Option<Moved>> = vec![None; body.blocks.len()];
    ins[0] = Some(start);
    let mut todo = vec![0];
    while let Some(bb) = todo.pop() {
        let mut state = ins[bb].clone().expect("reached");
        for s in &body.blocks[bb].stmts {
            step(b, &mut state, s).map_err(|e| e.0)?;
        }
        for succ in body.successors(Bb(bb)) {
            let new = match &ins[succ.0] {
                None => state.clone(),
                Some(old) => {
                    let mut new = old.clone();
                    for (p, span) in &state {
                        new.entry(p.clone()).or_insert(*span);
                    }
                    new
                }
            };
            if ins[succ.0].as_ref() != Some(&new) {
                ins[succ.0] = Some(new);
                todo.push(succ.0);
            }
        }
    }
    Ok(())
}

/// Not an error yet: one found while looking.
struct Found(Error);

/// What statement `s` does to `state`, the places that may be moved out of
/// before it, or the error if it uses one.
fn step(b: &Builder, state: &mut Moved, s: &Statement) -> Result<(), Found> {
    match &s.kind {
        StmtKind::Assign { place, value, .. } => {
            for o in value.operands() {
                match o {
                    Operand::Copy(p) => used(b, state, p, true, s.span)?,
                    Operand::Move(p) => {
                        used(b, state, p, true, s.span)?;
                        let (p, _) = owned(b, p);
                        state.retain(|m, _| !m.within(&p));
                        state.insert(p, s.span);
                    }
                    Operand::Const => {}
                }
            }
            if let Rvalue::Ref(_, _, p) = value {
                used(b, state, p, true, s.span)?;
            }
            let (owned, behind) = owned(b, place);
            if behind {
                // Through a reference: the reference itself must be there.
                used(b, state, &owned, false, s.span)?;
            } else {
                if let Some((m, _)) = state.iter().find(|(m, _)| owned.within(m) && **m != owned) {
                    return Err(Found(Error::ty(
                        s.span,
                        format!(
                            "cannot assign to `{}`, part of `{}`, which was moved out of",
                            describe(b, place),
                            describe(b, m)
                        ),
                    )));
                }
                state.retain(|m, _| !m.within(&owned));
            }
        }
        StmtKind::Inspect(p) => used(b, state, p, false, s.span)?,
        StmtKind::Dead(l) => {
            let p = Place::local(*l);
            state.retain(|m, _| !m.within(&p));
            state.insert(p, s.span);
        }
    }
    Ok(())
}

/// Checks that `place` can be used at `span`: that nothing it's part of
/// may have been moved out of, nor, if `whole`, any part of it.
fn used(b: &Builder, state: &Moved, place: &Place, whole: bool, span: Span) -> Result<(), Found> {
    let (owned, _) = owned(b, place);
    for (m, moved_at) in state {
        let partly = whole && m.within(&owned) && *m != owned;
        if !(owned.within(m) || partly) {
            continue;
        }
        let decl = &b.body.locals[m.local.0];
        // A variable that's out of scope, or not given a value yet: the
        // checker wouldn't let either be named, so it's a temporary, or a
        // variable from an earlier time round a loop.
        let target = describe(b, place);
        let message = match partly {
            true => format!(
                "use of partly moved value `{target}`: `{}` was moved out of it",
                describe(b, m)
            ),
            false if *m == Place::local(m.local) && *moved_at == decl.span => {
                format!("use of `{target}`, which has no value here")
            }
            false if owned == *m => format!("use of moved value `{target}`"),
            false => format!("use of moved value `{}`", describe(b, m)),
        };
        let mut e = Error::ty(span, message);
        if *moved_at != decl.span {
            let note = match moved_at.start > span.start {
                true => "moved here, in an earlier time round the loop".to_string(),
                false => format!("`{}` was moved here", describe(b, m)),
            };
            e = e.with_note(*moved_at, note);
        }
        if decl.kind != LocalKind::Temp && *moved_at != decl.span {
            let t = b.place_ty(m);
            e = e.with_note(
                decl.span,
                format!(
                    "`{}` is a `{t}`, which isn't `Copy`, so using it moves it",
                    describe(b, m)
                ),
            );
        }
        return Err(Found(e));
    }
    Ok(())
}

/// The variables declared `mut` that didn't need to be: nothing assigns
/// to them or borrows them `&mut`. Each with where the `mut` is.
pub(crate) fn unused_mut(b: &Builder, needed: &[Local]) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    for (i, l) in b.body.locals.iter().enumerate() {
        if let (Some(span), Some(name)) = (l.mut_span, &l.name)
            && l.mutable
            && !needed.contains(&Local(i))
        {
            out.push((name.clone(), span));
        }
    }
    out
}
