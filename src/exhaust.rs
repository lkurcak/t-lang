//! Exhaustiveness and reachability of `match` arms, using the *usefulness*
//! algorithm from Luc Maranget, "Warnings for pattern matching" (2007). It's
//! what rustc and OCaml use, minus their extensions.
//!
//! Everything reduces to one question:
//!
//! > Given patterns `p1..pn` (the rows), is there a value that pattern `q`
//! > matches but none of the rows do? If so, `q` is *useful*.
//!
//! - Arm `i` is **unreachable** if its pattern isn't useful after arms `0..i`.
//!   The same goes for each alternative of an or-pattern, after the arms and
//!   alternatives tried before it (see `redundant`).
//! - A match is **exhaustive** if `_` isn't useful after all the arms. If it
//!   is, the value it finds is the counterexample in the error message.
//!
//! Patterns are compared by *constructor*: the thing a value was built with,
//! like `true`, `Shape::Rect` or `Point`. A type's constructors are finite for
//! bool, `()`, structs and enums, but not for int (practically speaking), and
//! functions have none we can look inside.

use crate::error::Span;
use crate::types::{AdtDef, Adts, Type};

/// A pattern reduced to what matters for coverage. Bindings become wildcards
/// (`x` matches anything, just like `_`) and struct fields are put in
/// declaration order, with omitted ones filled in as wildcards.
#[derive(Debug, Clone)]
pub enum Pat {
    Wild,
    Ctor(Ctor, Vec<Pat>),
    /// Matches if any alternative does. Each alternative keeps its source
    /// span, to point at it if it's unreachable.
    Or(Vec<(Pat, Span)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctor {
    Bool(bool),
    Unit,
    Int(i64),
    /// A struct has a single constructor.
    Struct,
    /// The n-th variant of an enum.
    Variant(usize),
}

/// A part of a `match` that can never be the one that matches.
#[derive(Debug, PartialEq)]
pub enum Redundant {
    /// The arm at this index: earlier arms match everything it would.
    Arm(usize),
    /// An alternative of an or-pattern, by its span: earlier arms and earlier
    /// alternatives match everything it would.
    Alt(Span),
}

/// Finds the first arm, or else the first or-pattern alternative, that can
/// never match.
///
/// Checking only whole arms misses `Some(1 | 1)`, and checking each
/// or-pattern only against its own earlier alternatives misses the `1` in
/// `Some(1) => .., Some(1 | 2) | None => ..`. So each arm is expanded into
/// every combination of its alternatives, in the order they'd be tried
/// (`Some(1)`, `Some(2)`, `None`), and each combination is checked against
/// all the earlier arms and combinations. An alternative is unreachable if
/// no combination that uses it is useful, and an arm is if none of its
/// combinations are.
///
/// That's exponential in the number of or-patterns in one arm, but real
/// arms have few.
pub fn redundant(adts: &Adts, ty: &Type, arms: &[Pat]) -> Option<Redundant> {
    let cx = Cx { adts };
    let tys = [ty.clone()];
    let mut rows: Vec<Vec<Pat>> = Vec::new();
    for (i, arm) in arms.iter().enumerate() {
        let mut arm_useful = false;
        // Every alternative this arm uses, and the ones some useful
        // combination used.
        let mut alts: Vec<Span> = Vec::new();
        let mut useful_alts: Vec<Span> = Vec::new();
        for (p, chosen) in expand(arm) {
            let useful = cx.useful(&rows, std::slice::from_ref(&p), &tys).is_some();
            arm_useful |= useful;
            for span in chosen {
                if !alts.contains(&span) {
                    alts.push(span);
                }
                if useful && !useful_alts.contains(&span) {
                    useful_alts.push(span);
                }
            }
            rows.push(vec![p]);
        }
        if !arm_useful {
            return Some(Redundant::Arm(i));
        }
        let dead = alts.into_iter().filter(|s| !useful_alts.contains(s));
        if let Some(span) = dead.min_by_key(|s| s.start) {
            return Some(Redundant::Alt(span));
        }
    }
    None
}

/// Every or-free version of `p`, one per combination of alternatives, in the
/// order they'd be tried. Each comes with the spans of the alternatives it
/// chose.
fn expand(p: &Pat) -> Vec<(Pat, Vec<Span>)> {
    match p {
        Pat::Wild => vec![(Pat::Wild, Vec::new())],
        Pat::Or(alts) => alts
            .iter()
            .flat_map(|(alt, span)| {
                expand(alt).into_iter().map(move |(p, mut chosen)| {
                    chosen.push(*span);
                    (p, chosen)
                })
            })
            .collect(),
        Pat::Ctor(c, args) => {
            // The cartesian product of the fields' expansions.
            let mut combos = vec![(Vec::new(), Vec::new())];
            for arg in args {
                let options = expand(arg);
                let mut next = Vec::new();
                for (fields, chosen) in &combos {
                    for (p, more) in &options {
                        let mut fields: Vec<Pat> = fields.clone();
                        fields.push(p.clone());
                        next.push((fields, [chosen.clone(), more.clone()].concat()));
                    }
                }
                combos = next;
            }
            combos
                .into_iter()
                .map(|(fields, chosen)| (Pat::Ctor(*c, fields), chosen))
                .collect()
        }
    }
}

/// A value, written as a pattern, that none of the `arms` match. `None` means
/// the arms are exhaustive.
pub fn missing(adts: &Adts, ty: &Type, arms: &[Pat]) -> Option<String> {
    let cx = Cx { adts };
    let rows: Vec<Vec<Pat>> = arms.iter().map(|p| vec![p.clone()]).collect();
    let witness = cx.useful(&rows, &[Pat::Wild], std::slice::from_ref(ty))?;
    Some(cx.show(&witness[0], ty))
}

struct Cx<'a> {
    adts: &'a Adts,
}

impl Cx<'_> {
    /// Is the pattern vector `q` useful with respect to `rows`? Every row and
    /// `q` have one pattern per column, and column `j` has type `tys[j]`.
    ///
    /// Returns a *witness* if so: values for each column (as patterns) that
    /// `q` matches and no row does.
    ///
    /// It works one column at a time, peeling the first column off:
    ///
    /// - `q` starts with constructor `c`: only rows that could start with `c`
    ///   matter. *Specialize* by `c` (keep those rows, replacing `c`'s pattern
    ///   with its fields' patterns) and recurse.
    /// - `q` starts with `_` and the rows' first column mentions *every*
    ///   constructor of the type (that can hold a value): `_` is useful iff
    ///   it's useful as one of them, so try each.
    /// - `q` starts with `_` and some constructor is never mentioned: any value
    ///   built with it gets past every row that has a constructor there, so
    ///   only the rows starting with `_` can stop `q`. Keep just those (the
    ///   *default* matrix) and recurse on the remaining columns.
    fn useful(&self, rows: &[Vec<Pat>], q: &[Pat], tys: &[Type]) -> Option<Vec<Pat>> {
        let Some((ty, rest_tys)) = tys.split_first() else {
            // No columns left: every row matches whatever got here, so `q` is
            // useful exactly when no row made it this far.
            return rows.is_empty().then(Vec::new);
        };
        // A row starting with `a | b` matches whatever a row starting with `a`
        // or one starting with `b` would, so split it into both.
        let expanded;
        let rows = if rows.iter().any(|row| matches!(row[0], Pat::Or(_))) {
            expanded = expand_or(rows);
            &expanded[..]
        } else {
            rows
        };
        match &q[0] {
            // `redundant` expands or-patterns before asking, and `missing`
            // asks about `_`, so `q` never holds one.
            Pat::Or(_) => unreachable!("or-pattern in `q`"),
            Pat::Ctor(c, args) => {
                let fields = self.fields(*c, ty);
                let arity = fields.len();
                let rows = specialize(rows, *c, arity);
                let q: Vec<Pat> = args.iter().chain(&q[1..]).cloned().collect();
                let tys = [fields, rest_tys.to_vec()].concat();
                let mut witness = self.useful(&rows, &q, &tys)?;
                let rest = witness.split_off(arity);
                Some([vec![Pat::Ctor(*c, witness)], rest].concat())
            }
            Pat::Wild => {
                let seen: Vec<Ctor> = rows
                    .iter()
                    .filter_map(|row| match &row[0] {
                        Pat::Ctor(c, _) => Some(*c),
                        Pat::Wild => None,
                        Pat::Or(_) => unreachable!("expanded above"),
                    })
                    .collect();
                let all = self.all_ctors(ty);
                // Constructors no row mentions. Ones that can't hold a value,
                // like `Maybe::Impossible(Never)`, don't count: no value is
                // built with them, so none can slip past the rows that way.
                let unseen: Option<Vec<Ctor>> = all.as_ref().map(|all| {
                    let unseen = all.iter().filter(|c| !seen.contains(c));
                    unseen
                        .filter(|c| self.inhabited_ctor(**c, ty))
                        .copied()
                        .collect()
                });
                match (all, unseen) {
                    // Note that an enum with no variants always lands here,
                    // and then there's nothing to try: `_` is useless because
                    // there are no values at all.
                    (Some(all), Some(unseen)) if unseen.is_empty() => {
                        all.into_iter().find_map(|c| {
                            let wilds = vec![Pat::Wild; self.fields(c, ty).len()];
                            let q: Vec<Pat> = [vec![Pat::Ctor(c, wilds)], q[1..].to_vec()].concat();
                            self.useful(rows, &q, tys)
                        })
                    }
                    (_, unseen) => {
                        let defaults: Vec<Vec<Pat>> = rows
                            .iter()
                            .filter(|row| matches!(row[0], Pat::Wild))
                            .map(|row| row[1..].to_vec())
                            .collect();
                        let rest = self.useful(&defaults, &q[1..], rest_tys)?;
                        // Name a constructor nobody mentioned, if we can. If no
                        // row had a constructor here, `_` says it better.
                        let head = match unseen {
                            Some(unseen) if !seen.is_empty() => {
                                let c = unseen[0];
                                Pat::Ctor(c, vec![Pat::Wild; self.fields(c, ty).len()])
                            }
                            _ => Pat::Wild,
                        };
                        Some([vec![head], rest].concat())
                    }
                }
            }
        }
    }

    /// Every constructor of `ty`, or `None` if there are too many to list
    /// (int) or none that patterns can name (functions).
    fn all_ctors(&self, ty: &Type) -> Option<Vec<Ctor>> {
        match ty {
            Type::Bool => Some(vec![Ctor::Bool(false), Ctor::Bool(true)]),
            Type::Unit => Some(vec![Ctor::Unit]),
            Type::Adt(name) => Some(match &self.adts[name] {
                AdtDef::Struct(_) => vec![Ctor::Struct],
                AdtDef::Enum(variants) => (0..variants.len()).map(Ctor::Variant).collect(),
            }),
            Type::Int | Type::Fn(..) => None,
        }
    }

    /// Does `ty` have any values at all? `enum Never {}` doesn't, and neither
    /// does anything that must hold a `Never`.
    ///
    /// This terminates because types can't contain themselves yet. Recursive
    /// types (step 5) will need more care: `enum Stream { Next(int, Stream) }`
    /// has no finite values.
    fn inhabited(&self, ty: &Type) -> bool {
        match self.all_ctors(ty) {
            Some(all) => all.into_iter().any(|c| self.inhabited_ctor(c, ty)),
            None => true,
        }
    }

    /// Can constructor `c` build a value? Only if every field has a value.
    fn inhabited_ctor(&self, c: Ctor, ty: &Type) -> bool {
        self.fields(c, ty).iter().all(|t| self.inhabited(t))
    }

    /// The types of the fields that constructor `c` of type `ty` holds.
    fn fields(&self, c: Ctor, ty: &Type) -> Vec<Type> {
        match (c, ty) {
            (Ctor::Bool(_) | Ctor::Unit | Ctor::Int(_), _) => Vec::new(),
            (Ctor::Struct, Type::Adt(name)) => match &self.adts[name] {
                AdtDef::Struct(fields) => fields.iter().map(|(_, t)| t.clone()).collect(),
                AdtDef::Enum(_) => unreachable!("struct pattern on enum `{name}`"),
            },
            (Ctor::Variant(i), Type::Adt(name)) => match &self.adts[name] {
                AdtDef::Enum(variants) => variants[i].1.clone(),
                AdtDef::Struct(_) => unreachable!("variant pattern on struct `{name}`"),
            },
            _ => unreachable!("constructor {c:?} doesn't belong to `{ty}`"),
        }
    }

    /// Writes a witness back out in source syntax, for error messages.
    fn show(&self, p: &Pat, ty: &Type) -> String {
        let Pat::Ctor(c, args) = p else {
            // Witnesses are built from `_` and constructors only.
            return "_".to_string();
        };
        let field_tys = self.fields(*c, ty);
        let args: Vec<String> = args
            .iter()
            .zip(&field_tys)
            .map(|(p, t)| self.show(p, t))
            .collect();
        match (c, ty) {
            (Ctor::Bool(b), _) => b.to_string(),
            (Ctor::Unit, _) => "()".to_string(),
            (Ctor::Int(n), _) => n.to_string(),
            (Ctor::Struct, Type::Adt(name)) => {
                let AdtDef::Struct(fields) = &self.adts[name] else {
                    unreachable!()
                };
                if fields.is_empty() {
                    return format!("{name} {{}}");
                }
                let fields: Vec<String> = fields
                    .iter()
                    .zip(&args)
                    .map(|((f, _), a)| format!("{f}: {a}"))
                    .collect();
                format!("{name} {{ {} }}", fields.join(", "))
            }
            (Ctor::Variant(i), Type::Adt(name)) => {
                let AdtDef::Enum(variants) = &self.adts[name] else {
                    unreachable!()
                };
                let variant = &variants[*i].0;
                if args.is_empty() {
                    format!("{name}::{variant}")
                } else {
                    format!("{name}::{variant}({})", args.join(", "))
                }
            }
            _ => unreachable!("constructor {c:?} doesn't belong to `{ty}`"),
        }
    }
}

/// Replaces each row starting with `a | b | ..` with one row per alternative.
/// (The parser never nests an or-pattern directly inside another, and ones
/// nested deeper, like `Some(1 | 2)`, reach the first column after
/// specializing and are split then.)
fn expand_or(rows: &[Vec<Pat>]) -> Vec<Vec<Pat>> {
    let mut out = Vec::new();
    for row in rows {
        match &row[0] {
            Pat::Or(alts) => {
                out.extend(
                    alts.iter()
                        .map(|(a, _)| [vec![a.clone()], row[1..].to_vec()].concat()),
                );
            }
            _ => out.push(row.clone()),
        }
    }
    out
}

/// Keeps the rows whose first pattern could match constructor `c`, with that
/// pattern replaced by `c`'s `arity` field patterns (all `_` if it was `_`).
fn specialize(rows: &[Vec<Pat>], c: Ctor, arity: usize) -> Vec<Vec<Pat>> {
    rows.iter()
        .filter_map(|row| {
            let fields = match &row[0] {
                Pat::Ctor(d, args) if *d == c => args.clone(),
                Pat::Ctor(..) => return None,
                Pat::Wild => vec![Pat::Wild; arity],
                Pat::Or(_) => unreachable!("expanded before specializing"),
            };
            Some([fields, row[1..].to_vec()].concat())
        })
        .collect()
}
