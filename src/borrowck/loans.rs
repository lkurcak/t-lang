//! Loans: that nothing borrowed is used in a way that clashes with the
//! borrow while it lasts.
//!
//! A loan lasts as long as its region holds the points control gets to
//! from where it was made, until what it borrows is given a new value
//! (which no longer has anything to do with the loan). That's another
//! dataflow problem: the loans *in scope* flow forward, joining where paths
//! meet. Then each access to a place is checked against the loans in scope
//! there: reading clashes with a `&mut` loan of an overlapping place, and
//! writing, moving or dropping clashes with any.
//!
//! A call's borrows of places are made after its other arguments (see
//! `Builder::call_into`), so `v.push(v.len())` needs nothing special here;
//! but what such a place is found through mustn't change in between
//! (`pins`).

use super::build::{self, Builder, Loan, project};
use super::describe;
use super::regions::Solution;
use crate::error::{Error, Span};
use crate::mir::*;
use crate::types::Type;

/// How a statement uses a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    /// Reads it, all of it, through any references in it.
    Read,
    /// Looks at it, not through references: a `match` reading which
    /// variant it is.
    Inspect,
    /// Moves out of it.
    Move,
    /// Borrows it `&`.
    Borrow,
    /// Borrows it `&mut`.
    BorrowMut,
    /// Gives it a new value, dropping what was there.
    Assign,
    /// It goes out of scope.
    Dead,
}

impl Access {
    fn writes(self) -> bool {
        !matches!(self, Access::Read | Access::Inspect | Access::Borrow)
    }

    /// Does it reach through references in the place, to what they point
    /// to? Reading a reference reads what it points to; giving a variable
    /// that holds a reference a new value, or dropping it, doesn't touch
    /// what it pointed to.
    fn deep(self) -> bool {
        !matches!(self, Access::Inspect | Access::Assign | Access::Dead)
    }
}

/// The places statement `s` uses, and how, in order.
fn accesses(s: &Statement) -> Vec<(Place, Access)> {
    let mut out = Vec::new();
    match &s.kind {
        StmtKind::Assign { place, value, .. } => {
            for o in value.operands() {
                match o {
                    Operand::Copy(p) => out.push((p.clone(), Access::Read)),
                    Operand::Move(p) => out.push((p.clone(), Access::Move)),
                    Operand::Const(_) => {}
                }
            }
            if let Rvalue::Ref(_, m, p) = value {
                let a = if *m {
                    Access::BorrowMut
                } else {
                    Access::Borrow
                };
                out.push((p.clone(), a));
            }
            out.push((place.clone(), Access::Assign));
        }
        StmtKind::Inspect(p) => out.push((p.clone(), Access::Inspect)),
        StmtKind::Dead(l) => out.push((Place::local(*l), Access::Dead)),
        StmtKind::Consume(p, _) => out.push((p.clone(), Access::Move)),
        // Only the interpreter looks, at what the refinement checker proved.
        StmtKind::Refine(..)
        | StmtKind::Holds(_)
        | StmtKind::Kept(_)
        | StmtKind::Drop(..)
        | StmtKind::Free(_) => {}
    }
    out
}

/// Checks that no argument gives a new value to what a borrow an earlier
/// one waits for is found through (`build::Pin`, lemma A5 of
/// docs/83-sources.md): giving it one, lending it `&mut`, or moving it,
/// or a place it's in, short of a `*`.
pub(crate) fn pins(b: &Builder) -> Result<(), Error> {
    for pin in &b.pins {
        let Some(until) = pin.until else {
            continue;
        };
        // Only the later arguments' blocks, and of those, only the ones
        // from which the borrows are got to: a `?`, `return` or `break` in
        // one leaves the call, which then borrows nothing.
        let ours = |bb: Bb| bb == pin.from.block || bb.0 >= pin.first_block;
        let mut reaches = std::collections::HashSet::from([until.block]);
        loop {
            let more: Vec<Bb> = (0..b.body.blocks.len())
                .map(Bb)
                .filter(|bb| ours(*bb) && !reaches.contains(bb))
                .filter(|bb| b.body.successors(*bb).iter().any(|s| reaches.contains(s)))
                .collect();
            if more.is_empty() {
                break;
            }
            reaches.extend(more);
        }
        let mut todo = vec![pin.from];
        let mut seen = std::collections::HashSet::new();
        while let Some(p) = todo.pop() {
            if p == until || !seen.insert(p) {
                continue;
            }
            let block = &b.body.blocks[p.block.0];
            if p.index < block.stmts.len() {
                let s = &block.stmts[p.index];
                for (place, access) in accesses(s) {
                    if access.writes() && changes(&place, &pin.place) {
                        return Err(pin_clash(b, pin, &place, access, s.span));
                    }
                }
                todo.push(Point {
                    block: p.block,
                    index: p.index + 1,
                });
            } else {
                for succ in b.body.successors(p.block) {
                    if ours(succ) && reaches.contains(&succ) {
                        todo.push(Point {
                            block: succ,
                            index: 0,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

/// Does writing `place` change what's at `pinned`: is it `pinned`, or a
/// place `pinned` is in, reached without a `*`?
fn changes(place: &Place, pinned: &Place) -> bool {
    place.local == pinned.local
        && place.proj.len() <= pinned.proj.len()
        && place.proj.iter().zip(&pinned.proj).all(|(p, q)| p == q)
        && !pinned.proj[place.proj.len()..].contains(&Proj::Deref)
}

/// The error for `access` to `place`, at `span`, which changes what `pin`
/// pins.
fn pin_clash(b: &Builder, pin: &build::Pin, place: &Place, access: Access, span: Span) -> Error {
    let what = describe(b, place);
    let borrowed = describe(b, &pin.borrowed);
    let doing = match access {
        Access::BorrowMut => format!("lend `{what}` here"),
        Access::Move => format!("move `{what}` here"),
        _ => format!("give `{what}` a new value here"),
    };
    Error::ty(
        span,
        format!(
            "can't {doing}: an earlier argument borrows `{borrowed}`, which is found through it \
             when the call's borrows are taken, after the other arguments; change it before \
             the call"
        ),
    )
}

/// Do loan `loan` and an access to `place` of kind `access` overlap?
fn overlaps(b: &Builder, loan: &Place, place: &Place, access: Access) -> bool {
    if loan.local != place.local {
        return false;
    }
    for (p, q) in loan.proj.iter().zip(&place.proj) {
        match (p, q) {
            _ if p == q => {}
            // Different fields of the same struct, or of the same variant,
            // are separate places.
            (Proj::Field(a), Proj::Field(b)) if a != b => return false,
            (Proj::Variant(v, a), Proj::Variant(w, b)) if v == w && a != b => return false,
            _ => return true,
        }
    }
    if loan.proj.len() <= place.proj.len() || access.deep() {
        return true;
    }
    // The loan is of a part of the place. A shallow access only reaches it
    // if it doesn't take a reference to get there.
    let mut t = b.body.locals[loan.local.0].ty.clone();
    for (i, p) in loan.proj.iter().enumerate() {
        if i >= place.proj.len() && matches!((p, &t), (Proj::Deref, Type::Ref(..))) {
            return false;
        }
        t = project(b.adts(), &t, *p);
    }
    true
}

/// Checks every access against the loans in scope where it happens.
pub(crate) fn check(b: &Builder, sol: &Solution) -> Result<(), Error> {
    let body = &b.body;
    let n = b.loans.len();
    // Which loans are in scope. Where paths meet, those in scope on
    // either.
    let mut ins: Vec<Option<Vec<bool>>> = vec![None; body.blocks.len()];
    ins[0] = Some(vec![false; n]);
    let mut todo = vec![0];
    let mut first: Option<Error> = None;
    while let Some(bb) = todo.pop() {
        let mut live = ins[bb].clone().expect("reached");
        for (i, s) in body.blocks[bb].stmts.iter().enumerate() {
            let point = Point {
                block: Bb(bb),
                index: i,
            };
            in_region(b, sol, &mut live, point);
            if let Err(e) =
                pin_reads(b, sol, &live, point).and_then(|()| conflicts(b, sol, &live, s))
                && first.as_ref().is_none_or(|f| e.span.start < f.span.start)
            {
                first = Some(e);
            }
            effect(b, &mut live, s, point);
        }
        let end = Point {
            block: Bb(bb),
            index: body.blocks[bb].stmts.len(),
        };
        in_region(b, sol, &mut live, end);
        if let Err(e) = pin_reads(b, sol, &live, end)
            && first.as_ref().is_none_or(|f| e.span.start < f.span.start)
        {
            first = Some(e);
        }
        for succ in body.successors(Bb(bb)) {
            let new = match &ins[succ.0] {
                None => live.clone(),
                Some(old) => old.iter().zip(&live).map(|(a, b)| *a || *b).collect(),
            };
            if ins[succ.0].as_ref() != Some(&new) {
                ins[succ.0] = Some(new);
                todo.push(succ.0);
            }
        }
    }
    match first {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Drops the loans whose regions don't hold `point` from `live`.
fn in_region(b: &Builder, sol: &Solution, live: &mut [bool], point: Point) {
    for (i, l) in b.loans.iter().enumerate() {
        if live[i] && !sol.holds(l.region, point) {
            live[i] = false;
        }
    }
}

/// What statement `s`, at `point`, does to the loans in scope: those of
/// places it gives new values go, and the one it makes comes.
fn effect(b: &Builder, live: &mut [bool], s: &Statement, point: Point) {
    let gone = match &s.kind {
        StmtKind::Assign { place, .. } => Some(place.clone()),
        StmtKind::Dead(l) => Some(Place::local(*l)),
        StmtKind::Inspect(_)
        | StmtKind::Consume(..)
        | StmtKind::Refine(..)
        | StmtKind::Holds(_)
        | StmtKind::Kept(_)
        | StmtKind::Drop(..)
        | StmtKind::Free(_) => None,
    };
    for (i, l) in b.loans.iter().enumerate() {
        if let Some(p) = &gone
            && l.place.within(p)
        {
            live[i] = false;
        }
        if l.point == point {
            live[i] = true;
        }
    }
}

/// The first clash between reading each place a pin starts at `point`
/// pins, where its argument is, and the loans in `live` (`build::Pin`): a
/// `&mut` to it in use there could change it later.
fn pin_reads(b: &Builder, sol: &Solution, live: &[bool], point: Point) -> Result<(), Error> {
    for pin in b.pins.iter().filter(|p| p.from == point) {
        clashes(
            b,
            sol,
            live,
            &[(pin.place.clone(), Access::Inspect)],
            pin.span,
        )?;
    }
    Ok(())
}

/// The first clash between what `s` does and the loans in `live`.
fn conflicts(b: &Builder, sol: &Solution, live: &[bool], s: &Statement) -> Result<(), Error> {
    clashes(b, sol, live, &accesses(s), s.span)
}

/// The first clash between accesses `accesses`, at `span`, and the loans
/// in `live`.
fn clashes(
    b: &Builder,
    sol: &Solution,
    live: &[bool],
    accesses: &[(Place, Access)],
    span: Span,
) -> Result<(), Error> {
    for (place, access) in accesses.iter().cloned() {
        for (i, loan) in b.loans.iter().enumerate() {
            let mutable = loan.mutable;
            if !live[i] || !(mutable || access.writes()) {
                continue;
            }
            if overlaps(b, &loan.place, &place, access) {
                let escapes = !sol.ends(loan.region, b.regions.universal.len()).is_empty();
                let e = clash(b, loan, mutable, &place, access, span, escapes);
                return Err(match dropped_later(b, sol, loan) {
                    Some((name, span)) => e.with_note(
                        span,
                        format!(
                            "{name} is dropped here, which may call a `discard` that uses \
                             what it borrows (see docs/20-destructors.md)"
                        ),
                    ),
                    None => e,
                });
            }
        }
    }
    Ok(())
}

/// Where a drop that uses what it drops keeps `loan` alive, if one does:
/// the first, by where it is, of those its region holds (`Body::drop_uses`),
/// with what's dropped there.
fn dropped_later(b: &Builder, sol: &Solution, loan: &Loan) -> Option<(String, Span)> {
    let mut points: Vec<&Point> = (b.body.drop_uses.iter())
        .filter(|p| sol.holds(loan.region, **p))
        .collect();
    points.sort_by_key(|p| (p.block.0, p.index));
    let s = &b.body.blocks[points.first()?.block.0].stmts[points.first()?.index];
    let l = match &s.kind {
        StmtKind::Dead(l) => *l,
        StmtKind::Assign { place, .. } => place.local,
        _ => return None,
    };
    let name = match &b.body.locals[l.0].name {
        Some(n) => format!("`{n}`"),
        None => "a temporary".to_string(),
    };
    Some((name, s.span))
}

/// The error for `access` to `place` at `span` while `loan` is in scope,
/// as a `&mut` if `mutable`. `escapes` says the loan lasts past the end of
/// the function.
fn clash(
    b: &Builder,
    loan: &Loan,
    mutable: bool,
    place: &Place,
    access: Access,
    span: Span,
    escapes: bool,
) -> Error {
    let target = describe(b, place);
    let borrowed = describe(b, &loan.place);
    let kind = match mutable {
        true => "mutably borrowed",
        false => "borrowed",
    };
    let decl = &b.body.locals[place.local.0];
    let message = match access {
        Access::Read | Access::Inspect => {
            format!("cannot use `{target}` while `{borrowed}` is mutably borrowed")
        }
        Access::Move => format!("cannot move out of `{target}` while `{borrowed}` is {kind}"),
        Access::Assign => format!("cannot assign to `{target}` while `{borrowed}` is {kind}"),
        Access::Borrow => {
            format!("cannot borrow `{target}` while `{borrowed}` is mutably borrowed")
        }
        Access::BorrowMut if mutable => format!(
            "cannot borrow `{target}` as mutable more than once at a time: `{borrowed}` is \
             already mutably borrowed"
        ),
        Access::BorrowMut => {
            format!("cannot borrow `{target}` as mutable while `{borrowed}` is borrowed")
        }
        Access::Dead if decl.kind == LocalKind::Temp => {
            "a temporary value is dropped here while still borrowed; keep it in a variable \
             with `let`, so it lives longer"
                .to_string()
        }
        Access::Dead if escapes => {
            let name = decl.name.clone().unwrap_or_default();
            let what = if b.closure { "closure" } else { "function" };
            format!(
                "`{name}` doesn't live long enough: it goes away when the {what} returns, \
                 but it's borrowed for longer than that"
            )
        }
        Access::Dead => {
            let name = decl.name.clone().unwrap_or_default();
            format!(
                "`{name}` doesn't live long enough: it goes out of scope here while still borrowed"
            )
        }
    };
    let what = match decl.kind {
        LocalKind::Temp => "the temporary value".to_string(),
        _ => format!("`{borrowed}`"),
    };
    let note = match &loan.why {
        Some(why) => format!("{what} is {kind} here: {why}"),
        None => format!("{what} is {kind} here"),
    };
    Error::ty(span, message).with_note(loan.span, note)
}
