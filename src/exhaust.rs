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
//! - A match is **exhaustive** if `_` isn't useful after all the arms. If it
//!   is, the value it finds is the counterexample in the error message.
//!
//! Patterns are compared by *constructor*: the thing a value was built with,
//! like `true`, `Shape::Rect` or `Point`. A type's constructors are finite for
//! bool, `()`, structs and enums, but not for int (practically speaking), and
//! functions have none we can look inside.

use crate::types::{AdtDef, Adts, Type};

/// A pattern reduced to what matters for coverage. Bindings become wildcards
/// (`x` matches anything, just like `_`) and struct fields are put in
/// declaration order, with omitted ones filled in as wildcards.
#[derive(Debug, Clone)]
pub enum Pat {
    Wild,
    Ctor(Ctor, Vec<Pat>),
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

/// The index of the first arm that can never match because earlier arms
/// already cover everything it would.
pub fn unreachable_arm(adts: &Adts, ty: &Type, arms: &[Pat]) -> Option<usize> {
    let cx = Cx { adts };
    let tys = [ty.clone()];
    (0..arms.len()).find(|&i| {
        let rows: Vec<Vec<Pat>> = arms[..i].iter().map(|p| vec![p.clone()]).collect();
        cx.useful(&rows, &[arms[i].clone()], &tys).is_none()
    })
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
        match &q[0] {
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
                    })
                    .collect();
                let all = self.all_ctors(ty);
                // Constructors no row mentions. Ones that can't hold a value,
                // like `Maybe::Impossible(Never)`, don't count: no value is
                // built with them, so none can slip past the rows that way.
                let unseen: Option<Vec<Ctor>> = all.as_ref().map(|all| {
                    let unseen = all.iter().filter(|c| !seen.contains(c));
                    unseen.filter(|c| self.inhabited_ctor(**c, ty)).copied().collect()
                });
                match (all, unseen) {
                    // Note that an enum with no variants always lands here,
                    // and then there's nothing to try: `_` is useless because
                    // there are no values at all.
                    (Some(all), Some(unseen)) if unseen.is_empty() => all.into_iter().find_map(|c| {
                        let wilds = vec![Pat::Wild; self.fields(c, ty).len()];
                        let q: Vec<Pat> = [vec![Pat::Ctor(c, wilds)], q[1..].to_vec()].concat();
                        self.useful(rows, &q, tys)
                    }),
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
        let Pat::Ctor(c, args) = p else { return "_".to_string() };
        let field_tys = self.fields(*c, ty);
        let args: Vec<String> = args.iter().zip(&field_tys).map(|(p, t)| self.show(p, t)).collect();
        match (c, ty) {
            (Ctor::Bool(b), _) => b.to_string(),
            (Ctor::Unit, _) => "()".to_string(),
            (Ctor::Int(n), _) => n.to_string(),
            (Ctor::Struct, Type::Adt(name)) => {
                let AdtDef::Struct(fields) = &self.adts[name] else { unreachable!() };
                if fields.is_empty() {
                    return format!("{name} {{}}");
                }
                let fields: Vec<String> =
                    fields.iter().zip(&args).map(|((f, _), a)| format!("{f}: {a}")).collect();
                format!("{name} {{ {} }}", fields.join(", "))
            }
            (Ctor::Variant(i), Type::Adt(name)) => {
                let AdtDef::Enum(variants) = &self.adts[name] else { unreachable!() };
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

/// Keeps the rows whose first pattern could match constructor `c`, with that
/// pattern replaced by `c`'s `arity` field patterns (all `_` if it was `_`).
fn specialize(rows: &[Vec<Pat>], c: Ctor, arity: usize) -> Vec<Vec<Pat>> {
    rows.iter()
        .filter_map(|row| {
            let fields = match &row[0] {
                Pat::Ctor(d, args) if *d == c => args.clone(),
                Pat::Ctor(..) => return None,
                Pat::Wild => vec![Pat::Wild; arity],
            };
            Some([fields, row[1..].to_vec()].concat())
        })
        .collect()
}
