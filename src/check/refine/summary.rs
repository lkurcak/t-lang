//! What a closure gives back, as a call it's given to can count on
//! (docs/74-refined-instantiation.md): its *summary*, of each place its
//! body returns, the value it gives there and what's known there.
//!
//! A closure given straight to a call is walked at the call, from what's
//! known there, as every run of its body (lemma S3 of docs/69-loans.md): so
//! what the walk knows where it returns is true of what each run gives, but
//! of the values the run made, which another run makes anew. The values a
//! summary names that the walk made are said to *exist*: each time the
//! summary is counted on, of a value of `R` the call gives, they're new
//! variables of the logic (`Walk::freshen`), so two results of two runs are
//! never taken to be the same value (lemma C1). What it names from before
//! the walk is the same values, as objects never change. And what the walk
//! made that's a function of what was there before, a measure or a part of
//! a value from before, or a constant of the function (`size`, `bounds`),
//! is the same in every run: it's kept (`Summary::keep`).

use super::*;
use std::rc::Rc;

/// One place a closure's body returns: what it gives there, and what's
/// known there of what the walk made (`Summary::rets`).
#[derive(Debug)]
pub(super) struct Ret {
    pub(super) val: Val,
    pub(super) facts: Vec<Formula>,
}

/// What every run of a closure's body gives back: one of `rets`, of type
/// `ty`, what they name from `vars` and `objs` on made in the run, but
/// what's in `keep`.
#[derive(Debug)]
pub(super) struct Summary {
    pub(super) rets: Vec<Ret>,
    pub(super) ty: Type,
    vars: Var,
    objs: Obj,
    keep: HashSet<Var>,
    keep_objs: HashSet<Obj>,
}

/// One case of an instantiation, from a closure's summary: a value at
/// position `pos` of what one of its returns gave.
#[derive(Debug)]
pub(super) struct Made {
    pub(super) summary: Rc<Summary>,
    pub(super) ret: usize,
    pub(super) pos: Vec<usize>,
}

/// How a summary's values are renamed, once: each variable and object
/// the run made, to a new one.
struct Fresh {
    vars: HashMap<Var, Var>,
    objs: HashMap<Obj, Obj>,
}

impl Walk<'_> {
    /// Where a closure's walk starts: the variables and objects from here
    /// on are made by it.
    pub(super) fn summary_start(&self) -> (Var, Obj) {
        (self.kinds.len() as Var, self.next_obj)
    }

    /// The summary of a closure whose walk started at `start`, gave back
    /// values of type `ty`, and returned with `rets`, what was known at
    /// each: of those facts, the ones about what the walk made.
    pub(super) fn summary(&mut self, start: (Var, Obj), ty: Type, rets: Vec<Ret>) -> Summary {
        let (vars, objs) = start;
        let (keep, keep_objs) = self.kept(vars, objs);
        let made = |v: Var| v >= vars && !keep.contains(&v);
        let rets = (rets.into_iter())
            .map(|r| {
                let facts = (r.facts.into_iter())
                    .filter(|f| {
                        let mut vs = Vec::new();
                        f.vars(&mut vs);
                        vs.into_iter().any(made)
                    })
                    .collect();
                Ret { val: r.val, facts }
            })
            .collect();
        Summary {
            rets,
            ty,
            vars,
            objs,
            keep,
            keep_objs,
        }
    }

    /// The variables and objects made from `vars` and `objs` on that are
    /// the same in every run: constants of the function, and the measures
    /// and parts of values from before, at any depth; and the quotients
    /// and products of those.
    fn kept(&self, vars: Var, objs: Obj) -> (HashSet<Var>, HashSet<Obj>) {
        let mut keep: HashSet<Var> = HashSet::new();
        let mut keep_objs: HashSet<Obj> = HashSet::new();
        keep.extend(self.size);
        for (lo, hi) in self.bounds.values() {
            keep.extend(lo.vars().chain(hi.vars()));
        }
        for v in self.consts.values() {
            val_vars(v, &mut keep);
        }
        // The values that are functions of a value from before.
        let mut todo: Vec<Obj> = Vec::new();
        let outer = |o: Obj| o < objs;
        let mut seen_objs = HashSet::new();
        for &(o, _) in self.measured.keys() {
            if outer(o) && seen_objs.insert(o) {
                todo.push(o);
            }
        }
        for &o in self.parts.keys() {
            if outer(o) && seen_objs.insert(o) {
                todo.push(o);
            }
        }
        while let Some(o) = todo.pop() {
            let mut vals: Vec<Val> = (self.measured.iter())
                .filter(|((p, _), _)| *p == o)
                .map(|(_, v)| v.clone())
                .collect();
            if let Some(parts) = self.parts.get(&o) {
                keep.extend(parts.tag.vars());
                vals.extend(parts.fields.iter().flatten().cloned());
            }
            for v in vals {
                val_vars(&v, &mut keep);
                let mut os = Vec::new();
                val_objs(&v, &mut os);
                for p in os {
                    if !outer(p) && seen_objs.insert(p) {
                        keep_objs.insert(p);
                        todo.push(p);
                    }
                }
            }
        }
        // Quotients of sums of those.
        loop {
            let mut more = false;
            for ((a, _), &q) in &self.quotients {
                if q >= vars
                    && !keep.contains(&q)
                    && a.vars().all(|v| v < vars || keep.contains(&v))
                {
                    keep.insert(q);
                    more = true;
                }
            }
            if !more {
                break;
            }
        }
        (keep, keep_objs)
    }

    /// What `made` says, of value `v`: that what return `made.ret` of its
    /// summary gave at position `made.pos` is `v`, and what was known
    /// there, of new variables and objects for what the run made (lemma C1).
    pub(super) fn made_formula(&mut self, made: &Made, v: &Val) -> Option<Formula> {
        let s = made.summary.clone();
        let ret = s.rets.get(made.ret)?;
        let mut fresh = Fresh {
            vars: HashMap::new(),
            objs: HashMap::new(),
        };
        let val = self.freshen_val(&s, &mut fresh, &ret.val);
        let mut all: Vec<Formula> = (ret.facts.iter())
            .map(|f| f.rename(&mut |x| self.freshen_var(&s, &mut fresh, x)))
            .collect();
        let cases = self.values_at(&val, &s.ty, &made.pos);
        let d = targs::Disj(cases);
        all.push(self.disj_formula(&d, v, false)?);
        Some(Formula::and(all))
    }

    /// Variable `x`, renamed: one the run made, to a new one.
    fn freshen_var(&mut self, s: &Summary, fresh: &mut Fresh, x: Var) -> Var {
        if x < s.vars || s.keep.contains(&x) {
            return x;
        }
        if let Some(&y) = fresh.vars.get(&x) {
            return y;
        }
        let y = match self.kinds[x as usize].clone() {
            // A product is of the two values, whichever they are.
            Kind::Product(a, b) => {
                let (a, b) = (self.freshen_var(s, fresh, a), self.freshen_var(s, fresh, b));
                logic::product(&mut self.kinds, a, b)
            }
            k => self.fresh(k),
        };
        fresh.vars.insert(x, y);
        y
    }

    fn freshen_lin(&mut self, s: &Summary, fresh: &mut Fresh, l: &Lin) -> Lin {
        l.rename(&mut |x| self.freshen_var(s, fresh, x))
    }

    /// Object `o`, renamed: one the run made, to a new one, whose parts,
    /// measures and what's said of its positions are its, renamed.
    fn freshen_obj(&mut self, s: &Summary, fresh: &mut Fresh, o: Obj) -> Obj {
        if o < s.objs || s.keep_objs.contains(&o) {
            return o;
        }
        if let Some(&p) = fresh.objs.get(&o) {
            return p;
        }
        let p = self.new_obj();
        fresh.objs.insert(o, p);
        if let Some(parts) = self.parts.get(&o).cloned() {
            let tag = self.freshen_lin(s, fresh, &parts.tag);
            let fields = (parts.fields.iter())
                .map(|fs| fs.iter().map(|f| self.freshen_val(s, fresh, f)).collect())
                .collect();
            self.parts.insert(
                p,
                Parts {
                    symbol: parts.symbol,
                    tag,
                    fields,
                },
            );
        }
        let measured: Vec<(String, Val)> = (self.measured.iter())
            .filter(|((q, _), _)| *q == o)
            .map(|((_, m), v)| (m.clone(), v.clone()))
            .collect();
        for (m, v) in measured {
            let w = self.freshen_val(s, fresh, &v);
            self.measured.insert((p, m), w);
        }
        if let Some(tree) = self.trees.get(&o).cloned() {
            let tree = (tree.iter())
                .map(|(pos, d)| (pos.clone(), self.freshen_disj(s, fresh, d)))
                .collect();
            self.trees.insert(p, tree);
        }
        p
    }

    fn freshen_disj(&mut self, s: &Summary, fresh: &mut Fresh, d: &targs::Disj) -> targs::Disj {
        let cases = (d.0.iter())
            .map(|(g, c)| {
                let g = (g.iter())
                    .map(|f| f.rename(&mut |x| self.freshen_var(s, fresh, x)))
                    .collect();
                (g, self.freshen_cand(s, fresh, c))
            })
            .collect();
        targs::Disj(cases)
    }

    fn freshen_cand(&mut self, s: &Summary, fresh: &mut Fresh, c: &targs::Cand) -> targs::Cand {
        use targs::Cand;
        match c {
            Cand::Exact(v) => Cand::Exact(self.freshen_val(s, fresh, v)),
            Cand::Pred(closed) => {
                let mut names = HashMap::new();
                for (id, v) in &closed.names {
                    names.insert(*id, self.freshen_val(s, fresh, v));
                }
                Cand::Pred(Rc::new(targs::Closed {
                    pred: closed.pred.clone(),
                    names,
                }))
            }
            Cand::All(ds) => Cand::All(ds.iter().map(|d| self.freshen_disj(s, fresh, d)).collect()),
            // What an inner closure's summary says is of values of this
            // run's it would have to rename too: left out, which says less.
            Cand::Made(_) | Cand::Any => Cand::Any,
        }
    }

    fn freshen_val(&mut self, s: &Summary, fresh: &mut Fresh, v: &Val) -> Val {
        match v {
            Val::Int(l) => Val::Int(self.freshen_lin(s, fresh, l)),
            Val::Bool(f) => Val::Bool(f.rename(&mut |x| self.freshen_var(s, fresh, x))),
            Val::Struct(o, vals) => {
                let o = self.freshen_obj(s, fresh, *o);
                let vals = vals.iter().map(|w| self.freshen_val(s, fresh, w)).collect();
                Val::Struct(o, vals)
            }
            Val::Array(o, l) => {
                let o = self.freshen_obj(s, fresh, *o);
                Val::Array(o, self.freshen_lin(s, fresh, l))
            }
            Val::Ref(o, p) => {
                let o = self.freshen_obj(s, fresh, *o);
                let p = match p {
                    Pointee::Val(w) => Pointee::Val(Box::new(self.freshen_val(s, fresh, w))),
                    p => p.clone(),
                };
                Val::Ref(o, p)
            }
            Val::Obj(o) => Val::Obj(self.freshen_obj(s, fresh, *o)),
            Val::Other => Val::Other,
        }
    }
}

/// The variables in `v`, at any depth, into `out`.
fn val_vars(v: &Val, out: &mut HashSet<Var>) {
    match v {
        Val::Int(l) | Val::Array(_, l) => out.extend(l.vars()),
        Val::Bool(f) => {
            let mut vs = Vec::new();
            f.vars(&mut vs);
            out.extend(vs);
        }
        Val::Struct(_, vals) => vals.iter().for_each(|w| val_vars(w, out)),
        Val::Ref(_, Pointee::Val(w)) => val_vars(w, out),
        Val::Ref(..) | Val::Obj(_) | Val::Other => {}
    }
}

/// The objects in `v`, at any depth, into `out`.
fn val_objs(v: &Val, out: &mut Vec<Obj>) {
    match v {
        Val::Struct(o, vals) => {
            out.push(*o);
            vals.iter().for_each(|w| val_objs(w, out));
        }
        Val::Array(o, _) | Val::Obj(o) => out.push(*o),
        Val::Ref(o, p) => {
            out.push(*o);
            if let Pointee::Val(w) = p {
                val_objs(w, out);
            }
        }
        Val::Int(_) | Val::Bool(_) | Val::Other => {}
    }
}
