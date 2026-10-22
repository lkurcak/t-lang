//! The logic refinements are written in, and the procedure that decides
//! what follows from what (see docs/08-refinement-types.md).
//!
//! The logic is quantifier-free linear integer arithmetic: sums of
//! variables times constants, compared with `<=` and `==`, under `&&`, `||`
//! and `!`. A `bool` is a variable that's 0 or 1. Integers are the
//! mathematical ones; that a `u8` is between 0 and 255 is a fact like any
//! other, which each variable comes with. So is that a `usize` is between
//! 0 and the greatest `usize`, which is a variable too, as how big it is
//! depends on the machine, and all the checker knows of it is that it's
//! between the greatest 32-bit and the greatest 64-bit number: so what it
//! proves, it proves of every machine. A type parameter `T: Int`'s values
//! are between two such variables, the greatest at least -1 minus the
//! least, as in every two's complement type; of a `T: Unsigned` the least
//! is 0, and of a `T: Signed` it's -1 minus the greatest (see
//! docs/11-integers.md and docs/12-bits.md). What the bitwise operators
//! give isn't linear: the logic knows only some facts of it
//! (`refine::bitwise`).
//!
//! `valid(facts, goal)` says whether the goal follows from the facts. It
//! asks the opposite question, whether the facts and the goal's negation
//! can all hold at once, and answers yes unless it finds that they can't,
//! so a "no" from it is never wrong but a "yes" may be (it's *sound* but
//! *incomplete*): it only ever proves true things, but not every true
//! thing. That's what it needs to be trusted for, and the only part of it
//! that is: a weaker procedure rejects more programs, never accepts a
//! wrong one.
//!
//! It splits the question into cases, one for each way the `||`s can go
//! (*disjunctive normal form*), and refutes each case, a list of linear
//! constraints, by *Fourier–Motzkin elimination*: it removes the variables
//! one at a time, replacing the constraints on one with what they say
//! about the others together, until only constants are left, which are
//! either true or false. That's exact for rational numbers. For integers
//! it rounds each constraint's constant to what integers allow (dividing
//! `2x <= 3` by 2 gives `x <= 1`, not `x <= 1.5`): Chvátal–Gomory rounding,
//! which no integer solution notices, so a contradiction it finds is a
//! real one. Some contradictions only integers have, it misses.
//!
//! Every step is bounded: cases, constraints and the size of numbers (with
//! checked `i128` arithmetic). Going past a bound gives up, which is a
//! "maybe", so the answer never depends on anything but the question.

use std::collections::{BTreeMap, HashMap};

/// A variable of the logic, by number (see `Kind`).
pub(crate) type Var = u32;

/// What values a variable stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    /// An integer from the first sum to the second: its type's least and
    /// greatest values. Each is a constant, or a sum of `Bound`s.
    Int(Lin, Lin),
    /// A `bool`: 0 or 1.
    Bool,
    /// An integer from the first constant to the second: the least or
    /// greatest value of a type whose size isn't known, a `usize` or a type
    /// parameter's.
    Bound(i128, i128),
}

/// A linear sum, `c1·x1 + .. + cn·xn + k`, with no zero coefficients and
/// the variables in order, so that equal sums are equal values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Lin {
    terms: Vec<(Var, i128)>,
    k: i128,
}

impl Lin {
    pub(crate) fn constant(k: i128) -> Lin {
        Lin {
            terms: Vec::new(),
            k,
        }
    }

    pub(crate) fn var(v: Var) -> Lin {
        Lin {
            terms: vec![(v, 1)],
            k: 0,
        }
    }

    /// Its value, if it has no variables.
    pub(crate) fn as_constant(&self) -> Option<i128> {
        self.terms.is_empty().then_some(self.k)
    }

    pub(crate) fn vars(&self) -> impl Iterator<Item = Var> + '_ {
        self.terms.iter().map(|&(v, _)| v)
    }

    /// `self + other`, or `None` if a number gets too big.
    pub(crate) fn add(&self, other: &Lin) -> Option<Lin> {
        let mut terms: BTreeMap<Var, i128> = self.terms.iter().copied().collect();
        for &(v, c) in &other.terms {
            let e = terms.entry(v).or_insert(0);
            *e = e.checked_add(c)?;
        }
        Some(Lin {
            terms: terms.into_iter().filter(|&(_, c)| c != 0).collect(),
            k: self.k.checked_add(other.k)?,
        })
    }

    /// `c · self`.
    pub(crate) fn scale(&self, c: i128) -> Option<Lin> {
        if c == 0 {
            return Some(Lin::constant(0));
        }
        let terms = self
            .terms
            .iter()
            .map(|&(v, d)| Some((v, d.checked_mul(c)?)))
            .collect::<Option<_>>()?;
        Some(Lin {
            terms,
            k: self.k.checked_mul(c)?,
        })
    }

    pub(crate) fn sub(&self, other: &Lin) -> Option<Lin> {
        self.add(&other.scale(-1)?)
    }

    pub(crate) fn plus(&self, k: i128) -> Option<Lin> {
        Some(Lin {
            terms: self.terms.clone(),
            k: self.k.checked_add(k)?,
        })
    }

    fn coeff(&self, v: Var) -> i128 {
        self.terms
            .iter()
            .find(|&&(w, _)| w == v)
            .map_or(0, |&(_, c)| c)
    }

    /// `self` with `v` replaced by `by`.
    fn substitute(&self, v: Var, by: &Lin) -> Option<Lin> {
        let c = self.coeff(v);
        if c == 0 {
            return Some(self.clone());
        }
        let rest = Lin {
            terms: self
                .terms
                .iter()
                .copied()
                .filter(|&(w, _)| w != v)
                .collect(),
            k: self.k,
        };
        rest.add(&by.scale(c)?)
    }
}

/// A statement of the logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Formula {
    True,
    False,
    /// `lin <= 0`
    Le(Lin),
    /// `lin == 0`
    Eq(Lin),
    Not(Box<Formula>),
    And(Vec<Formula>),
    Or(Vec<Formula>),
}

impl Formula {
    /// `a <= b`, or `None` if a number gets too big.
    pub(crate) fn le(a: &Lin, b: &Lin) -> Option<Formula> {
        Some(Formula::Le(a.sub(b)?))
    }

    /// `a < b`, which for integers is `a + 1 <= b`.
    pub(crate) fn lt(a: &Lin, b: &Lin) -> Option<Formula> {
        Some(Formula::Le(a.sub(b)?.plus(1)?))
    }

    pub(crate) fn eq(a: &Lin, b: &Lin) -> Option<Formula> {
        Some(Formula::Eq(a.sub(b)?))
    }

    /// That `bool` variable `b` is true.
    pub(crate) fn holds(b: Var) -> Formula {
        // 1 - b <= 0
        Formula::Le(Lin {
            terms: vec![(b, -1)],
            k: 1,
        })
    }

    pub(crate) fn not(self) -> Formula {
        match self {
            Formula::True => Formula::False,
            Formula::False => Formula::True,
            Formula::Not(f) => *f,
            f => Formula::Not(Box::new(f)),
        }
    }

    pub(crate) fn and(fs: impl IntoIterator<Item = Formula>) -> Formula {
        let mut out = Vec::new();
        for f in fs {
            match f {
                Formula::True => {}
                Formula::False => return Formula::False,
                Formula::And(gs) => out.extend(gs),
                f => out.push(f),
            }
        }
        match out.len() {
            0 => Formula::True,
            1 => out.pop().expect("one"),
            _ => Formula::And(out),
        }
    }

    pub(crate) fn or(fs: impl IntoIterator<Item = Formula>) -> Formula {
        let mut out = Vec::new();
        for f in fs {
            match f {
                Formula::False => {}
                Formula::True => return Formula::True,
                Formula::Or(gs) => out.extend(gs),
                f => out.push(f),
            }
        }
        match out.len() {
            0 => Formula::False,
            1 => out.pop().expect("one"),
            _ => Formula::Or(out),
        }
    }

    /// `a ⇔ b`.
    pub(crate) fn iff(a: Formula, b: Formula) -> Formula {
        Formula::or([
            Formula::and([a.clone(), b.clone()]),
            Formula::and([a.not(), b.not()]),
        ])
    }

    fn vars(&self, out: &mut Vec<Var>) {
        match self {
            Formula::True | Formula::False => {}
            Formula::Le(l) | Formula::Eq(l) => out.extend(l.vars()),
            Formula::Not(f) => f.vars(out),
            Formula::And(fs) | Formula::Or(fs) => fs.iter().for_each(|f| f.vars(out)),
        }
    }

    /// The same statement with negations only on atoms, and those turned
    /// into atoms too: `!(l <= 0)` is `-l + 1 <= 0`, for integers, and
    /// `!(l == 0)` is `l < 0 || l > 0`. `None` if a number gets too big.
    fn nnf(&self, negate: bool) -> Option<Formula> {
        Some(match (self, negate) {
            (Formula::True, false) | (Formula::False, true) => Formula::True,
            (Formula::True, true) | (Formula::False, false) => Formula::False,
            (Formula::Le(l), false) => Formula::Le(l.clone()),
            (Formula::Le(l), true) => Formula::Le(l.scale(-1)?.plus(1)?),
            (Formula::Eq(l), false) => Formula::Eq(l.clone()),
            (Formula::Eq(l), true) => {
                Formula::or([Formula::Le(l.plus(1)?), Formula::Le(l.scale(-1)?.plus(1)?)])
            }
            (Formula::Not(f), n) => f.nnf(!n)?,
            (Formula::And(fs), false) | (Formula::Or(fs), true) => Formula::and(
                fs.iter()
                    .map(|f| f.nnf(negate))
                    .collect::<Option<Vec<_>>>()?,
            ),
            (Formula::And(fs), true) | (Formula::Or(fs), false) => Formula::or(
                fs.iter()
                    .map(|f| f.nnf(negate))
                    .collect::<Option<Vec<_>>>()?,
            ),
        })
    }
}

/// The most cases `valid` looks at before giving up.
const MAX_CASES: usize = 512;

/// The most constraints one case may grow to while variables are
/// eliminated.
const MAX_CONSTRAINTS: usize = 400;

/// Whether `goal` follows from `facts`, where variable `v` is of kind
/// `kinds[v]`. Never true unless it does; may be false when it does.
pub(crate) fn valid(facts: &[Formula], goal: &Formula, kinds: &[Kind]) -> bool {
    // Only the facts that share variables with the goal, or with facts that
    // do, and so on, can matter: the rest can be made true on their own.
    // A fact with no variables at all, `false` on a path that can't be
    // taken, matters by itself.
    let mut wanted = Vec::new();
    goal.vars(&mut wanted);
    let mut vars_of: Vec<Vec<Var>> = facts
        .iter()
        .map(|f| {
            let mut vs = Vec::new();
            f.vars(&mut vs);
            vs
        })
        .collect();
    let mut taken = vec![false; facts.len()];
    let mut seen: std::collections::HashSet<Var> = wanted.iter().copied().collect();
    loop {
        let mut changed = false;
        for (i, vs) in vars_of.iter_mut().enumerate() {
            if taken[i] {
                continue;
            }
            if vs.is_empty() || vs.iter().any(|v| seen.contains(v)) {
                taken[i] = true;
                changed = true;
                seen.extend(vs.iter().copied());
            }
        }
        if !changed {
            break;
        }
    }
    let mut parts: Vec<Formula> = facts
        .iter()
        .zip(&taken)
        .filter(|&(_, &t)| t)
        .map(|(f, _)| f.clone())
        .collect();
    parts.push(goal.clone().not());
    let Some(whole) = Formula::and(parts).nnf(false) else {
        return false;
    };
    let mut cases = 0;
    refuted(&mut vec![whole], &mut Vec::new(), kinds, &mut cases).unwrap_or(false)
}

/// Whether every case of `todo` with `atoms` is impossible: `Some(true)`
/// if it found that each is, `Some(false)` if one may not be, `None` if
/// it gave up. `todo` is in negation normal form.
fn refuted(
    todo: &mut Vec<Formula>,
    atoms: &mut Vec<Formula>,
    kinds: &[Kind],
    cases: &mut usize,
) -> Option<bool> {
    // Atoms and `&&` first, so a case is as narrow as can be before it's
    // split.
    while let Some(i) = todo.iter().position(|f| !matches!(f, Formula::Or(_))) {
        match todo.swap_remove(i) {
            Formula::True => {}
            Formula::False => return Some(true),
            Formula::And(fs) => todo.extend(fs),
            atom => atoms.push(atom),
        }
    }
    let Some(Formula::Or(alts)) = todo.pop() else {
        *cases += 1;
        if *cases > MAX_CASES {
            return None;
        }
        return impossible(atoms, kinds);
    };
    // If the atoms so far can't all hold, neither can any case of them.
    if impossible(atoms, kinds) == Some(true) {
        return Some(true);
    }
    for alt in alts {
        let mut todo = todo.clone();
        todo.push(alt);
        let mut atoms = atoms.clone();
        if !refuted(&mut todo, &mut atoms, kinds, cases)? {
            return Some(false);
        }
    }
    Some(true)
}

/// Whether the conjunction of `atoms` (each `Le` or `Eq`) has no integer
/// solution, as far as Fourier–Motzkin with rounding can tell: `None` if
/// it gave up.
fn impossible(atoms: &[Formula], kinds: &[Kind]) -> Option<bool> {
    let mut les: Vec<Lin> = Vec::new();
    let mut eqs: Vec<Lin> = Vec::new();
    for a in atoms {
        match a {
            Formula::Le(l) => les.push(l.clone()),
            Formula::Eq(l) => eqs.push(l.clone()),
            _ => unreachable!("atoms only"),
        }
    }
    // Each variable's range, and the ranges of the bounds those mention.
    let mut vars: Vec<Var> = Vec::new();
    for l in les.iter().chain(&eqs) {
        vars.extend(l.vars());
    }
    let mut i = 0;
    while i < vars.len() {
        let v = vars[i];
        i += 1;
        if vars[..i - 1].contains(&v) {
            continue;
        }
        let (lo, hi) = match &kinds[v as usize] {
            Kind::Int(lo, hi) => {
                vars.extend(lo.vars().chain(hi.vars()));
                (lo.clone(), hi.clone())
            }
            Kind::Bool => (Lin::constant(0), Lin::constant(1)),
            Kind::Bound(lo, hi) => (Lin::constant(*lo), Lin::constant(*hi)),
        };
        // lo - v <= 0 and v - hi <= 0
        les.push(lo.sub(&Lin::var(v))?);
        les.push(Lin::var(v).sub(&hi)?);
    }
    // Equations: one with a variable of coefficient ±1 says what that
    // variable is, everywhere. Others become two inequalities.
    while let Some(e) = eqs.pop() {
        let Some(e) = tighten_eq(e)? else {
            return Some(true);
        };
        if let Some(k) = e.as_constant() {
            if k != 0 {
                return Some(true);
            }
            continue;
        }
        match e.terms.iter().find(|&&(_, c)| c == 1 || c == -1) {
            Some(&(v, c)) => {
                // c·v + rest = 0, so v = -rest / c = -c·rest.
                let rest = Lin {
                    terms: e.terms.iter().copied().filter(|&(w, _)| w != v).collect(),
                    k: e.k,
                };
                let by = rest.scale(-c)?;
                for l in les.iter_mut().chain(eqs.iter_mut()) {
                    *l = l.substitute(v, &by)?;
                }
            }
            None => {
                les.push(e.clone());
                les.push(e.scale(-1)?);
            }
        }
    }
    eliminate(les)
}

/// `e == 0` divided through by the gcd of its coefficients, or `None`
/// inside if that leaves a fraction: no integers make it true.
fn tighten_eq(e: Lin) -> Option<Option<Lin>> {
    let g = e.terms.iter().fold(0, |g, &(_, c)| gcd(g, c));
    if g <= 1 {
        return Some(Some(e));
    }
    if e.k % g != 0 {
        return Some(None);
    }
    Some(Some(Lin {
        terms: e.terms.iter().map(|&(v, c)| (v, c / g)).collect(),
        k: e.k / g,
    }))
}

/// `l <= 0` divided through by the gcd of its coefficients, with the
/// constant rounded up, as integers allow: `2x - 3 <= 0` is `x - 1 <= 0`.
fn tighten(l: Lin) -> Lin {
    let g = l.terms.iter().fold(0, |g, &(_, c)| gcd(g, c));
    if g <= 1 {
        return l;
    }
    Lin {
        terms: l.terms.iter().map(|&(v, c)| (v, c / g)).collect(),
        k: l.k.div_euclid(g) + i128::from(l.k.rem_euclid(g) != 0),
    }
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Fourier–Motzkin on `les`, each `l <= 0`: `Some(true)` if it derives a
/// false constant.
fn eliminate(les: Vec<Lin>) -> Option<bool> {
    // Of constraints with the same variables and coefficients, only the
    // strongest matters: the one with the greatest constant.
    let mut set: HashMap<Vec<(Var, i128)>, i128> = HashMap::new();
    let add = |set: &mut HashMap<Vec<(Var, i128)>, i128>, l: Lin| -> bool {
        let l = tighten(l);
        if l.terms.is_empty() {
            return l.k > 0;
        }
        let e = set.entry(l.terms).or_insert(l.k);
        *e = (*e).max(l.k);
        false
    };
    for l in les {
        if add(&mut set, l) {
            return Some(true);
        }
    }
    loop {
        // The variable whose elimination makes fewest new constraints.
        let mut counts: BTreeMap<Var, (usize, usize)> = BTreeMap::new();
        for terms in set.keys() {
            for &(v, c) in terms {
                let e = counts.entry(v).or_default();
                if c > 0 { e.0 += 1 } else { e.1 += 1 }
            }
        }
        let Some((&v, _)) = counts
            .iter()
            .min_by_key(|&(_, &(p, n))| (p * n) as isize - (p + n) as isize)
        else {
            return Some(false);
        };
        let (mut uppers, mut lowers, mut rest) = (Vec::new(), Vec::new(), HashMap::new());
        for (terms, k) in set.drain() {
            let l = Lin { terms, k };
            match l.coeff(v) {
                c if c > 0 => uppers.push(l),
                c if c < 0 => lowers.push(l),
                _ => {
                    rest.insert(l.terms, l.k);
                }
            }
        }
        set = rest;
        for u in &uppers {
            for l in &lowers {
                // a·v + .. <= 0 and -b·v + .. <= 0 give b·(..) + a·(..) <= 0.
                let (a, b) = (u.coeff(v), -l.coeff(v));
                let combined = u.scale(b)?.add(&l.scale(a)?)?;
                if add(&mut set, combined) {
                    return Some(true);
                }
            }
        }
        if set.len() > MAX_CONSTRAINTS {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x() -> Lin {
        Lin::var(0)
    }

    fn y() -> Lin {
        Lin::var(1)
    }

    fn c(k: i128) -> Lin {
        Lin::constant(k)
    }

    fn i64() -> Kind {
        Kind::Int(c(i64::MIN as i128), c(i64::MAX as i128))
    }

    fn ints() -> Vec<Kind> {
        vec![i64(), i64(), i64()]
    }

    #[test]
    fn simple() {
        // x > 0 ⇒ x != 0
        let f = Formula::lt(&c(0), &x()).unwrap();
        let g = Formula::eq(&x(), &c(0)).unwrap().not();
        assert!(valid(std::slice::from_ref(&f), &g, &ints()));
        // x > 0 ⇏ x > 1
        let g = Formula::lt(&c(1), &x()).unwrap();
        assert!(!valid(&[f], &g, &ints()));
    }

    #[test]
    fn machine_range() {
        // x + 1 <= MAX doesn't follow from nothing, but from x < y.
        let g = Formula::le(&x().plus(1).unwrap(), &c(i64::MAX as i128)).unwrap();
        assert!(!valid(&[], &g, &ints()));
        let f = Formula::lt(&x(), &y()).unwrap();
        assert!(valid(&[f], &g, &ints()));
    }

    #[test]
    fn unknown_size() {
        // x: usize, whose greatest, y, is from 2^32 - 1 to 2^64 - 1.
        let kinds = &[
            Kind::Int(c(0), y()),
            Kind::Bound((1 << 32) - 1, (1 << 64) - 1),
        ];
        // x < 2^32 - 1 ⇒ x + 1 <= y, but x < 2^32 ⇏ x + 1 <= y.
        let g = Formula::le(&x().plus(1).unwrap(), &y()).unwrap();
        let f = Formula::lt(&x(), &c((1 << 32) - 1)).unwrap();
        assert!(valid(&[f], &g, kinds));
        let f = Formula::lt(&x(), &c(1 << 32)).unwrap();
        assert!(!valid(&[f], &g, kinds));
        // And nothing says x + 1 fits, without a fact.
        assert!(!valid(&[], &g, kinds));
    }

    #[test]
    fn integers_round() {
        // 2x == 1 has no integer solution, so anything about x follows.
        let anything = Formula::eq(&x(), &c(5)).unwrap();
        let f = Formula::eq(&x().scale(2).unwrap(), &c(1)).unwrap();
        assert!(valid(&[f], &anything, &ints()));
        // 1 <= 2x <= 1 neither, which only rounding sees.
        let lo = Formula::le(&c(1), &x().scale(2).unwrap()).unwrap();
        let hi = Formula::le(&x().scale(2).unwrap(), &c(1)).unwrap();
        assert!(valid(&[lo, hi], &anything, &ints()));
    }

    #[test]
    fn cases() {
        // x == 0 || x == 2 ⇒ x != 1
        let f = Formula::or([
            Formula::eq(&x(), &c(0)).unwrap(),
            Formula::eq(&x(), &c(2)).unwrap(),
        ]);
        let g = Formula::eq(&x(), &c(1)).unwrap().not();
        assert!(valid(std::slice::from_ref(&f), &g, &ints()));
        let g = Formula::eq(&x(), &c(2)).unwrap().not();
        assert!(!valid(&[f], &g, &ints()));
    }

    #[test]
    fn bools() {
        let kinds = &[Kind::Bool, i64()];
        // b ⇔ y > 0, b ⇒ y != 0
        let f = Formula::iff(Formula::holds(0), Formula::lt(&c(0), &y()).unwrap());
        let g = Formula::eq(&y(), &c(0)).unwrap().not();
        assert!(valid(&[f.clone(), Formula::holds(0)], &g, kinds));
        assert!(!valid(&[f], &g, kinds));
    }
}
