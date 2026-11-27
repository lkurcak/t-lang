//! A const's value read in place: a body that only reads an aggregate
//! const, `TABLE[i]` or `CONST.field`, reads the one copy of its value
//! there is, a static, rather than one it builds (docs/102-math.md,
//! "Tables").
//!
//! A use of a const is a call of its body, which builds its value
//! (`crate::consts`): for an array of 64 pairs, that's a kilobyte written
//! at every `TABLE[i]`, and the C compiler can't tell it needn't be. A
//! const's value is plain data, which has no `Cell` (docs/36-consts.md,
//! C4): so a body that never writes the local holding it, nor borrows it
//! `&mut`, can't tell the value it reads from the static's, which is the
//! same value, for the whole run (lemma P1 of docs/102-math.md).

use crate::mir::{
    Body, Const, Local, LocalDecl, LocalKind, Operand, Place, Proj, Rvalue, StmtKind,
};
use crate::types::{Region, Type};
use std::collections::HashMap;

/// Where a candidate local's one value comes from.
#[derive(Clone)]
enum Source {
    /// A call of const item `.0`.
    Const(String),
    /// A move or copy of all of another local.
    Local(Local),
}

/// Promotes, in `body`, each local holding an aggregate const's value
/// that's only read: `is_const` says which bodies are const items' (not
/// statics', whose places can change, nor functions').
pub(super) fn promote(body: &mut Body, is_const: &dyn Fn(&str) -> bool) {
    let n = body.locals.len();
    let mut sources: Vec<Option<Source>> = vec![None; n];
    let mut assigned = vec![0usize; n];
    let mut barred = vec![false; n];
    // Only temporaries, which hold a use's value, are promoted: not
    // parameters, the result, a closure's environment, nor variables,
    // which loop invariants may name. And only aggregates: a scalar is a
    // constant the C compiler folds.
    for (i, l) in body.locals.iter().enumerate() {
        let aggregate = matches!(l.ty, Type::Array(..) | Type::Tuple(_) | Type::Adt(..));
        barred[i] = i <= body.params || !aggregate || l.kind != LocalKind::Temp;
    }
    for block in &body.blocks {
        for s in &block.stmts {
            match &s.kind {
                StmtKind::Assign { place, value, .. } => {
                    if !place.proj.is_empty() {
                        barred[place.local.0] = true;
                    } else {
                        assigned[place.local.0] += 1;
                        sources[place.local.0] = match value {
                            Rvalue::Call(Operand::Const(Const::Fn(f, args)), call_args)
                                if args.is_empty() && call_args.is_empty() && is_const(f) =>
                            {
                                Some(Source::Const(f.clone()))
                            }
                            Rvalue::Use(Operand::Move(p) | Operand::Copy(p))
                                if p.proj.is_empty() =>
                            {
                                Some(Source::Local(p.local))
                            }
                            _ => None,
                        };
                    }
                    if let Rvalue::Ref(_, true, p) = value {
                        barred[p.local.0] = true;
                    }
                }
                // A pattern using it up, a refinement it must keep, and
                // what drops it are about a value of its own.
                StmtKind::Consume(p, _) | StmtKind::Drop(p, _) | StmtKind::Free(p) => {
                    barred[p.local.0] = true;
                }
                StmtKind::Refine(l, _) => barred[l.0] = true,
                StmtKind::Inspect(_)
                | StmtKind::Dead(_)
                | StmtKind::Holds(_)
                | StmtKind::Kept(_) => {}
            }
        }
    }
    // Which const each local's value is, if it's promoted: given once, by
    // the const or by another that's promoted, and never written after.
    let mut of: Vec<Option<String>> = vec![None; n];
    loop {
        let mut changed = false;
        for i in 0..n {
            if of[i].is_some() || barred[i] || assigned[i] != 1 {
                continue;
            }
            let c = match &sources[i] {
                Some(Source::Const(c)) => Some(c.clone()),
                Some(Source::Local(l)) => of[l.0].clone(),
                None => None,
            };
            if c.is_some() {
                of[i] = c;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if of.iter().all(Option::is_none) {
        return;
    }
    // Each promoted local `l` becomes a `&'static` to the const's value,
    // `r`, and its places `*r`'s.
    let mut refs: HashMap<usize, Local> = HashMap::new();
    for (i, c) in of.iter().enumerate() {
        if c.is_some() {
            let decl = &body.locals[i];
            let r = Local(body.locals.len());
            body.locals.push(LocalDecl {
                ty: Type::Ref(Region::Erased, false, Box::new(decl.ty.clone())),
                kind: LocalKind::Temp,
                mutable: false,
                name: None,
                span: decl.span,
                mut_span: None,
                by_ref: None,
                deferred: false,
                of_static: None,
            });
            refs.insert(i, r);
        }
    }
    let place = |p: &Place| -> Place {
        match refs.get(&p.local.0) {
            Some(r) => {
                let mut proj = vec![Proj::Deref];
                proj.extend_from_slice(&p.proj);
                Place { local: *r, proj }
            }
            None => p.clone(),
        }
    };
    // A move out of the static is a copy: the value is plain data, and
    // the static keeps it.
    let operand = |o: &Operand| -> Operand {
        match o {
            Operand::Copy(p) => Operand::Copy(place(p)),
            Operand::Move(p) if refs.contains_key(&p.local.0) => Operand::Copy(place(p)),
            Operand::Move(p) => Operand::Move(place(p)),
            Operand::Const(c) => Operand::Const(c.clone()),
        }
    };
    for block in &mut body.blocks {
        for s in &mut block.stmts {
            match &mut s.kind {
                StmtKind::Assign {
                    place: to, value, ..
                } => {
                    if let Some(r) = refs.get(&to.local.0) {
                        // Its one value: the static, or the reference the
                        // local it came from now is.
                        let from = match &sources[to.local.0] {
                            Some(Source::Const(c)) => Operand::Const(Const::Static(c.clone())),
                            Some(Source::Local(l)) => Operand::Copy(Place::local(refs[&l.0])),
                            None => unreachable!("a promoted local has a source"),
                        };
                        *to = Place::local(*r);
                        *value = Rvalue::Use(from);
                        continue;
                    }
                    *to = place(to);
                    *value = match value {
                        Rvalue::Use(o) => Rvalue::Use(operand(o)),
                        Rvalue::Ref(r, m, p) => Rvalue::Ref(*r, *m, place(p)),
                        Rvalue::Unary(op, o, id) => Rvalue::Unary(*op, operand(o), *id),
                        Rvalue::Binary(op, a, b, id) => {
                            Rvalue::Binary(*op, operand(a), operand(b), *id)
                        }
                        Rvalue::Cast(o, t, id) => Rvalue::Cast(operand(o), t.clone(), *id),
                        Rvalue::Aggregate(k, t, os) => {
                            Rvalue::Aggregate(*k, t.clone(), os.iter().map(&operand).collect())
                        }
                        Rvalue::Call(f, args) => {
                            Rvalue::Call(operand(f), args.iter().map(&operand).collect())
                        }
                    };
                }
                StmtKind::Inspect(p) => *p = place(p),
                StmtKind::Kept(p) => *p = place(p),
                StmtKind::Dead(l) => {
                    if let Some(r) = refs.get(&l.0) {
                        *l = *r;
                    }
                }
                StmtKind::Consume(..)
                | StmtKind::Drop(..)
                | StmtKind::Free(_)
                | StmtKind::Refine(..)
                | StmtKind::Holds(_) => {}
            }
        }
        if let Some(term) = &mut block.term {
            match term {
                crate::mir::Term::Branch(o, ..) => *o = operand(o),
                crate::mir::Term::Match(p, ..) => *p = place(p),
                crate::mir::Term::Goto(_)
                | crate::mir::Term::Return
                | crate::mir::Term::Unreachable => {}
            }
        }
    }
}
