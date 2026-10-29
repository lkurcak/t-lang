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
//! bool, `()`, structs and enums, and functions and type parameters have none
//! we can look inside.
//!
//! Ints are the odd one out: there are too many to list one by one, so their
//! constructors are *ranges*, and `5` is just `5..=5`. Ranges can overlap, so
//! before comparing them, `split` cuts them at every boundary the rows
//! mention. With rows `0..=9` and `5..`, the ints become `..=-1`, `0..=4`,
//! `5..=9` and `10..`, and each piece lies wholly inside or outside each row's
//! range. From there they work like any other constructors. This is what rustc
//! does too.

use crate::ast::Lit;
use crate::error::Span;
use crate::types::{AdtKind, Adts, Type, struct_fields, variant_fields};

/// A pattern reduced to what matters for coverage. Bindings become wildcards
/// (`x` matches anything, just like `_`) and struct fields are put in
/// declaration order, with omitted ones filled in as wildcards.
#[derive(Debug, Clone)]
pub enum Pat {
    Wild,
    Ctor(Ctor, Vec<Pat>),
    /// A range pattern `lo..=hi`, with its source span to point at if it can
    /// be narrowed. `useful` never sees one: `plain` turns it into
    /// `Ctor::Int(lo, hi)` first. (A single int is a `Ctor::Int` from the
    /// start. It can't be narrowed, only be unreachable.)
    Range(i128, i128, Span, Ends),
    /// Matches if any alternative does. Each alternative keeps its source
    /// span, to point at it if it's unreachable.
    Or(Vec<(Pat, Span)>),
}

/// The least and greatest values of an integer type, which a range with an
/// end left open runs to.
pub type Ends = (i128, i128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctor {
    Bool(bool),
    Unit,
    /// The ints from the first to the second, inclusive.
    Int(i128, i128),
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
    /// A range pattern, by its span, that only ever matches `lo..=hi`, a
    /// narrower range: earlier arms and alternatives match the ints at one or
    /// both of its ends. Ints they match in the middle are fine, as in
    /// `5 => .., 0..10 => ..`, since there's no narrower range to write.
    Trim(Span, Option<i128>, Option<i128>),
}

/// Finds every arm and or-pattern alternative that can never match, and every
/// range that could be narrower, in the order of the arms. Within an arm, a
/// dead alternative comes before a range to narrow, and an unreachable arm
/// reports nothing else.
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
///
/// A range is narrowed to the ints some useful combination can still match
/// with it (see `reached`), from the first to the last.
pub fn redundant(adts: &Adts, ty: &Type, arms: &[Pat]) -> Vec<Redundant> {
    let cx = Cx { adts };
    let mut found = Vec::new();
    let tys = [ty.clone()];
    let mut rows: Vec<Vec<Pat>> = Vec::new();
    for (i, arm) in arms.iter().enumerate() {
        let mut arm_useful = false;
        // Every alternative this arm uses, and the ones some useful
        // combination used.
        let mut alts: Vec<Span> = Vec::new();
        let mut useful_alts: Vec<Span> = Vec::new();
        let mut ranges: Vec<Reach> = Vec::new();
        for (p, chosen) in expand(arm) {
            let q = plain(&p, None);
            let useful = cx.useful(&rows, std::slice::from_ref(&q), &tys).is_some();
            arm_useful |= useful;
            for span in chosen {
                if !alts.contains(&span) {
                    alts.push(span);
                }
                if useful && !useful_alts.contains(&span) {
                    useful_alts.push(span);
                }
            }
            if useful {
                for (lo, hi, span, ends) in ranges_in(&p) {
                    let (a, b) = cx.reached(&rows, ty, &p, lo, hi, span);
                    match ranges.iter_mut().find(|r| r.span == span) {
                        Some(r) => r.reached = (r.reached.0.min(a), r.reached.1.max(b)),
                        None => ranges.push(Reach {
                            span,
                            ends,
                            written: (lo, hi),
                            reached: (a, b),
                        }),
                    }
                }
            }
            rows.push(vec![q]);
        }
        if !arm_useful {
            found.push(Redundant::Arm(i));
            continue;
        }
        let mut dead: Vec<Span> = alts
            .into_iter()
            .filter(|s| !useful_alts.contains(s))
            .collect();
        dead.sort_by_key(|s| s.start);
        found.extend(dead.into_iter().map(Redundant::Alt));
        let mut wide: Vec<Reach> = ranges
            .into_iter()
            .filter(|r| r.written != r.reached)
            .collect();
        wide.sort_by_key(|r| r.span.start);
        found.extend(wide.into_iter().map(|r| {
            let (lo, hi) = open(r.reached.0, r.reached.1, r.ends);
            Redundant::Trim(r.span, lo, hi)
        }));
    }
    found
}

/// A range pattern in one arm, as written and as far as useful combinations
/// of the arm reach with it. A range in several combinations reaches the
/// union, from the lowest int any of them reaches to the highest.
struct Reach {
    span: Span,
    ends: Ends,
    written: (i128, i128),
    reached: (i128, i128),
}

/// `p` with its ranges turned into plain constructors, for `useful`. The one
/// at `narrow`'s span, if any, becomes `narrow`'s constructor instead.
fn plain(p: &Pat, narrow: Option<(Span, Ctor)>) -> Pat {
    match p {
        Pat::Wild => Pat::Wild,
        Pat::Range(lo, hi, span, _) => match narrow {
            Some((s, c)) if s == *span => Pat::Ctor(c, Vec::new()),
            _ => Pat::Ctor(Ctor::Int(*lo, *hi), Vec::new()),
        },
        Pat::Ctor(c, args) => Pat::Ctor(*c, args.iter().map(|a| plain(a, narrow)).collect()),
        Pat::Or(alts) => Pat::Or(alts.iter().map(|(a, s)| (plain(a, narrow), *s)).collect()),
    }
}

/// The ranges in an or-free pattern, as `(lo, hi, span)`.
fn ranges_in(p: &Pat) -> Vec<(i128, i128, Span, Ends)> {
    match p {
        Pat::Wild => Vec::new(),
        Pat::Range(lo, hi, span, ends) => vec![(*lo, *hi, *span, *ends)],
        Pat::Ctor(_, args) => args.iter().flat_map(ranges_in).collect(),
        Pat::Or(_) => unreachable!("expanded before looking for ranges"),
    }
}

/// Every int constructor in an or-free, plain pattern.
fn ints_in(p: &Pat, out: &mut Vec<Ctor>) {
    if let Pat::Ctor(c, args) = p {
        if let Ctor::Int(..) = c {
            out.push(*c);
        }
        for a in args {
            ints_in(a, out);
        }
    }
}

/// Every or-free version of `p`, one per combination of alternatives, in the
/// order they'd be tried. Each comes with the spans of the alternatives it
/// chose.
fn expand(p: &Pat) -> Vec<(Pat, Vec<Span>)> {
    match p {
        Pat::Wild | Pat::Range(..) => vec![(p.clone(), Vec::new())],
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
    let rows: Vec<Vec<Pat>> = arms.iter().map(|p| vec![plain(p, None)]).collect();
    let witness = cx.useful(&rows, &[Pat::Wild], std::slice::from_ref(ty))?;
    Some(cx.show(&witness[0], ty))
}

/// Does `ty` have no values only because each would hold another value of
/// a type it's inside, without end? Like `struct Stream { next:
/// Box<Stream> }`: it would have some if values could be infinite, as
/// they can in a lazy language, where that's a stream.
pub fn endless(adts: &Adts, ty: &Type) -> bool {
    let cx = Cx { adts };
    !cx.inhabited_in(ty, &mut Vec::new(), false) && cx.inhabited_in(ty, &mut Vec::new(), true)
}

struct Cx<'a> {
    adts: &'a Adts,
}

impl Cx<'_> {
    /// The first and last ints of the range `lo..=hi` at `span` in `p` that
    /// `p` can match with no row matching too. `p` must be useful, so there
    /// are some.
    ///
    /// The range is `split` at every int boundary anywhere in the rows, so
    /// that each piece is wholly inside or outside every range the rows have
    /// at its position. Then all the ints in a piece are reached or none are,
    /// and asking about one piece at a time is enough.
    fn reached(
        &self,
        rows: &[Vec<Pat>],
        ty: &Type,
        p: &Pat,
        lo: i128,
        hi: i128,
        span: Span,
    ) -> (i128, i128) {
        let mut seen = Vec::new();
        for row in rows {
            ints_in(&row[0], &mut seen);
        }
        let pieces = split(lo, hi, &seen);
        let tys = std::slice::from_ref(ty);
        let live = |c: &&Ctor| {
            let q = plain(p, Some((span, **c)));
            self.useful(rows, std::slice::from_ref(&q), tys).is_some()
        };
        match (pieces.iter().find(live), pieces.iter().rev().find(live)) {
            (Some(Ctor::Int(a, _)), Some(Ctor::Int(_, b))) => (*a, *b),
            _ => unreachable!("`p` is useful, so some piece of each range is"),
        }
    }

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
            Pat::Or(_) | Pat::Range(..) => unreachable!("or-pattern or range in `q`"),
            Pat::Ctor(c, args) => {
                // A range in `q` may overlap a row's only in part, like
                // `0..=9` and `5..`. Then `q` is useful iff one of its pieces
                // is, and each piece is either covered by a row or not.
                let pieces = match *c {
                    Ctor::Int(lo, hi) => split(lo, hi, &head_ctors(rows)),
                    c => vec![c],
                };
                pieces.into_iter().find_map(|c| {
                    let fields = self.fields(c, ty);
                    let arity = fields.len();
                    let rows = specialize(rows, c, arity);
                    let q: Vec<Pat> = args.iter().chain(&q[1..]).cloned().collect();
                    let tys = [fields, rest_tys.to_vec()].concat();
                    let mut witness = self.useful(&rows, &q, &tys)?;
                    let rest = witness.split_off(arity);
                    Some([vec![Pat::Ctor(c, witness)], rest].concat())
                })
            }
            Pat::Wild => {
                let seen = head_ctors(rows);
                let all = self.all_ctors(ty, &seen);
                // Constructors no row mentions. Ones that can't hold a value,
                // like `Maybe::Impossible(Never)`, don't count: no value is
                // built with them, so none can slip past the rows that way.
                let unseen: Option<Vec<Ctor>> = all.as_ref().map(|all| {
                    let unseen = all.iter().filter(|c| !seen.iter().any(|d| covers(*d, **c)));
                    unseen
                        .filter(|c| self.inhabited_ctor(**c, ty))
                        .copied()
                        .collect()
                });
                match (all, unseen) {
                    // Note that an enum with no variants always lands here,
                    // and then there's nothing to try: `_` is useless because
                    // there are no values at all.
                    // Only the constructors that can build a value can
                    // build one that gets past the rows.
                    (Some(all), Some(unseen)) if unseen.is_empty() => {
                        let mut all = all.into_iter().filter(|c| self.inhabited_ctor(*c, ty));
                        all.find_map(|c| {
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

    /// Every constructor of `ty`, or `None` if there are none that patterns
    /// can name (functions, and types the checker hasn't inferred because
    /// every pattern was `_` or a variable). For int, that's every int, split into ranges at
    /// the boundaries of the ones in `seen`.
    ///
    /// A reference has the constructors of what it points to: patterns look
    /// through it (*match ergonomics*). A box has none: patterns can't.
    fn all_ctors(&self, ty: &Type, seen: &[Ctor]) -> Option<Vec<Ctor>> {
        match peel(ty) {
            // All of them, on every machine: a `usize` may be 64 bits wide.
            Type::Int(t) => {
                let (lo, hi) = t.range_64();
                Some(split(lo, hi, seen))
            }
            // The scalar values: all but the surrogates.
            Type::Char => {
                let mut all = split(0, 0xD7FF, seen);
                all.extend(split(0xE000, 0x10FFFF, seen));
                Some(all)
            }
            Type::Bool => Some(vec![Ctor::Bool(false), Ctor::Bool(true)]),
            Type::Unit => Some(vec![Ctor::Unit]),
            Type::Never => Some(Vec::new()),
            Type::Adt(name, ..) => Some(match &self.adts[name].kind {
                AdtKind::Struct(_) => vec![Ctor::Struct],
                AdtKind::Enum(variants) => (0..variants.len()).map(Ctor::Variant).collect(),
            }),
            // An array of a known length is like a struct with that many
            // fields; one of a length a const parameter says has no
            // pattern but `_` and variables.
            Type::Array(_, n) if matches!(**n, Type::Nat(_)) => Some(vec![Ctor::Struct]),
            Type::Array(..) => None,
            // Nothing is known about a type parameter's values: it could be
            // anything.
            // Nor about an opaque type's, outside the function that picks it.
            Type::Fn(..)
            | Type::Param(_)
            | Type::Nat(_)
            | Type::Proj(..)
            | Type::Opaque(..)
            | Type::Var(_)
            | Type::Box(_)
            | Type::Slice(_)
            | Type::Str => None,
            Type::Ref(..) => unreachable!("peeled"),
            Type::Effects(_) | Type::Alloc | Type::Panic | Type::Unsafe | Type::Inferred(..) => {
                unreachable!("effects have no values")
            }
        }
    }

    /// Can constructor `c` build a value? Only if every field has a value.
    /// `enum Never {}` has none, and neither does anything that must hold a
    /// `Never`, a reference to one or a box with one in it included: a
    /// reference always points to a value.
    fn inhabited_ctor(&self, c: Ctor, ty: &Type) -> bool {
        self.inhabited_ctor_in(c, ty, &mut Vec::new(), false)
    }

    /// Does `ty` have any values, inside the types in `building`, each of which is being
    /// built a value of, and that holds the next.
    ///
    /// A type can hold itself, through a reference or a box, and a value of
    /// it has to stop doing that somewhere, since it's finite. So a value
    /// of a type that's already being built can't be needed to build one:
    /// a smaller one, with the inner value in place of the outer, would do.
    /// `struct Stream { next: Box<Stream> }` has no values at all. Types
    /// can't grow as they hold themselves (no polymorphic recursion), so
    /// there are only so many to build, and this ends.
    ///
    /// That a `&Never` can't exist is a promise unsafe code will have to
    /// keep too, once there is any: Rust doesn't make it, so it asks for
    /// arms here that T doesn't.
    ///
    /// With `endless`, a type already being built is taken to have values
    /// instead, as if values could be infinite: then only an empty enum
    /// makes a type have none.
    fn inhabited_in(&self, ty: &Type, building: &mut Vec<Type>, endless: bool) -> bool {
        if let Type::Ref(_, _, t) | Type::Box(t) = ty {
            return self.inhabited_in(t, building, endless);
        }
        if building.contains(ty) {
            return endless;
        }
        let Some(all) = self.all_ctors(ty, &[]) else {
            return true;
        };
        building.push(ty.clone());
        let found = all
            .into_iter()
            .any(|c| self.inhabited_ctor_in(c, ty, building, endless));
        building.pop();
        found
    }

    fn inhabited_ctor_in(
        &self,
        c: Ctor,
        ty: &Type,
        building: &mut Vec<Type>,
        endless: bool,
    ) -> bool {
        self.fields(c, ty)
            .iter()
            .all(|t| self.inhabited_in(t, building, endless))
    }

    /// The types of the fields that constructor `c` of type `ty` holds.
    fn fields(&self, c: Ctor, ty: &Type) -> Vec<Type> {
        let ty = peel(ty);
        match c {
            Ctor::Bool(_) | Ctor::Unit | Ctor::Int(..) => Vec::new(),
            Ctor::Struct if let Type::Array(elem, n) = ty => match **n {
                Type::Nat(k) => vec![(**elem).clone(); k as usize],
                _ => unreachable!("array pattern on `{ty}`"),
            },
            Ctor::Struct => struct_fields(self.adts, ty)
                .unwrap_or_else(|| unreachable!("struct pattern on `{ty}`"))
                .into_iter()
                .map(|(_, t)| t)
                .collect(),
            Ctor::Variant(i) => variant_fields(self.adts, ty, i),
        }
    }

    /// Writes a witness back out in source syntax, for error messages.
    fn show(&self, p: &Pat, ty: &Type) -> String {
        let ty = peel(ty);
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
            (Ctor::Int(lo, hi), Type::Int(t)) => {
                let (lo, hi) = open(*lo, *hi, t.range_64());
                show_range(lo, hi, true, Lit::Int)
            }
            (Ctor::Int(lo, hi), Type::Char) => {
                let (lo, hi) = open(*lo, *hi, (0, 0x10FFFF));
                show_range(lo, hi, true, Lit::Char)
            }
            (Ctor::Int(lo, hi), _) => show_range(Some(*lo), Some(*hi), true, Lit::Int),
            (Ctor::Struct, Type::Array(..)) => format!("[{}]", args.join(", ")),
            (Ctor::Struct, Type::Adt(symbol, ..)) => {
                let name = crate::check::source_name(symbol);
                let fields = struct_fields(self.adts, ty).expect("a struct");
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
            (Ctor::Variant(i), Type::Adt(symbol, ..)) => {
                let AdtKind::Enum(variants) = &self.adts[symbol].kind else {
                    unreachable!()
                };
                let name = crate::check::source_name(symbol);
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

/// What `ty` points to, if it's a reference, at any depth: patterns look
/// through references to it.
fn peel(ty: &Type) -> &Type {
    match ty {
        Type::Ref(_, _, t) => peel(t),
        t => t,
    }
}

/// Writes the ints `lo..=hi` as a pattern: `5`, `1..=9` (or `1..10` if not
/// `inclusive`), `10..`, `..=-1` (or `..0`), or `_` for all of them.
///
/// Each end is written as a `lit`: `b'a'..=b'z'`, `'a'..='z'`. A range
/// of characters ending just before the surrogates is written with `..=`,
/// as the character after it would be one.
pub fn show_range(lo: Option<i128>, hi: Option<i128>, inclusive: bool, lit: Lit) -> String {
    let start = lo.map_or(String::new(), |lo| lit.show(lo));
    match (lo, hi) {
        (Some(lo), Some(hi)) if lo == hi => lit.show(lo),
        (None, None) => "_".to_string(),
        (_, None) => format!("{start}.."),
        (_, Some(hi)) if inclusive || (lit == Lit::Char && hi == 0xD7FF) => {
            format!("{start}..={}", lit.show(hi))
        }
        (_, Some(hi)) => format!("{start}..{}", lit.show(hi + 1)),
    }
}

/// The ints `lo..=hi` of a type whose values are `ends`, with an end
/// that's the type's left open, as it can be written: `200..` of a `u8`.
/// Unless it's just one.
fn open(lo: i128, hi: i128, (min, max): Ends) -> (Option<i128>, Option<i128>) {
    match lo < hi {
        true => ((lo != min).then_some(lo), (hi != max).then_some(hi)),
        false => (Some(lo), Some(hi)),
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

/// The constructors the rows start with. They must not start with an
/// or-pattern.
fn head_ctors(rows: &[Vec<Pat>]) -> Vec<Ctor> {
    rows.iter()
        .filter_map(|row| match &row[0] {
            Pat::Ctor(c, _) => Some(*c),
            Pat::Wild => None,
            Pat::Or(_) | Pat::Range(..) => {
                unreachable!("expanded and made plain before looking at constructors")
            }
        })
        .collect()
}

/// Does a row starting with constructor `d` match every value built with `c`?
/// For ints, that's when `d`'s range contains `c`'s. Ranges are always
/// `split` first, so `c` is wholly inside `d` or wholly outside it.
fn covers(d: Ctor, c: Ctor) -> bool {
    match (d, c) {
        (Ctor::Int(lo, hi), Ctor::Int(a, b)) => lo <= a && b <= hi,
        _ => d == c,
    }
}

/// Cuts `lo..=hi` into consecutive ranges at every boundary of the int ranges
/// in `seen`, so each piece is wholly inside or wholly outside each of them.
fn split(lo: i128, hi: i128, seen: &[Ctor]) -> Vec<Ctor> {
    // Where each piece starts: `lo`, and wherever a seen range starts, or
    // starts having ended. Values are at most `u64::MAX`, so one past one
    // can't overflow.
    let mut starts = vec![lo];
    for c in seen {
        if let Ctor::Int(a, b) = *c {
            starts.extend([a, b + 1]);
        }
    }
    starts.retain(|s| (lo..=hi).contains(s));
    starts.sort_unstable();
    starts.dedup();
    let ends = starts.iter().skip(1).map(|s| s - 1).chain([hi]);
    starts
        .iter()
        .zip(ends)
        .map(|(&a, b)| Ctor::Int(a, b))
        .collect()
}

/// Keeps the rows whose first pattern could match constructor `c`, with that
/// pattern replaced by `c`'s `arity` field patterns (all `_` if it was `_`).
fn specialize(rows: &[Vec<Pat>], c: Ctor, arity: usize) -> Vec<Vec<Pat>> {
    rows.iter()
        .filter_map(|row| {
            let fields = match &row[0] {
                Pat::Ctor(d, args) if covers(*d, c) => args.clone(),
                Pat::Ctor(..) => return None,
                Pat::Wild => vec![Pat::Wild; arity],
                Pat::Or(_) | Pat::Range(..) => {
                    unreachable!("expanded and made plain before specializing")
                }
            };
            Some([fields, row[1..].to_vec()].concat())
        })
        .collect()
}
