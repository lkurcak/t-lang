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
//! `valid(facts, goal)` proves that the goal follows from the facts. It
//! asks the opposite question, whether the facts and the goal's negation
//! can all hold at once. Only refuting every case establishes the goal;
//! otherwise it preserves why the proof failed. Failure does not refute
//! the goal. The procedure is *sound* but *incomplete*: it only ever proves
//! true things, but not every true thing. That's what it needs to be
//! trusted for: a weaker procedure rejects more programs, never accepts a
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
//! Each case's atoms remember which splits they come from, and so does
//! each constraint Fourier–Motzkin derives from them: a case refuted
//! without the alternative its last split took refutes the split's other
//! cases too, which are skipped (*backjumping*, `refuted`). And a fact said
//! twice is split once, and an atom with no variables is `true` or `false`
//! where it's made. So a disjunction a proof doesn't need costs one case,
//! not a doubling (docs/61-proof-search.md). The goal's negation is split
//! first, then what's known of its variables, then of theirs, and so on:
//! a goal the facts without disjunctions prove is proved before any of
//! the facts' disjunctions is split (`valid`, docs/62-t-chess-findings.md).
//!
//! Every step is bounded: cases, constraints and the size of numbers (with
//! checked `i128` arithmetic). A constraint whose numbers don't fit is left
//! out, which only weakens what's known; going past another bound gives up,
//! which is a "maybe". Either way the answer never depends on anything but
//! the question.

use std::collections::{BTreeMap, HashMap};

#[cfg(test)]
mod generated_tests;

/// A variable of the logic, by number (see `Kind`).
pub(crate) type Var = u32;

/// Why a proof was not established. None of these is a counterexample:
/// even `Unproved` can be a true goal this incomplete procedure misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Failure {
    Unproved,
    Unsupported,
    ArithmeticLimit,
    CaseLimit,
    ConstraintLimit,
}

impl Failure {
    pub(crate) fn explanation(self) -> &'static str {
        match self {
            Self::Unproved => {
                "the current proof procedure could not establish this condition from the available facts; this does not show that it is false"
            }
            Self::Unsupported => {
                "the condition could not be represented by the supported reasoning: linear arithmetic, and products of two unknowns"
            }
            Self::ArithmeticLimit => {
                "the proof exceeded the solver's checked integer arithmetic range"
            }
            Self::CaseLimit => "the proof exceeded the solver's limit of 512 cases",
            Self::ConstraintLimit => {
                "the proof exceeded the solver's limit of 400 constraints per case"
            }
        }
    }
}

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
    /// The product of two other variables, integers, the lesser numbered
    /// first: a variable as any other to the procedure, which knows of it
    /// only what `lemmas` says, from what's known of the two.
    Product(Var, Var),
}

/// The variable for `u · v`, made if there isn't one yet. There's one for
/// each pair, so `w · h` and `h · w` are the same value.
pub(crate) fn product(kinds: &mut Vec<Kind>, u: Var, v: Var) -> Var {
    let key = Kind::Product(u.min(v), u.max(v));
    if let Some(i) = kinds.iter().position(|k| *k == key) {
        return i as Var;
    }
    kinds.push(key);
    (kinds.len() - 1) as Var
}

/// `a · b`, if the logic has it: products of two variables, but none of
/// three. `None` too if a number gets too big; `Product` says which.
pub(crate) fn times(a: &Lin, b: &Lin, kinds: &mut Vec<Kind>) -> Result<Lin, Failure> {
    if let Some(k) = a.as_constant() {
        return b.scale(k).ok_or(Failure::ArithmeticLimit);
    }
    if let Some(k) = b.as_constant() {
        return a.scale(k).ok_or(Failure::ArithmeticLimit);
    }
    let is_product = |kinds: &[Kind], v: Var| matches!(kinds[v as usize], Kind::Product(..));
    if a.vars().chain(b.vars()).any(|v| is_product(kinds, v)) {
        return Err(Failure::Unsupported);
    }
    // (Σ ci·xi + k)(Σ dj·yj + l) = Σ ci·dj·xi·yj + l·Σ ci·xi + k·Σ dj·yj + k·l
    let limit = Failure::ArithmeticLimit;
    let mut sum = Lin::constant(a.k.checked_mul(b.k).ok_or(limit)?);
    let linear_a = Lin {
        terms: a.terms.clone(),
        k: 0,
    };
    let linear_b = Lin {
        terms: b.terms.clone(),
        k: 0,
    };
    sum = sum.add(&linear_a.scale(b.k).ok_or(limit)?).ok_or(limit)?;
    sum = sum.add(&linear_b.scale(a.k).ok_or(limit)?).ok_or(limit)?;
    for &(x, c) in &a.terms {
        for &(y, d) in &b.terms {
            let p = product(kinds, x, y);
            let term = Lin::var(p)
                .scale(c.checked_mul(d).ok_or(limit)?)
                .ok_or(limit)?;
            sum = sum.add(&term).ok_or(limit)?;
        }
    }
    Ok(sum)
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

    pub(crate) fn as_var(&self) -> Option<Var> {
        match self.terms.as_slice() {
            [(v, 1)] if self.k == 0 => Some(*v),
            _ => None,
        }
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// What it is, if it's an atom with no variables: `0 == 0` is true.
    fn constant(&self) -> Option<bool> {
        match self {
            Formula::Le(l) => Some(l.as_constant()? <= 0),
            Formula::Eq(l) => Some(l.as_constant()? == 0),
            _ => None,
        }
    }

    /// `true` or `false` for an atom with no variables, as `and` and `or`
    /// take them: a case that can't be taken is `false`, which the proof
    /// then needs no other fact to see (`relevant`).
    fn folded(self) -> Formula {
        match self.constant() {
            Some(true) => Formula::True,
            Some(false) => Formula::False,
            None => self,
        }
    }

    pub(crate) fn and(fs: impl IntoIterator<Item = Formula>) -> Formula {
        let mut out = Vec::new();
        for f in fs {
            match f.folded() {
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
            match f.folded() {
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

    pub(crate) fn vars(&self, out: &mut Vec<Var>) {
        match self {
            Formula::True | Formula::False => {}
            Formula::Le(l) | Formula::Eq(l) => out.extend(l.vars()),
            Formula::Not(f) => f.vars(out),
            Formula::And(fs) | Formula::Or(fs) => fs.iter().for_each(|f| f.vars(out)),
        }
    }

    /// A source-level condition, if all its values can be named here.
    /// Formatting uses magnitudes, so even i128::MIN needs no negation.
    pub(crate) fn text(&self, name: &impl Fn(Var) -> Option<String>) -> Option<String> {
        self.text_bounded(name, &mut 80)
    }

    fn text_bounded(
        &self,
        name: &impl Fn(Var) -> Option<String>,
        budget: &mut usize,
    ) -> Option<String> {
        *budget = budget.checked_sub(1)?;
        Some(match self {
            Self::True => "true".into(),
            Self::False => "false".into(),
            Self::Le(l) | Self::Eq(l) => {
                *budget = budget.checked_sub(l.terms.len())?;
                if let Some(k) = l.as_constant() {
                    return Some(
                        (if matches!(self, Self::Le(_)) {
                            k <= 0
                        } else {
                            k == 0
                        })
                        .to_string(),
                    );
                }
                let strict = matches!(self, Self::Le(_)) && l.k == 1;
                let (mut left, mut right) = (Vec::new(), Vec::new());
                for &(v, c) in &l.terms {
                    let n = name(v)?;
                    let term = match c.unsigned_abs() {
                        1 => n,
                        c => format!("{c} * {n}"),
                    };
                    if c > 0 {
                        left.push(term)
                    } else {
                        right.push(term)
                    }
                }
                if l.k != 0 && !strict {
                    if left.is_empty() && !right.is_empty() {
                        left.push(l.k.to_string())
                    } else if l.k > 0 {
                        right.push(format!("-{}", l.k))
                    } else {
                        right.push(l.k.unsigned_abs().to_string())
                    }
                }
                let side = |xs: Vec<String>| {
                    if xs.is_empty() {
                        "0".into()
                    } else {
                        xs.join(" + ").replace(" + -", " - ")
                    }
                };
                let op = if strict {
                    "<"
                } else if matches!(self, Self::Le(_)) {
                    "<="
                } else {
                    "=="
                };
                format!("{} {op} {}", side(left), side(right))
            }
            Self::Not(f) => {
                let text = f.text_bounded(name, budget)?;
                if text == "true" {
                    return Some("false".into());
                }
                if text == "false" {
                    return Some("true".into());
                }
                match &**f {
                    Self::Eq(_) => text.replace(" == ", " != "),
                    Self::Le(l) if l.k == 1 => text.replace(" < ", " >= "),
                    Self::Le(_) => text.replace(" <= ", " > "),
                    _ => format!("!({text})"),
                }
            }
            Self::And(fs) | Self::Or(fs) => {
                let and = matches!(self, Self::And(_));
                let (identity, stop) = if and {
                    ("true", "false")
                } else {
                    ("false", "true")
                };
                let mut parts = Vec::new();
                for f in fs {
                    let text = f.text_bounded(name, budget)?;
                    if text == stop {
                        return Some(text);
                    }
                    if text != identity {
                        parts.push(if matches!(f, Self::And(_) | Self::Or(_)) {
                            format!("({text})")
                        } else {
                            text
                        });
                    }
                }
                if parts.is_empty() {
                    identity.into()
                } else {
                    parts.join(if and { " && " } else { " || " })
                }
            }
        })
    }

    /// The same statement with negations only on atoms, and those turned
    /// into atoms too: `!(l <= 0)` is `-l + 1 <= 0`, for integers, and
    /// `!(l == 0)` is `l < 0 || l > 0`. `None` if a number gets too big.
    fn nnf(&self, negate: bool) -> Option<Formula> {
        Some(match (self, negate) {
            (Formula::True, false) | (Formula::False, true) => Formula::True,
            (Formula::True, true) | (Formula::False, false) => Formula::False,
            (Formula::Le(l), n) if l.terms.is_empty() => {
                if (l.k <= 0) != n {
                    Formula::True
                } else {
                    Formula::False
                }
            }
            (Formula::Eq(l), n) if l.terms.is_empty() => {
                if (l.k == 0) != n {
                    Formula::True
                } else {
                    Formula::False
                }
            }
            (Formula::Le(l), false) => Formula::Le(l.clone()),
            (Formula::Le(l), true) => Formula::Le(l.scale(-1)?.plus(1)?),
            (Formula::Eq(l), false) => Formula::Eq(l.clone()),
            (Formula::Eq(l), true) => {
                Formula::or([Formula::Le(l.plus(1)?), Formula::Le(l.scale(-1)?.plus(1)?)])
            }
            (Formula::Not(f), n) => f.nnf(!n)?,
            (Formula::And(fs), false) | (Formula::Or(fs), true) => {
                let v = fs
                    .iter()
                    .map(|f| f.nnf(negate))
                    .collect::<Option<Vec<_>>>()?;
                match Formula::and(v) {
                    Formula::And(gs) => Formula::And(dedup(gs)),
                    f => f,
                }
            }
            (Formula::And(fs), true) | (Formula::Or(fs), false) => {
                let v = fs
                    .iter()
                    .map(|f| f.nnf(negate))
                    .collect::<Option<Vec<_>>>()?;
                match Formula::or(v) {
                    Formula::Or(gs) => Formula::Or(dedup(gs)),
                    f => f,
                }
            }
        })
    }
}

fn dedup(fs: Vec<Formula>) -> Vec<Formula> {
    let mut seen = std::collections::HashSet::new();
    fs.into_iter().filter(|f| seen.insert(f.clone())).collect()
}

/// The most cases `valid` looks at before giving up.
const MAX_CASES: usize = 512;

thread_local! {
    /// How much `valid` has done on this thread (`work`).
    static WORK: std::cell::Cell<Work> =
        const { std::cell::Cell::new(Work { refutations: 0, cases: 0, carried: 0 }) };
}

/// How much checking refinements has done on this thread: how many
/// conjunctions Fourier–Motzkin was run on, how many cases were looked at,
/// and how big the carried refinements said of values were, summed over
/// each time one was (`targs`). Each is what the program asked, not how
/// fast the machine is, so tests can bound them (`tests/proof_work.rs`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Work {
    pub refutations: u64,
    pub cases: u64,
    pub carried: u64,
}

/// What's been done on this thread since it started.
pub fn work() -> Work {
    WORK.with(|w| w.get())
}

pub(super) fn count(f: impl FnOnce(&mut Work)) {
    WORK.with(|w| {
        let mut v = w.get();
        f(&mut v);
        w.set(v);
    })
}

/// The most constraints one case may grow to while variables are
/// eliminated.
const MAX_CONSTRAINTS: usize = 400;

/// Proves `goal` from `facts`, where variable `v` is of kind `kinds[v]`.
/// Only `Ok(())` establishes it; an error never establishes its negation.
pub(crate) fn valid(facts: &[Formula], goal: &Formula, kinds: &[Kind]) -> Result<(), Failure> {
    let (parts, layer) = connected(facts, goal, kinds);
    let known = Formula::and(parts.into_iter().cloned())
        .nnf(false)
        .ok_or(Failure::ArithmeticLimit)?;
    let denied = goal.nnf(true).ok_or(Failure::ArithmeticLimit)?;
    // What's split on first is what the goal says, then what's said of its
    // variables, then of theirs, and so on (`refuted` splits the last
    // disjunction first): a proof that needs none of the facts'
    // disjunctions needs none of their cases.
    let mut known = match known {
        Formula::And(fs) => fs,
        f => vec![f],
    };
    let distance = |f: &Formula| {
        let mut vs = Vec::new();
        f.vars(&mut vs);
        vs.iter()
            .filter_map(|v| layer.get(v))
            .min()
            .copied()
            .unwrap_or(0)
    };
    known.sort_by_key(|f| std::cmp::Reverse(distance(f)));
    let mut todo: Vec<(Formula, Deps)> = known.into_iter().map(|f| (f, 0)).collect();
    todo.push((denied, 0));
    let mut cases = 0;
    let mut kinds = kinds.to_vec();
    match refuted(&mut todo, &mut Vec::new(), &mut kinds, &mut cases, 0, false)? {
        Some(_) => Ok(()),
        None => Err(Failure::Unproved),
    }
}

/// Facts connected to the goal by shared variables, in their original
/// order. Used both by the proof and its explanation.
pub(crate) fn relevant<'a>(
    facts: &'a [Formula],
    goal: &Formula,
    kinds: &[Kind],
) -> Vec<&'a Formula> {
    connected(facts, goal, kinds).0
}

/// The facts `relevant` gives, and how far each variable they mention is
/// from the goal: 0 if the goal mentions it, 1 if a fact mentioning one
/// of those does, and so on.
fn connected<'a>(
    facts: &'a [Formula],
    goal: &Formula,
    kinds: &[Kind],
) -> (Vec<&'a Formula>, HashMap<Var, usize>) {
    // Only the facts that share variables with the goal, or with facts that
    // do, and so on, can matter: the rest can be made true on their own.
    // A fact with no variables at all, `false` on a path that can't be
    // taken, matters by itself. A product shares its factors' variables:
    // what bounds them bounds it (`lemmas`).
    let with_factors = |vs: &mut Vec<Var>| {
        let mut i = 0;
        while i < vs.len() {
            if let Some(Kind::Product(u, v)) = kinds.get(vs[i] as usize) {
                vs.extend([*u, *v]);
            }
            i += 1;
        }
    };
    let mut wanted = Vec::new();
    goal.vars(&mut wanted);
    with_factors(&mut wanted);
    let vars_of: Vec<Vec<Var>> = facts
        .iter()
        .map(|f| {
            let mut vs = Vec::new();
            f.vars(&mut vs);
            with_factors(&mut vs);
            vs
        })
        .collect();
    let mut taken = vec![false; facts.len()];
    let mut layer: HashMap<Var, usize> = wanted.iter().map(|&v| (v, 0)).collect();
    // Layer by layer: the facts that mention a variable of the layers so
    // far give the next one.
    for depth in 1.. {
        let mut next = Vec::new();
        let mut took = false;
        for (i, vs) in vars_of.iter().enumerate() {
            if !taken[i] && (vs.is_empty() || vs.iter().any(|v| layer.contains_key(v))) {
                taken[i] = true;
                took = true;
                next.extend(vs.iter().copied());
            }
        }
        if !took {
            break;
        }
        for v in next {
            layer.entry(v).or_insert(depth);
        }
    }
    let facts = facts
        .iter()
        .zip(&taken)
        .filter(|&(_, &t)| t)
        .map(|(f, _)| f)
        .collect();
    (facts, layer)
}

/// The most lemmas `lemmas` adds to one proof.
const MAX_LEMMAS: usize = 64;

/// How many lemmas deep `lemmas` goes: a product a lemma makes gets lemmas
/// of its own, and so on, this many times.
const MAX_DEPTH: usize = 2;

/// The most factors of unknown sign `lemmas` splits a proof on.
const MAX_SPLITS: usize = 3;

/// Facts about the products in a case, a conjunction of `atoms`, each
/// true of every integer, so adding them proves nothing false: for a
/// product `u · v` and a bound `u <= L` among the atoms, if the atoms
/// prove `v >= 0` then `u · v <= L · v`, and if they prove `v <= 0` then
/// `u · v >= L · v`; the same for `u >= L`, `u == L`, the constant bounds
/// of `u`'s type, and with `u` and `v` swapped. `L · v` is expanded, which
/// may make products the logic hadn't had (in `kinds`), but not of three
/// variables: those lemmas are left out. A square isn't negative. For a
/// factor of unknown sign, the case is split on its sign, into one where
/// it's at least 0 and one where it's at most 0, but only for the first
/// few: each doubles the cases.
///
/// The bounds are the atoms', never other lemmas'. A product a lemma makes
/// gets lemmas too, `MAX_DEPTH` deep, and there are at most `MAX_LEMMAS`:
/// so it ends, and the answer depends on nothing but the question.
fn lemmas(atoms: &[Formula], kinds: &mut Vec<Kind>) -> Vec<Formula> {
    let mut vars = Vec::new();
    for f in atoms {
        f.vars(&mut vars);
    }
    // Each product, its factors, and how many lemmas deep it was made:
    // 0 for those in the atoms.
    let mut products: Vec<(Var, Var, Var, usize)> = Vec::new();
    for &p in &vars {
        if let Kind::Product(u, v) = kinds[p as usize]
            && !products.iter().any(|&(q, ..)| q == p)
        {
            products.push((p, u, v, 0));
        }
    }
    if products.is_empty() {
        return Vec::new();
    }
    // The bounds the atoms state of each factor: (`L`, which way), with
    // `Ordering::Less` for `u <= L`.
    use std::cmp::Ordering::{self, Equal, Greater, Less};
    let bounds = |u: Var, kinds: &[Kind]| -> Vec<(Lin, Ordering)> {
        let mut out = Vec::new();
        for a in atoms {
            let (Formula::Le(l) | Formula::Eq(l)) = a else {
                continue;
            };
            // c·u + rest <= 0, c = ±1: u <= -rest, or u >= rest.
            let c = l.coeff(u);
            if c != 1 && c != -1 {
                continue;
            }
            let Some(rest) = Lin::var(u).scale(c).and_then(|cu| l.sub(&cu)) else {
                continue;
            };
            let Some(bound) = rest.scale(-c) else {
                continue;
            };
            let way = match (a, c) {
                (Formula::Eq(_), _) => Equal,
                (_, 1) => Less,
                _ => Greater,
            };
            out.push((bound, way));
        }
        let range = match &kinds[u as usize] {
            Kind::Int(lo, hi) => (lo.as_constant(), hi.as_constant()),
            Kind::Bound(lo, hi) => (Some(*lo), Some(*hi)),
            _ => (None, None),
        };
        if let Some(lo) = range.0 {
            out.push((Lin::constant(lo), Greater));
        }
        if let Some(hi) = range.1 {
            out.push((Lin::constant(hi), Less));
        }
        out
    };
    // The sign the atoms prove of a factor: `v >= 0` if they can't hold
    // with `v <= -1`.
    let sign = |v: Var, kinds: &[Kind]| -> Option<Ordering> {
        let x = Lin::var(v);
        let with = |f: Formula| {
            let mut all: Vec<(Formula, Deps)> = atoms.iter().map(|a| (a.clone(), 0)).collect();
            all.push((f, 0));
            matches!(impossible(&all, kinds), Ok(Some(_)))
        };
        if with(Formula::Le(x.plus(1)?)) {
            Some(Greater)
        } else if with(Formula::Le(x.scale(-1)?.plus(1)?)) {
            Some(Less)
        } else {
            None
        }
    };
    // By the factor `b` they're about: the lemmas that hold if `b >= 0`,
    // and those that hold if `b <= 0`. Those that hold either way are
    // `out`.
    let mut out = Vec::new();
    let mut by_sign: BTreeMap<Var, (Vec<Formula>, Vec<Formula>)> = BTreeMap::new();
    let mut count = 0;
    let mut next = 0;
    while let Some(&(p, u, v, depth)) = products.get(next) {
        next += 1;
        let pv = Lin::var(p);
        if u == v {
            // A square isn't negative.
            out.extend(Formula::le(&Lin::constant(0), &pv));
        }
        for (a, b) in [(u, v), (v, u)] {
            for (bound, way) in bounds(a, kinds) {
                if count >= MAX_LEMMAS {
                    break;
                }
                // a <= L and b >= 0 give a·b <= L·b; b <= 0 turns it round.
                let Ok(lb) = times(&bound, &Lin::var(b), kinds) else {
                    continue;
                };
                count += 1;
                // A product the lemma makes, as `L · b` for an `L` that
                // names another factor, gets lemmas too: `h.width() ==
                // g.width()` and `g.width() == 2` say what `h.width() *
                // h.height()` is only in two steps.
                for w in lb.vars() {
                    if let Kind::Product(x, y) = kinds[w as usize]
                        && depth < MAX_DEPTH
                        && !products.iter().any(|&(q, ..)| q == w)
                    {
                        products.push((w, x, y, depth + 1));
                    }
                }
                let (pos, neg) = by_sign.entry(b).or_default();
                match way {
                    Equal => out.extend(Formula::eq(&pv, &lb)),
                    Less => {
                        pos.extend(Formula::le(&pv, &lb));
                        neg.extend(Formula::le(&lb, &pv));
                    }
                    Greater => {
                        pos.extend(Formula::le(&lb, &pv));
                        neg.extend(Formula::le(&pv, &lb));
                    }
                }
            }
            if u == v {
                break;
            }
        }
    }
    // A factor whose sign the facts don't say is split on, into the case
    // where it's at least 0 and the one where it's at most 0, but only for
    // the first few: each doubles the cases.
    let mut splits = 0;
    for (b, (pos, neg)) in by_sign {
        match sign(b, kinds) {
            Some(Greater) => out.extend(pos),
            Some(_) => out.extend(neg),
            None if splits < MAX_SPLITS => {
                splits += 1;
                let zero = Lin::constant(0);
                let x = Lin::var(b);
                let (Some(nonneg), Some(nonpos)) = (Formula::le(&zero, &x), Formula::le(&x, &zero))
                else {
                    continue;
                };
                out.push(Formula::or([
                    Formula::and(std::iter::once(nonneg).chain(pos)),
                    Formula::and(std::iter::once(nonpos).chain(neg)),
                ]));
            }
            None => {}
        }
    }
    out
}

/// The case splits a statement depends on, by depth: bit `d` is the
/// alternative taken at the `d`th split of the current case. Splits past
/// the 128th get no bit, so nothing is skipped there (`refuted`).
type Deps = u128;

/// The bit for the split at `depth`, if it has one.
fn marker(depth: usize) -> Deps {
    if depth < 128 { 1 << depth } else { 0 }
}

/// Whether every case of `todo` with `atoms` is impossible: `Some(deps)`
/// if it found that each is, where `deps` are the splits above that this
/// depends on, `None` if one may not be, an error if it gave up. `todo`
/// is in negation normal form, each with the splits it depends on, as is
/// each atom.
///
/// *Backjumping*: if a case of a split is refuted without what its
/// alternative says, so is every other case of it, the same way, and
/// they're skipped. Irrelevant disjunctions then cost one case each, not
/// a doubling (Horrocks's dependency-directed backtracking).
fn refuted(
    todo: &mut Vec<(Formula, Deps)>,
    atoms: &mut Vec<(Formula, Deps)>,
    kinds: &mut Vec<Kind>,
    cases: &mut usize,
    depth: usize,
    lemmas_added: bool,
) -> Result<Option<Deps>, Failure> {
    // Atoms and `&&` first, so a case is as narrow as can be before it's
    // split.
    while let Some(i) = todo.iter().position(|(f, _)| !matches!(f, Formula::Or(_))) {
        match todo.remove(i) {
            (Formula::True, _) => {}
            (Formula::False, d) => return Ok(Some(d)),
            (Formula::And(fs), d) => todo.extend(fs.into_iter().map(|f| (f, d))),
            atom => atoms.push(atom),
        }
    }
    let Some((Formula::Or(alts), or_deps)) = todo.pop() else {
        // A case: what its atoms say of the products in it, once, may
        // refute it where the atoms alone don't.
        if !lemmas_added {
            let plain: Vec<Formula> = atoms.iter().map(|(f, _)| f.clone()).collect();
            let more = lemmas(&plain, kinds);
            if !more.is_empty() {
                if let Ok(Some(d)) = impossible(atoms, kinds) {
                    return Ok(Some(d));
                }
                // Lemmas follow from the atoms: all of them, to be safe.
                let all = atoms.iter().fold(0, |d, (_, e)| d | e);
                let mut more = more.into_iter().map(|f| (f, all)).collect();
                return refuted(&mut more, atoms, kinds, cases, depth, true);
            }
        }
        *cases += 1;
        count(|w| w.cases += 1);
        if *cases > MAX_CASES {
            return Err(Failure::CaseLimit);
        }
        return impossible(atoms, kinds);
    };
    // If the atoms so far can't all hold, neither can any case of them.
    if let Ok(Some(d)) = impossible(atoms, kinds) {
        return Ok(Some(d));
    }
    let mark = marker(depth);
    let mut deps = or_deps;
    for alt in alts {
        let mut todo = todo.clone();
        todo.push((alt, or_deps | mark));
        let mut atoms = atoms.clone();
        match refuted(&mut todo, &mut atoms, kinds, cases, depth + 1, lemmas_added)? {
            None => return Ok(None),
            Some(d) if mark != 0 && d & mark == 0 => return Ok(Some(d)),
            Some(d) => deps |= d & !mark,
        }
    }
    Ok(Some(deps))
}

/// Whether the conjunction of `atoms` (each `Le` or `Eq`) has no integer
/// solution, as far as Fourier–Motzkin with rounding can tell, preserving
/// why it gave up: `Some` with the splits of the atoms it used.
fn impossible(atoms: &[(Formula, Deps)], kinds: &[Kind]) -> Result<Option<Deps>, Failure> {
    count(|w| w.refutations += 1);
    // A constraint whose numbers get too big is left out: what's left
    // follows from the atoms still, so a contradiction found is a real
    // one. But not finding one is then said to be why.
    let mut dropped = false;
    let mut les: Vec<(Lin, Deps)> = Vec::new();
    let mut eqs: Vec<(Lin, Deps)> = Vec::new();
    for (a, d) in atoms {
        match a {
            Formula::Le(l) => les.push((l.clone(), *d)),
            Formula::Eq(l) => eqs.push((l.clone(), *d)),
            _ => unreachable!("atoms only"),
        }
    }
    // Each variable's range, and the ranges of the bounds those mention.
    let mut vars: Vec<Var> = Vec::new();
    for (l, _) in les.iter().chain(&eqs) {
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
            // A product of integers is an integer, with no range of its own.
            Kind::Product(..) => continue,
        };
        // lo - v <= 0 and v - hi <= 0, which hold of every case.
        for l in [lo.sub(&Lin::var(v)), Lin::var(v).sub(&hi)] {
            match l {
                Some(l) => les.push((l, 0)),
                None => dropped = true,
            }
        }
    }
    // Equations: one with a variable of coefficient ±1 says what that
    // variable is, everywhere. Others become two inequalities.
    while let Some((e, d)) = eqs.pop() {
        let Some(e) = tighten_eq(e) else {
            dropped = true;
            continue;
        };
        let Some(e) = e else {
            return Ok(Some(d));
        };
        if let Some(k) = e.as_constant() {
            if k != 0 {
                return Ok(Some(d));
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
                let Some(by) = rest.scale(-c) else {
                    dropped = true;
                    continue;
                };
                // What mentions `v` is said of `by` instead, or left out.
                let subst = |xs: &mut Vec<(Lin, Deps)>, dropped: &mut bool| {
                    xs.retain_mut(|(l, ld)| {
                        if l.coeff(v) == 0 {
                            return true;
                        }
                        *ld |= d;
                        match l.substitute(v, &by) {
                            Some(m) => {
                                *l = m;
                                true
                            }
                            None => {
                                *dropped = true;
                                false
                            }
                        }
                    })
                };
                subst(&mut les, &mut dropped);
                subst(&mut eqs, &mut dropped);
            }
            None => match e.scale(-1) {
                Some(neg) => {
                    les.push((e, d));
                    les.push((neg, d));
                }
                None => dropped = true,
            },
        }
    }
    match eliminate(les)? {
        (Some(d), _) => Ok(Some(d)),
        (None, more) if dropped || more => Err(Failure::ArithmeticLimit),
        (None, _) => Ok(None),
    }
}

/// `e == 0` divided through by the gcd of its coefficients, or `None`
/// inside if that leaves a fraction: no integers make it true.
fn tighten_eq(e: Lin) -> Option<Option<Lin>> {
    let g = e.terms.iter().try_fold(0, |g, &(_, c)| gcd(g, c))?;
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
fn tighten(l: Lin) -> Option<Lin> {
    let g = l.terms.iter().try_fold(0, |g, &(_, c)| gcd(g, c))?;
    if g <= 1 {
        return Some(l);
    }
    Some(Lin {
        terms: l.terms.iter().map(|&(v, c)| (v, c / g)).collect(),
        k: l.k.div_euclid(g) + i128::from(l.k.rem_euclid(g) != 0),
    })
}

fn gcd(a: i128, b: i128) -> Option<i128> {
    // A coefficient can be i128::MIN even though its magnitude doesn't
    // fit. Give up like every other arithmetic limit, in all build modes.
    let (mut a, mut b) = (a.checked_abs()?, b.checked_abs()?);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    Some(a)
}

/// Fourier–Motzkin on `les`, each `l <= 0` with the splits it depends
/// on: `Some(deps)` if it derives a false constant from statements
/// depending on `deps`; and whether it left out a constraint whose
/// numbers got too big.
fn eliminate(les: Vec<(Lin, Deps)>) -> Result<(Option<Deps>, bool), Failure> {
    // Of constraints with the same variables and coefficients, only the
    // strongest matters: the one with the greatest constant, and of equal
    // ones, either.
    type Set = HashMap<Vec<(Var, i128)>, (i128, Deps)>;
    let mut set: Set = HashMap::new();
    let mut dropped = false;
    // A constraint that's false by itself, with what it depends on.
    let add = |set: &mut Set, dropped: &mut bool, l: Option<Lin>, d: Deps| -> Option<Deps> {
        let Some(l) = l.and_then(tighten) else {
            *dropped = true;
            return None;
        };
        if l.terms.is_empty() {
            return (l.k > 0).then_some(d);
        }
        let e = set.entry(l.terms).or_insert((l.k, d));
        if l.k > e.0 || (l.k == e.0 && d.count_ones() < e.1.count_ones()) {
            *e = (l.k, d);
        }
        None
    };
    for (l, d) in les {
        if let Some(d) = add(&mut set, &mut dropped, Some(l), d) {
            return Ok((Some(d), dropped));
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
            return Ok((None, dropped));
        };
        let (mut uppers, mut lowers, mut rest) = (Vec::new(), Vec::new(), HashMap::new());
        for (terms, (k, d)) in set.drain() {
            let l = Lin { terms, k };
            match l.coeff(v) {
                c if c > 0 => uppers.push((l, d)),
                c if c < 0 => lowers.push((l, d)),
                _ => {
                    rest.insert(l.terms, (l.k, d));
                }
            }
        }
        set = rest;
        for (u, ud) in &uppers {
            for (l, ld) in &lowers {
                // a·v + .. <= 0 and -b·v + .. <= 0 give b·(..) + a·(..) <= 0.
                let (a, b) = (u.coeff(v), l.coeff(v).checked_neg());
                let combined = b.and_then(|b| u.scale(b)?.add(&l.scale(a)?));
                if let Some(d) = add(&mut set, &mut dropped, combined, ud | ld) {
                    return Ok((Some(d), dropped));
                }
            }
        }
        if set.len() > MAX_CONSTRAINTS {
            return Err(Failure::ConstraintLimit);
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
        assert_eq!(valid(std::slice::from_ref(&f), &g, &ints()), Ok(()));
        // x > 0 ⇏ x > 1
        let g = Formula::lt(&c(1), &x()).unwrap();
        assert_eq!(valid(&[f], &g, &ints()), Err(Failure::Unproved));
    }

    #[test]
    fn arithmetic_limit_is_not_an_unproved_goal() {
        let goal = Formula::eq(&x(), &c(1)).unwrap();
        let fact = Formula::Le(x().scale(i128::MIN).unwrap());
        assert_eq!(
            valid(&[fact], &goal, &ints()),
            Err(Failure::ArithmeticLimit)
        );
        // Negation normalisation can overflow before elimination starts.
        let goal = Formula::Le(x().plus(i128::MIN).unwrap());
        assert_eq!(valid(&[], &goal, &ints()), Err(Failure::ArithmeticLimit));
    }

    #[test]
    fn case_limit_preserves_the_acceptance_boundary() {
        // x is one of 0, 1, .., n - 1: each a case of its own.
        let fact =
            |n: usize| Formula::or((0..n).map(|i| Formula::eq(&x(), &c(i as i128)).unwrap()));
        let goal = Formula::eq(&x(), &c(-1)).unwrap().not();
        assert_eq!(valid(&[fact(MAX_CASES)], &goal, &ints()), Ok(()));
        assert_eq!(
            valid(&[fact(MAX_CASES + 1)], &goal, &ints()),
            Err(Failure::CaseLimit)
        );
    }

    #[test]
    fn constraint_limit_is_reported_through_valid() {
        let mut facts = Vec::new();
        for a in 0..31 {
            for b in a + 1..31 {
                facts.push(Formula::Le(Lin::var(a).add(&Lin::var(b)).unwrap()));
            }
        }
        assert_eq!(
            valid(&facts, &Formula::Le(x()), &vec![i64(); 31]),
            Err(Failure::ConstraintLimit)
        );
    }

    #[test]
    fn readable_conditions_include_extreme_coefficients() {
        let names = |v| {
            Some(if v == 0 {
                "index".into()
            } else {
                "items.len()".into()
            })
        };
        assert_eq!(
            Formula::lt(&x(), &y()).unwrap().text(&names).unwrap(),
            "index < items.len()"
        );
        assert_eq!(
            Formula::Le(x().scale(i128::MIN).unwrap())
                .text(&names)
                .unwrap(),
            "0 <= 170141183460469231731687303715884105728 * index"
        );
    }

    /// Runs `valid`, and says how many cases it looked at.
    fn cases_of(facts: &[Formula], goal: &Formula, kinds: &[Kind]) -> (Result<(), Failure>, u64) {
        let (r, w) = work_of(facts, goal, kinds);
        (r, w.cases)
    }

    /// Runs `valid`, and says how much it did.
    fn work_of(facts: &[Formula], goal: &Formula, kinds: &[Kind]) -> (Result<(), Failure>, Work) {
        let before = work();
        let r = valid(facts, goal, kinds);
        let after = work();
        let w = Work {
            refutations: after.refutations - before.refutations,
            cases: after.cases - before.cases,
            carried: 0,
        };
        (r, w)
    }

    #[test]
    fn the_same_disjunction_twice_is_split_once() {
        // x == 0 || x == 2, said 30 times: still two cases, not 2^30.
        let f = Formula::or([
            Formula::eq(&x(), &c(0)).unwrap(),
            Formula::eq(&x(), &c(2)).unwrap(),
        ]);
        let g = Formula::eq(&x(), &c(1)).unwrap().not();
        let (r, n) = cases_of(&vec![f; 30], &g, &ints());
        assert_eq!(r, Ok(()));
        assert!(n <= 2, "{n} cases");
    }

    #[test]
    fn constant_atoms_are_true_or_false() {
        // x == 0 || 0 == 1 || 0 == 1 || ..: one case; and x == 1 || 0 == 0
        // is true, whatever x is.
        let mut alts = vec![Formula::eq(&x(), &c(0)).unwrap()];
        alts.extend((0..MAX_CASES).map(|_| Formula::Eq(c(1))));
        let g = Formula::eq(&x(), &c(1)).unwrap().not();
        let (r, n) = cases_of(&[Formula::Or(alts)], &g, &ints());
        assert_eq!(r, Ok(()));
        assert!(n <= 1, "{n} cases");
        let t = Formula::Or(vec![Formula::eq(&x(), &c(1)).unwrap(), Formula::Eq(c(0))]);
        assert_eq!(valid(&[], &t, &ints()), Ok(()));
    }

    #[test]
    fn a_case_that_cant_be_taken_needs_no_other_fact() {
        // y == 3 && 0 == 1 can't hold, whatever y is: so anything follows,
        // x == 5 too, though x isn't in it (`relevant` keeps a fact with no
        // variables, which it is once `and` has folded `0 == 1`).
        let f = Formula::and([Formula::eq(&y(), &c(3)).unwrap(), Formula::Eq(c(1))]);
        let g = Formula::eq(&x(), &c(5)).unwrap();
        assert_eq!(valid(&[f], &g, &ints()), Ok(()));
    }

    #[test]
    fn disjunctions_a_proof_doesnt_use_cost_a_case_each() {
        // x == 5, and 40 disjunctions about x and other numbers that have
        // nothing to do with why x == 5 (x <= y_i || x >= y_i + 3): proving
        // x == 5 splits on x < 5 || x > 5, and each of those cases is
        // refuted without any of them, so their other cases aren't looked
        // at (backjumping).
        let n = 40;
        let mut kinds = vec![i64(); n + 1];
        kinds[0] = i64();
        let y = |i: usize| Lin::var(i as Var + 1);
        let mut facts: Vec<Formula> = (0..n)
            .map(|i| {
                Formula::or([
                    Formula::le(&x(), &y(i)).unwrap(),
                    Formula::le(&y(i).plus(3).unwrap(), &x()).unwrap(),
                ])
            })
            .collect();
        facts.insert(n / 2, Formula::eq(&x(), &c(5)).unwrap());
        let g = Formula::eq(&x(), &c(5)).unwrap();
        let (r, w) = work_of(&facts, &g, &kinds);
        assert_eq!(r, Ok(()));
        // Each split is refuted by its first case: two runs of
        // Fourier–Motzkin a split, not two to the number of splits.
        assert!(w.cases <= 4 && w.refutations <= 4 * n as u64, "{w:?}");
    }

    #[test]
    fn a_split_inside_an_alternative_depends_on_it() {
        // x == 0, and (z == 1 && (x == 1 || x == 2)) || y == 5: the first
        // alternative can't hold, but the second can, so y != 5 doesn't
        // follow. Refuting the first's cases uses x == 1 and x == 2, which
        // are there only because the first was taken: so the split on it
        // isn't skipped past.
        let z = || Lin::var(2);
        let f = Formula::or([
            Formula::and([
                Formula::eq(&z(), &c(1)).unwrap(),
                Formula::or([
                    Formula::eq(&x(), &c(1)).unwrap(),
                    Formula::eq(&x(), &c(2)).unwrap(),
                ]),
            ]),
            Formula::eq(&y(), &c(5)).unwrap(),
        ]);
        let facts = [Formula::eq(&x(), &c(0)).unwrap(), f];
        let g = Formula::eq(&y(), &c(5)).unwrap().not();
        assert_eq!(valid(&facts, &g, &ints()), Err(Failure::Unproved));
        // And y == 5 does.
        let g = Formula::eq(&y(), &c(5)).unwrap();
        assert_eq!(valid(&facts, &g, &ints()), Ok(()));
    }

    #[test]
    fn a_number_too_big_leaves_out_its_constraint_not_the_proof() {
        // 2^100·x + y <= 0 and x's range can't be combined in i128, which
        // eliminating x, first, would do: so that's left out, and z <= 5
        // and z >= 6 refute without it.
        let z = || Lin::var(2);
        let big = Formula::Le(x().scale(1 << 100).unwrap().add(&y()).unwrap());
        let atoms = |fs: Vec<Formula>| fs.into_iter().map(|f| (f, 0)).collect::<Vec<_>>();
        let refuted = atoms(vec![
            big.clone(),
            Formula::le(&z(), &c(5)).unwrap(),
            Formula::le(&c(6), &z()).unwrap(),
        ]);
        assert_eq!(impossible(&refuted, &ints()), Ok(Some(0)));
        // Not refuted, having left something out: that's said to be why.
        let open = atoms(vec![big, Formula::le(&z(), &c(5)).unwrap()]);
        assert_eq!(impossible(&open, &ints()), Err(Failure::ArithmeticLimit));
    }

    #[test]
    fn machine_range() {
        // x + 1 <= MAX doesn't follow from nothing, but from x < y.
        let g = Formula::le(&x().plus(1).unwrap(), &c(i64::MAX as i128)).unwrap();
        assert!(valid(&[], &g, &ints()).is_err());
        let f = Formula::lt(&x(), &y()).unwrap();
        assert_eq!(valid(&[f], &g, &ints()), Ok(()));
    }

    #[test]
    fn extreme_coefficients_do_not_prove_false_goals() {
        let goal = Formula::eq(&x(), &c(1)).unwrap();
        for coefficient in [i128::MIN, i128::MIN + 1, i128::MAX] {
            let term = x().scale(coefficient).unwrap();
            for fact in [Formula::Le(term.clone()), Formula::Eq(term)] {
                // x = 0 satisfies each fact and falsifies the goal.
                assert!(valid(&[fact], &goal, &ints()).is_err());
            }
        }
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
        assert_eq!(valid(&[f], &g, kinds), Ok(()));
        let f = Formula::lt(&x(), &c(1 << 32)).unwrap();
        assert!(valid(&[f], &g, kinds).is_err());
        // And nothing says x + 1 fits, without a fact.
        assert!(valid(&[], &g, kinds).is_err());
    }

    #[test]
    fn integers_round() {
        // 2x == 1 has no integer solution, so anything about x follows.
        let anything = Formula::eq(&x(), &c(5)).unwrap();
        let f = Formula::eq(&x().scale(2).unwrap(), &c(1)).unwrap();
        assert_eq!(valid(&[f], &anything, &ints()), Ok(()));
        // 1 <= 2x <= 1 neither, which only rounding sees.
        let lo = Formula::le(&c(1), &x().scale(2).unwrap()).unwrap();
        let hi = Formula::le(&x().scale(2).unwrap(), &c(1)).unwrap();
        assert_eq!(valid(&[lo, hi], &anything, &ints()), Ok(()));
    }

    #[test]
    fn cases() {
        // x == 0 || x == 2 ⇒ x != 1
        let f = Formula::or([
            Formula::eq(&x(), &c(0)).unwrap(),
            Formula::eq(&x(), &c(2)).unwrap(),
        ]);
        let g = Formula::eq(&x(), &c(1)).unwrap().not();
        assert_eq!(valid(std::slice::from_ref(&f), &g, &ints()), Ok(()));
        let g = Formula::eq(&x(), &c(2)).unwrap().not();
        assert!(valid(&[f], &g, &ints()).is_err());
    }

    #[test]
    fn bools() {
        let kinds = &[Kind::Bool, i64()];
        // b ⇔ y > 0, b ⇒ y != 0
        let f = Formula::iff(Formula::holds(0), Formula::lt(&c(0), &y()).unwrap());
        let g = Formula::eq(&y(), &c(0)).unwrap().not();
        assert_eq!(valid(&[f.clone(), Formula::holds(0)], &g, kinds), Ok(()));
        assert!(valid(&[f], &g, kinds).is_err());
    }
}
