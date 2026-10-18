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
use crate::types::{Type, struct_fields, variant_fields};
use std::collections::{BTreeMap, BTreeSet};

/// The places moved out of, each with where, in order.
type Moved = BTreeMap<Place, Span>;

/// The same, with the point of each move too, for saying whether it was in
/// an earlier time round a loop.
type MovedAt = BTreeMap<Place, (Span, Point)>;

/// The part of `place` that the body owns: up to the first `*` of a
/// reference. What's behind a reference belongs to someone else, so moving
/// out of it isn't allowed, and assigning to it doesn't make anything the
/// body owns hold a value again.
pub(super) fn owned(b: &Builder, place: &Place) -> (Place, bool) {
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
                if !declare && !first_value(b, place) {
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
    deferred(b, &mut needed)?;
    Ok(needed)
}

/// Is assigning to `place` maybe giving a variable declared with no value,
/// `let x;`, its first one? `deferred` checks it is.
fn first_value(b: &Builder, place: &Place) -> bool {
    place.proj.is_empty() && b.body.locals[place.local.0].deferred
}

/// Checks that what's assigned to a variable declared with no value, and
/// not `mut`, doesn't have one already, and adds those that are `mut` to
/// `needed` where they may.
///
/// Which may have a value depends on the path taken, so it's another
/// forward dataflow problem: where each variable may have been given one
/// flows forward, and where paths meet, it's what either brings.
fn deferred(b: &Builder, needed: &mut Vec<Local>) -> Result<(), Error> {
    let body = &b.body;
    if !body.locals.iter().any(|l| l.deferred) {
        return Ok(());
    }
    type Given = Vec<Option<Span>>;
    let step = |state: &mut Given, s: &Statement| match &s.kind {
        StmtKind::Assign { place, .. } if first_value(b, place) => {
            state[place.local.0].get_or_insert(s.span);
        }
        StmtKind::Dead(l) => state[l.0] = None,
        _ => {}
    };
    let mut ins: Vec<Option<Given>> = vec![None; body.blocks.len()];
    ins[0] = Some(vec![None; body.locals.len()]);
    let mut todo = vec![0];
    while let Some(bb) = todo.pop() {
        let mut state = ins[bb].clone().expect("reached");
        body.blocks[bb]
            .stmts
            .iter()
            .for_each(|s| step(&mut state, s));
        for succ in body.successors(Bb(bb)) {
            let new = match &ins[succ.0] {
                None => state.clone(),
                Some(old) => old.iter().zip(&state).map(|(a, b)| a.or(*b)).collect(),
            };
            if ins[succ.0].as_ref() != Some(&new) {
                ins[succ.0] = Some(new);
                todo.push(succ.0);
            }
        }
    }
    let mut first: Option<Error> = None;
    for (bb, block) in body.blocks.iter().enumerate() {
        let Some(mut state) = ins[bb].clone() else {
            continue;
        };
        for s in &block.stmts {
            if let StmtKind::Assign { place, .. } = &s.kind
                && first_value(b, place)
                && let Some(given) = state[place.local.0]
            {
                let decl = &body.locals[place.local.0];
                if decl.mutable {
                    needed.push(place.local);
                } else if first.as_ref().is_none_or(|f| s.span.start < f.span.start) {
                    let name = decl.name.clone().unwrap_or_default();
                    let note = match given.start >= s.span.start {
                        true => "given a value here, in an earlier time round the loop",
                        false => "given a value here",
                    };
                    first = Some(
                        Error::ty(
                            s.span,
                            format!(
                                "cannot assign twice to `{name}`, as it isn't declared `mut`: it \
                                 may have a value already"
                            ),
                        )
                        .with_note(given, note)
                        .with_fix(Fix::new(
                            format!("make `{name}` mutable"),
                            vec![Edit::insert(decl.span.start, "mut ")],
                        )),
                    );
                }
            }
            step(&mut state, s);
        }
    }
    match first {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Checks that nothing is used while it may have been moved out of.
pub(crate) fn moves(b: &Builder) -> Result<(), Error> {
    let body = &b.body;
    // At the start, only the parameters and captured variables have values.
    let mut start = MovedAt::new();
    let entry = Point {
        block: Bb(0),
        index: 0,
    };
    for (i, l) in body.locals.iter().enumerate() {
        if !matches!(l.kind, LocalKind::Param | LocalKind::Captured) {
            start.insert(Place::local(Local(i)), (l.span, entry));
        }
    }
    let mut ins: Vec<Option<MovedAt>> = vec![None; body.blocks.len()];
    ins[0] = Some(start);
    let mut todo = vec![0];
    while let Some(bb) = todo.pop() {
        let mut state = ins[bb].clone().expect("reached");
        for (index, s) in body.blocks[bb].stmts.iter().enumerate() {
            let point = Point {
                block: Bb(bb),
                index,
            };
            step(b, &mut state, s, point).map_err(|e| e.0)?;
        }
        for succ in body.successors(Bb(bb)) {
            let new = match &ins[succ.0] {
                None => state.clone(),
                Some(old) => {
                    let mut new = old.clone();
                    for (p, at) in &state {
                        new.entry(p.clone()).or_insert(*at);
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

/// What statement `s`, at `point`, does to `state`, the places that may be
/// moved out of before it, or the error if it uses one.
fn step(b: &Builder, state: &mut MovedAt, s: &Statement, point: Point) -> Result<(), Found> {
    let here = (s.span, point);
    match &s.kind {
        StmtKind::Assign { place, value, .. } => {
            for o in value.operands() {
                match o {
                    Operand::Copy(p) => used(b, state, p, true, here)?,
                    Operand::Move(p) => {
                        used(b, state, p, true, here)?;
                        let (p, _) = owned(b, p);
                        state.retain(|m, _| !m.within(&p));
                        state.insert(p, here);
                    }
                    Operand::Const => {}
                }
            }
            if let Rvalue::Ref(_, _, p) = value {
                used(b, state, p, true, here)?;
            }
            let (owned, behind) = owned(b, place);
            if behind {
                // Through a reference: the reference itself must be there.
                used(b, state, &owned, false, here)?;
            } else {
                if let Some((m, (at, _))) =
                    state.iter().find(|(m, _)| owned.within(m) && **m != owned)
                {
                    let decl = &b.body.locals[m.local.0];
                    let why = match *m == Place::local(m.local) && *at == decl.span {
                        true => "which has no value here".to_string(),
                        false => "which was moved out of".to_string(),
                    };
                    return Err(Found(Error::ty(
                        s.span,
                        format!(
                            "cannot assign to `{}`, part of `{}`, {why}",
                            describe(b, place),
                            describe(b, m)
                        ),
                    )));
                }
                state.retain(|m, _| !m.within(&owned));
            }
        }
        StmtKind::Inspect(p) => used(b, state, p, false, here)?,
        // What the pattern moved out of it is gone already, so it's the
        // rest that has to be there.
        StmtKind::Consume(p, _) => {
            used(b, state, p, false, here)?;
            state.retain(|m, _| !m.within(p));
            state.insert(p.clone(), here);
        }
        StmtKind::Dead(l) => {
            let p = Place::local(*l);
            state.retain(|m, _| !m.within(&p));
            state.insert(p, here);
        }
    }
    Ok(())
}

/// Checks that `place` can be used where `at` says, a span and a point:
/// that nothing it's part of may have been moved out of, nor, if `whole`,
/// any part of it.
fn used(
    b: &Builder,
    state: &MovedAt,
    place: &Place,
    whole: bool,
    at: (Span, Point),
) -> Result<(), Found> {
    let (span, point) = at;
    let (owned, _) = owned(b, place);
    for (m, (moved_at, moved_point)) in state {
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
            // A move written after the use can still come first: a call's
            // borrows are taken after its arguments.
            let note = match b.body.same_round(*moved_point, point) {
                false => "moved here, in an earlier time round the loop".to_string(),
                true => format!("`{}` was moved here", describe(b, m)),
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

//   Γ ⊢ e : U
//  ------------------ (Weaken)   if T: Discard
//   Γ, x : T ⊢ e : U
//
//   Γ, x : T, x : T ⊢ e : U
//  ------------------------- (Contract)   if T: Copy
//   Γ, x : T ⊢ e : U
//
// Without the two, each variable in the context is used exactly once: a
// *linear* type system. Moves give `Contract` up for what isn't `Copy`,
// and this, `Weaken` for what isn't `Discard`: a value of such a type must
// be *used up* on every path that goes on, moved somewhere or taken apart
// by a pattern, rather than dropped where its owner goes out of scope, or
// thrown away by giving its place a new value. `trait Copy: Discard`, as
// a value that can be copied can't be used up exactly once.
//
// A path that never goes on is exempt: a call that returns `!` ends its
// block, with nothing after it, as a panic aborts the program, and nothing
// is dropped on the way.

/// What may have been used up or moved out of, each with where, as in
/// `moves`, and what *definitely* has, on every path to here.
#[derive(Clone, PartialEq)]
struct Gone {
    may: Moved,
    must: BTreeSet<Place>,
}

/// Is `p`, or what it's part of, in `set`?
fn covered(set: &BTreeSet<Place>, p: &Place) -> bool {
    set.iter().any(|m| p.within(m))
}

/// Adds `p` to `set`, unless it's covered already, and drops its parts,
/// which it covers.
fn add(set: &mut BTreeSet<Place>, p: Place) {
    if !covered(set, &p) {
        set.retain(|m| !m.within(&p));
        set.insert(p);
    }
}

/// What's gone on both paths, where they meet: what one has that the
/// other covers.
fn meet(a: &BTreeSet<Place>, b: &BTreeSet<Place>) -> BTreeSet<Place> {
    let mut out = BTreeSet::new();
    for (x, y) in [(a, b), (b, a)] {
        for p in x.iter().filter(|p| covered(y, p)) {
            add(&mut out, p.clone());
        }
    }
    out
}

/// Why a value is thrown away, for saying so.
#[derive(Clone, Copy)]
enum Dropped {
    /// It goes out of scope.
    Dead,
    /// Its place is given a new value.
    Assign,
    /// A pattern takes it apart, of this variant if it's an enum, and
    /// doesn't move out the rest.
    Consume(Option<usize>),
}

/// Checks that every value that isn't `Discard` is used up on every path
/// that goes on: never dropped while some part of it may still hold one.
///
/// Invariant: MIR drops a value exactly where the interpreter does, at
/// `Dead` (a scope's or statement's end, or a jump out of them), at an
/// `Assign` (what the place held), and at a `Consume` (what the pattern
/// didn't move out); and moves exactly where it does. The interpreter
/// checks each drop for a leak (`Interp::drop`), so a program this lets
/// through that throws away a linear value fails there.
///
/// Whether it's been used up depends on the path, so it's a dataflow
/// problem, like `moves`, but asking what *must* be gone: where paths meet,
/// what's gone on both. Then each statement that drops a value is checked
/// against what's gone before it.
///
/// Whether a type is `Discard` is up to `Traits::implements`, with the
/// bounds in scope. Monomorphisation's copy has to agree, at every type
/// argument, or a value this lets be dropped would leak there. For a rigid
/// projection, `T::Out`, that holds as it's `Discard` only by a bound of
/// the function's, which each caller proved of what it stands for, or by
/// the trait's declaration, `type Out: Discard;`, which each impl was
/// checked against (`Checker::assoc_bounds`).
pub(crate) fn linear(b: &Builder) -> Result<(), Error> {
    let body = &b.body;
    let mut start = Gone {
        may: Moved::new(),
        must: BTreeSet::new(),
    };
    for (i, l) in body.locals.iter().enumerate() {
        if !matches!(l.kind, LocalKind::Param | LocalKind::Captured) {
            start.may.insert(Place::local(Local(i)), l.span);
            start.must.insert(Place::local(Local(i)));
        }
    }
    let mut ins: Vec<Option<Gone>> = vec![None; body.blocks.len()];
    ins[0] = Some(start);
    let mut todo = vec![0];
    while let Some(bb) = todo.pop() {
        let mut state = ins[bb].clone().expect("reached");
        for s in &body.blocks[bb].stmts {
            gone(b, &mut state, s, false)?;
        }
        for succ in body.successors(Bb(bb)) {
            let new = match &ins[succ.0] {
                None => state.clone(),
                Some(old) => {
                    let mut may = old.may.clone();
                    for (p, span) in &state.may {
                        may.entry(p.clone()).or_insert(*span);
                    }
                    let must = meet(&old.must, &state.must);
                    Gone { may, must }
                }
            };
            if ins[succ.0].as_ref() != Some(&new) {
                ins[succ.0] = Some(new);
                todo.push(succ.0);
            }
        }
    }
    let mut first: Option<Error> = None;
    for (bb, block) in body.blocks.iter().enumerate() {
        let Some(mut state) = ins[bb].clone() else {
            continue;
        };
        for s in &block.stmts {
            if let Err(e) = gone(b, &mut state, s, true)
                && first.as_ref().is_none_or(|f| e.span.start < f.span.start)
            {
                first = Some(e);
            }
        }
    }
    match first {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// What statement `s` does to `state`, and if `check`, the error if it
/// throws away what it can't. Moving out of a place comes before giving
/// it a new value: `x = f(x)` throws nothing away.
fn gone(b: &Builder, state: &mut Gone, s: &Statement, check: bool) -> Result<(), Error> {
    let away = |state: &mut Gone, p: Place| {
        state.may.retain(|m, _| !m.within(&p));
        state.may.insert(p.clone(), s.span);
        add(&mut state.must, p);
    };
    match &s.kind {
        StmtKind::Assign { place, value, .. } => {
            for o in value.operands() {
                if let Operand::Move(p) = o {
                    away(state, owned(b, p).0);
                }
            }
            if check {
                dropped(b, state, place, Dropped::Assign, s.span)?;
            }
            let (owned, behind) = owned(b, place);
            if !behind {
                state.may.retain(|m, _| !m.within(&owned));
                state.must.retain(|m| !m.within(&owned));
            }
        }
        StmtKind::Consume(p, variant) => {
            if check {
                dropped(b, state, p, Dropped::Consume(*variant), s.span)?;
            }
            away(state, p.clone());
        }
        StmtKind::Dead(l) => {
            if check {
                dropped(b, state, &Place::local(*l), Dropped::Dead, s.span)?;
            }
            away(state, Place::local(*l));
        }
        StmtKind::Inspect(_) => {}
    }
    Ok(())
}

/// The part of the value at `place` that holds one that isn't `Discard`,
/// and isn't gone, if there is one: what dropping it would throw away. A
/// value that's been taken apart, by moving out of some of it, or by
/// `consumed`, a pattern, of the variant it says if it's an enum, has only
/// its parts left to check.
fn leftover(
    b: &Builder,
    must: &BTreeSet<Place>,
    place: &Place,
    consumed: Option<Option<usize>>,
) -> Option<Place> {
    if covered(must, place) {
        return None;
    }
    let t = b.place_ty(place);
    if b.cx.is_discard(&t) {
        return None;
    }
    let partly = must.iter().any(|m| m.within(place) && m != place);
    let variant = match consumed {
        Some(v) => v,
        None if partly => None,
        None => return Some(place.clone()),
    };
    let parts: Vec<Place> = match &t {
        Type::Box(_) => vec![place.project(Proj::Deref)],
        Type::Adt(..) => match struct_fields(b.adts(), &t) {
            Some(fields) => (0..fields.len())
                .map(|i| place.project(Proj::Field(i)))
                .collect(),
            None => {
                // Which variant it is, if a pattern doesn't say: the one
                // that was moved out of.
                let moved = must
                    .iter()
                    .find_map(|m| match m.proj.get(place.proj.len()) {
                        Some(Proj::Variant(v, _)) if m.within(place) => Some(*v),
                        _ => None,
                    });
                let Some(v) = variant.or(moved) else {
                    return Some(place.clone());
                };
                (0..variant_fields(b.adts(), &t, v).len())
                    .map(|i| place.project(Proj::Variant(v, i)))
                    .collect()
            }
        },
        _ => return Some(place.clone()),
    };
    parts.iter().find_map(|p| leftover(b, must, p, None))
}

/// Checks that dropping the value at `place`, at `span`, for `why`,
/// throws away nothing that isn't `Discard`.
fn dropped(
    b: &Builder,
    state: &Gone,
    place: &Place,
    why: Dropped,
    span: Span,
) -> Result<(), Error> {
    let consumed = match why {
        Dropped::Consume(v) => Some(v),
        _ => None,
    };
    let Some(q) = leftover(b, &state.must, place, consumed) else {
        return Ok(());
    };
    let t = b.place_ty(&q);
    let decl = &b.body.locals[q.local.0];
    let what = describe(b, &q);
    let hint = match b.cx.missing_discard(&t) {
        Some(Type::Param(p)) if p == "Self" => {
            "; to let it be thrown away, make `Discard` a supertrait of the trait".to_string()
        }
        Some(Type::Param(p)) => {
            format!("; to let it be thrown away, add a bound: `{p}: Discard`")
        }
        // A function's bound can say it of a projection on its parameter;
        // otherwise only the trait can.
        Some(Type::Proj(s, tr, a)) => match *s {
            Type::Param(p) if p != "Self" => {
                format!("; to let it be thrown away, add a bound: `{p}: {tr}<{a}: Discard>`")
            }
            _ => format!(
                "; to let it be thrown away, declare it in the trait as `type {a}: Discard;`"
            ),
        },
        Some(u) => format!(
            "; to let it be thrown away, add `#[derive(Discard)]` to `{}`",
            crate::check::source_name(&u.to_string())
        ),
        None => String::new(),
    };
    let isnt = format!("its type, `{t}`, isn't `Discard`{hint}");
    // Where it, or part of it, was used up on some paths, but not all.
    let some_paths = state
        .may
        .iter()
        .find(|(m, at)| (q.within(m) || m.within(&q)) && **at != decl.span)
        .map(|(_, at)| *at);
    let e = match why {
        Dropped::Dead => {
            let temp = decl.kind == LocalKind::Temp;
            let subject = match (temp, q.proj.is_empty()) {
                (true, true) => "this value".to_string(),
                (true, false) => "part of this value".to_string(),
                (false, _) => format!("`{what}`"),
            };
            let never = match some_paths {
                Some(_) => "isn't used up on every path",
                None => "is never used up",
            };
            let e = Error::ty(decl.span, format!("{subject} {never}, and {isnt}"));
            let e = match some_paths {
                Some(at) => e.with_note(at, "it's used up here, on some paths".to_string()),
                None => e,
            };
            let gone = match (temp, decl.kind) {
                (true, _) => "it's dropped here, at the end of the statement".to_string(),
                (_, LocalKind::Param) => format!(
                    "`{}` is dropped here, when the function returns",
                    decl.name.clone().unwrap_or_default()
                ),
                _ => format!(
                    "`{}` goes out of scope here",
                    decl.name.clone().unwrap_or_default()
                ),
            };
            e.with_note(span, gone)
        }
        Dropped::Assign => {
            let replace = format!(
                "; to get it back instead, use `replace(&mut {}, ..)`",
                describe(b, place)
            );
            let held = match q == *place {
                true => format!("the value `{what}` holds"),
                false => format!("`{what}`"),
            };
            Error::ty(
                span,
                format!("this assignment would throw away {held}, and {isnt}{replace}"),
            )
        }
        Dropped::Consume(_) => {
            let part = match decl.kind {
                LocalKind::Temp => "part of the value it matches".to_string(),
                _ => format!("`{what}`"),
            };
            Error::ty(
                span,
                format!(
                    "this pattern throws away {part}, and {isnt}; bind it to a variable instead, \
                     and use that up"
                ),
            )
        }
    };
    Err(e)
}
