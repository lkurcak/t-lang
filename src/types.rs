//! Types as the checker understands them.

use crate::check::source_name;
use crate::traits::{Pred, Projections};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// An integer type: `i8` to `i64`, `u8` to `u64`, `isize` or `usize`
    /// (see docs/11-integers.md).
    Int(IntTy),
    Bool,
    /// A *character*: a Unicode scalar value, a number in 0 to `D7FF` or
    /// `E000` to `10FFFF` (see docs/13-strings.md). Not an integer: it has
    /// no arithmetic, and `as` converts it to and from one.
    Char,
    /// The type with exactly one value, `()`. "Nothing interesting to return."
    Unit,
    /// `!`, the type with no values at all: *never*. It's the type of what
    /// never finishes, like `return x` or a `loop` with no `break`, so a
    /// value of it can be used as any type, since there won't be one (see
    /// docs/05.1-control-flow.md).
    Never,
    /// `fn(A, B) -> R`. The arrow type `A × B → R` of the lambda calculus.
    Fn(FnType),
    /// A struct or enum, by its symbol, with its type arguments: `Shape`,
    /// or `Option<i64>`. Its definition lives in an `Adts` table. The
    /// symbol is its name, unless it's a local type (see `check/names.rs`).
    /// `Display` shows the name, as written.
    ///
    /// Types are compared by *name* (nominal typing): `struct A { x: i64 }`
    /// and `struct B { x: i64 }` are different types even though they look
    /// the same. The alternative, structural typing, would compare what's
    /// inside instead.
    Adt(String, Vec<Region>, Vec<Type>),
    /// A type parameter, like the `T` in `fn id<T>(x: T) -> T`, inside the
    /// function or type that declares it. There it stands for a type the
    /// caller picks, so nothing is known about it: it's equal only to itself
    /// (see docs/04-generics.md).
    Param(String),
    /// A type not known yet: a *type variable*, standing for some type the
    /// checker will work out from how the value is used (see docs/03-inference.md).
    /// The number indexes the checker's table of what each one has turned out
    /// to be. Variables only ever exist while a function body is checked.
    Var(u32),
    /// `&'a T` or `&'a mut T` (the `bool`): a *reference*, borrowing a `T`
    /// that lives somewhere else, for as long as region `'a` says. See
    /// docs/05-ownership.md.
    Ref(Region, bool, Box<Type>),
    /// `Box<T>`: a `T` on the heap, which the box *owns*: it's freed when
    /// the box is.
    Box(Box<Type>),
    /// `[T]`: a *slice*, some number of `T`s one after another in memory,
    /// how many known only as the program runs. So it's never a value of
    /// its own, only what a reference points to, `&'a [T]` or `&'a mut
    /// [T]`, which carries the length (see docs/10-slices.md): nor is it
    /// what a type parameter stands for, so generic code never has one by
    /// value either (`Checker::sized`).
    Slice(Box<Type>),
    /// `str`: a *string*, bytes that are UTF-8, how many known only as the
    /// program runs. Like a slice, it's only ever what a reference points
    /// to, and only a shared one, `&'a str`: changing its bytes could make
    /// them not UTF-8 (see docs/13-strings.md).
    Str,
    /// `<T as Iter>::Item`: a *projection*, the type that the impl of trait
    /// `.1` for type `.0` picks for its associated type `.2` (see
    /// docs/06b-associated-types.md). Once the type's outermost shape is
    /// known, an impl is found, and the projection *normalises* to what it
    /// says (`Projections::normalise`). What's left is *rigid*: on a type
    /// parameter, in generic code, where it's a type that's equal only to
    /// itself, like the parameter; or on a type variable, until it's known.
    Proj(Box<Type>, String, String),
    /// `impl Fn(i64) -> i64`, as a function's return type: an *opaque*
    /// type, standing for one type the function picks, its *hidden* type,
    /// of which the rest of the program knows only what the bounds say
    /// (see docs/06c.1-opaque-types.md). By its symbol, with every lifetime
    /// and type parameter in scope where it's written, which its hidden
    /// type may mention: it's equal only to itself at the same ones. Its
    /// definition is a `traits::OpaqueDef`. Monomorphisation replaces it
    /// with the hidden type.
    Opaque(String, Vec<Region>, Vec<Type>),
    /// `{alloc, e}`: a set of *effects*, what a call may do besides give
    /// its result (see docs/07-effects.md). A type of a kind of its own,
    /// as in Koka: it's only ever a function type's effects, or a type
    /// argument for an effect parameter, never the type of a value. Its
    /// elements are `Alloc`, `Panic`, `Unsafe`, effect parameters (`Param`),
    /// associated effects (`Proj`) and variables; `Type::effects` keeps
    /// them flat, sorted and without repeats, so equal sets are equal
    /// types.
    Effects(Vec<Type>),
    /// `alloc`, in an `Effects`: may allocate memory.
    Alloc,
    /// `panic`, in an `Effects`: may abort the program.
    Panic,
    /// `unsafe`, in an `Effects`: has obligations its caller must keep,
    /// which the checker doesn't check (see docs/09-unsafe.md). Only an
    /// `unsafe` block takes it away.
    Unsafe,
}

/// A *region*, or *lifetime*: the stretch of the program during which a
/// reference may be used. Type checking ignores them; the borrow checker
/// (`borrowck`) works them out.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Region {
    /// A lifetime parameter, like `'a` in `fn first<'a>(..)`, with its `'`.
    /// Elision gives each elided lifetime in a signature a name of its own,
    /// which no program can write: `'_0`, `'_1`, and so on.
    Named(String),
    /// `'static`: the whole run of the program.
    Static,
    /// A lifetime nobody wrote, in a function body. The borrow checker
    /// works out what it must be.
    Erased,
    /// One of the borrow checker's region variables.
    Var(usize),
    /// The `.1`th lifetime of the result of closure `.0`, until it's worked
    /// out from its body (see `check/closures.rs`). No program can write
    /// one.
    Result(crate::ast::NodeId, usize),
}

impl Region {
    /// Was it made up by elision, rather than written?
    pub fn is_elided(&self) -> bool {
        matches!(self, Region::Named(n) if n.starts_with("'_"))
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Region::Named(n) if self.is_elided() => write!(f, "'{}", &n[2..]),
            Region::Named(n) => f.write_str(n),
            Region::Static => f.write_str("'static"),
            Region::Erased => f.write_str("'_"),
            Region::Var(v) => write!(f, "'?{v}"),
            Region::Result(l, i) => write!(f, "'?r{}.{i}", l.0),
        }
    }
}

/// `'a: 'b` or `T: 'b`: what's before the `:` *outlives* the region after
/// it. For a type, that means every region in it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    Region(Region, Region),
    Param(String, Region),
    /// `T::Item: 'b`, of a rigid projection.
    Proj(Type, Region),
}

/// A function type's parts. A struct rather than fields of `Type::Fn`, so
/// that a function type can grow more parts without touching everything
/// that only looks at these: step 7 adds the effects a call may have, like
/// panicking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnType {
    pub params: Vec<Type>,
    pub ret: Box<Type>,
    /// The lifetimes it's generic over, named `'^0`, `'^1` and so on in it:
    /// those left out where it's written. `fn(&i64) -> i64` takes a
    /// reference of *any* lifetime, as `for<'x> fn(&'x i64) -> i64` would
    /// say in Rust: it's *higher-ranked*. Each call picks the lifetimes.
    pub bound: Vec<String>,
    /// What its calls may do: always a `Type::Effects`, empty if they can
    /// do nothing but give their result.
    pub effects: Box<Type>,
}

impl Type {
    /// `fn(params) -> ret`, whose calls do nothing else.
    pub fn func(params: Vec<Type>, ret: Type) -> Type {
        Type::func_with(params, ret, Type::no_effects())
    }

    /// `fn(params) -> ret with effects`.
    pub fn func_with(params: Vec<Type>, ret: Type, effects: Type) -> Type {
        Type::Fn(FnType {
            params,
            ret: Box::new(ret),
            bound: Vec::new(),
            effects: Box::new(effects),
        })
    }

    /// `{}`: no effects.
    pub fn no_effects() -> Type {
        Type::Effects(Vec::new())
    }

    /// The set of `items`, each an effect or a set of them: flat, sorted
    /// and without repeats.
    pub fn effects(items: impl IntoIterator<Item = Type>) -> Type {
        fn flatten(t: Type, out: &mut Vec<Type>) {
            match t {
                Type::Effects(items) => items.into_iter().for_each(|i| flatten(i, out)),
                t => out.push(t),
            }
        }
        let mut out = Vec::new();
        items.into_iter().for_each(|i| flatten(i, &mut out));
        out.sort_by_cached_key(effect_key);
        out.dedup();
        Type::Effects(out)
    }

    /// The elements of an effect set: none if this isn't one.
    pub fn effect_items(&self) -> &[Type] {
        match self {
            Type::Effects(items) => items,
            _ => &[],
        }
    }

    /// Is this an effect set with nothing in it?
    pub fn is_no_effects(&self) -> bool {
        matches!(self, Type::Effects(items) if items.is_empty())
    }
}

/// Where an effect goes in a set: `alloc`, `panic`, `unsafe`, then parameters,
/// associated effects and variables, each in an order of their own.
fn effect_key(t: &Type) -> (u8, String) {
    match t {
        Type::Alloc => (0, String::new()),
        Type::Panic => (1, String::new()),
        Type::Unsafe => (2, String::new()),
        Type::Param(p) => (3, p.clone()),
        Type::Proj(..) => (4, format!("{t:?}")),
        Type::Var(v) => (5, format!("{v:010}")),
        t => (6, format!("{t:?}")),
    }
}

/// `panic`, or `{alloc, panic}`: an effect set as it's written after `with`.
pub fn effects_text(t: &Type) -> String {
    match t.effect_items() {
        [one] => one.to_string(),
        items => format!("{{{}}}", comma_list(items)),
    }
}

impl FnType {
    /// Its parameters and result, with its lifetimes `regions`.
    pub fn instantiate(&self, regions: &[Region]) -> (Vec<Type>, Type) {
        let mut f = |r: &Region| match r {
            Region::Named(n) => match self.bound.iter().position(|b| b == n) {
                Some(i) => regions[i].clone(),
                None => r.clone(),
            },
            r => r.clone(),
        };
        let params = self.params.iter().map(|p| p.map_regions(&mut f)).collect();
        (params, self.ret.map_regions(&mut f))
    }

    /// The same signature, with these effects.
    pub fn with_effects(&self, effects: Type) -> FnType {
        FnType {
            effects: Box::new(effects),
            ..self.clone()
        }
    }
}

/// What a struct or enum is made of.
#[derive(Debug, Clone)]
pub struct AdtDef {
    /// Its type parameters: `["T"]` for `enum Option<T>`. Field types use
    /// them as `Type::Param`. None is an effect parameter.
    pub params: Params,
    /// Its lifetime parameters, with their `'`. Field types use them as
    /// `Region::Named`.
    pub lifetimes: Vec<String>,
    /// The bounds it declares, like `T: 'a`.
    pub bounds: Vec<Bound>,
    /// The traits it needs its type parameters to implement, like
    /// `T: Show`: every use of it must show they do.
    pub preds: Vec<Pred>,
    /// What its equality bounds say, like `(T::Item, i64)`: every use
    /// must show that too.
    pub eqs: Vec<(Type, Type)>,
    pub kind: AdtKind,
}

impl AdtDef {
    /// The type `symbol` names with its own parameters as arguments, as
    /// its fields see it: `Option<T>`, `Holder<'a, T>`.
    pub fn generic(&self, symbol: &str) -> Type {
        Type::Adt(
            symbol.to_string(),
            self.lifetimes.iter().cloned().map(Region::Named).collect(),
            self.params.iter().cloned().map(Type::Param).collect(),
        )
    }
}

#[derive(Debug, Clone)]
pub enum AdtKind {
    /// A *product* type: a value has every field.
    Struct(Vec<(String, Type)>),
    /// A *sum* type: a value is exactly one of the variants, and also holds
    /// that variant's fields.
    Enum(Vec<(String, Vec<Type>)>),
}

/// Every struct and enum in the program, by name, and what the
/// projections their fields may have stand for.
#[derive(Debug, Clone, Default)]
pub struct Adts {
    defs: HashMap<String, AdtDef>,
    pub proj: Projections,
}

impl Adts {
    pub fn new() -> Adts {
        Adts::default()
    }

    /// `t` with every projection in it normalised, as far as it goes (see
    /// `Projections::normalise`).
    pub fn normalise(&self, t: &Type) -> Type {
        self.proj.normalise(t)
    }
}

impl std::ops::Deref for Adts {
    type Target = HashMap<String, AdtDef>;

    fn deref(&self) -> &Self::Target {
        &self.defs
    }
}

impl std::ops::DerefMut for Adts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.defs
    }
}

// Looking inside struct and enum values. Everything that needs a field's type
// goes through these instead of reading an `AdtDef`, because the types an
// `AdtDef` holds mention its type parameters, and these put the type's
// arguments in their place: the field of `Option<i64>::Some` is an `i64`.

/// What `name<regions, args>` is made of, with `regions` and `args`
/// substituted for the parameters.
///
/// A field's type can have a projection on a parameter in it, like `T::Item`,
/// which the arguments may let normalise: with the arguments' regions, each
/// where the impl puts it, so the borrow checker gets the regions the field
/// really has.
fn instantiate(adts: &Adts, name: &str, regions: &[Region], args: &[Type]) -> AdtKind {
    let def = &adts[name];
    let s = Subst::new(&def.params, args).with_regions(&def.lifetimes, regions);
    let field = |t: &Type| adts.normalise(&s.apply(t));
    match &def.kind {
        AdtKind::Struct(fields) => {
            AdtKind::Struct(fields.iter().map(|(f, t)| (f.clone(), field(t))).collect())
        }
        AdtKind::Enum(variants) => AdtKind::Enum(
            variants
                .iter()
                .map(|(v, ts)| (v.clone(), ts.iter().map(field).collect()))
                .collect(),
        ),
    }
}

/// The fields of `ty`, if it's a struct.
pub fn struct_fields(adts: &Adts, ty: &Type) -> Option<Vec<(String, Type)>> {
    match ty {
        Type::Adt(name, regions, args) => match instantiate(adts, name, regions, args) {
            AdtKind::Struct(fields) => Some(fields),
            AdtKind::Enum(_) => None,
        },
        _ => None,
    }
}

/// The fields of variant `i` of `ty`, which must be an enum.
pub fn variant_fields(adts: &Adts, ty: &Type, i: usize) -> Vec<Type> {
    match ty {
        Type::Adt(name, regions, args) => match instantiate(adts, name, regions, args) {
            AdtKind::Enum(mut variants) => variants.swap_remove(i).1,
            AdtKind::Struct(_) => unreachable!("variant of struct `{name}`"),
        },
        _ => unreachable!("variant of `{ty}`"),
    }
}

/// The types of every field of every variant of `ty`. None, unless it's a
/// struct or enum.
pub fn all_fields(adts: &Adts, ty: &Type) -> Vec<Type> {
    match ty {
        Type::Adt(name, regions, args) => match instantiate(adts, name, regions, args) {
            AdtKind::Struct(fields) => fields.into_iter().map(|(_, t)| t).collect(),
            AdtKind::Enum(variants) => variants.into_iter().flat_map(|(_, ts)| ts).collect(),
        },
        _ => Vec::new(),
    }
}

/// A *substitution*: which type each type parameter stands for, and which
/// region each lifetime parameter does. Applying one to `fn(T) -> Option<T>`
/// with `T := i64` gives `fn(i64) -> Option<i64>`.
#[derive(Debug, Clone, Default)]
pub struct Subst {
    types: HashMap<String, Type>,
    regions: HashMap<String, Region>,
}

impl Subst {
    /// Parameter `params[i]` is `args[i]`.
    pub fn new(params: &[String], args: &[Type]) -> Subst {
        assert_eq!(params.len(), args.len(), "wrong number of type arguments");
        Subst {
            types: params.iter().cloned().zip(args.iter().cloned()).collect(),
            regions: HashMap::new(),
        }
    }

    /// This, and lifetime parameter `lifetimes[i]` is `regions[i]`. With no
    /// `regions`, each is `Region::Erased`: not known yet.
    pub fn with_regions(mut self, lifetimes: &[String], regions: &[Region]) -> Subst {
        for (i, l) in lifetimes.iter().enumerate() {
            let r = regions.get(i).cloned().unwrap_or(Region::Erased);
            self.regions.insert(l.clone(), r);
        }
        self
    }

    /// This, and type parameter `name` is `ty`.
    pub fn with_type(mut self, name: &str, ty: Type) -> Subst {
        self.types.insert(name.to_string(), ty);
        self
    }

    /// `t`, with every parameter this substitution knows replaced.
    pub fn apply(&self, t: &Type) -> Type {
        match t {
            Type::Param(p) => self.types.get(p).cloned().unwrap_or_else(|| t.clone()),
            Type::Ref(r, m, inner) => Type::Ref(self.region(r), *m, Box::new(self.apply(inner))),
            Type::Adt(name, regions, args) => Type::Adt(
                name.clone(),
                regions.iter().map(|r| self.region(r)).collect(),
                args.iter().map(|a| self.apply(a)).collect(),
            ),
            Type::Opaque(name, regions, args) => Type::Opaque(
                name.clone(),
                regions.iter().map(|r| self.region(r)).collect(),
                args.iter().map(|a| self.apply(a)).collect(),
            ),
            t => t.map(|p| self.apply(p)),
        }
    }

    /// Region `r`, replaced if it's a lifetime parameter this knows.
    pub fn region(&self, r: &Region) -> Region {
        match r {
            Region::Named(n) => self.regions.get(n).cloned().unwrap_or_else(|| r.clone()),
            r => r.clone(),
        }
    }
}

impl Type {
    /// The types directly inside this one: a function type's parameters and
    /// result, and a struct or enum's type arguments. Code that walks a type
    /// goes through this and `map`, so a new kind of type with parts only
    /// has to be taught to these two (and `same_shape`).
    pub fn parts(&self) -> impl Iterator<Item = &Type> {
        let (params, ret, effects): (&[Type], Option<&Type>, Option<&Type>) = match self {
            Type::Fn(f) => (&f.params, Some(&f.ret), Some(&f.effects)),
            Type::Adt(_, _, args) | Type::Opaque(_, _, args) | Type::Effects(args) => {
                (args, None, None)
            }
            Type::Ref(_, _, t) | Type::Box(t) | Type::Slice(t) | Type::Proj(t, ..) => {
                (&[], Some(t), None)
            }
            Type::Int(_)
            | Type::Bool
            | Type::Char
            | Type::Str
            | Type::Unit
            | Type::Never
            | Type::Param(_)
            | Type::Var(_)
            | Type::Alloc
            | Type::Panic
            | Type::Unsafe => (&[], None, None),
        };
        params.iter().chain(ret).chain(effects)
    }

    /// Is it a slice or `str`: a type whose values' size is known only as
    /// the program runs, so it's only ever behind a reference?
    pub fn is_unsized(&self) -> bool {
        matches!(self, Type::Slice(_) | Type::Str)
    }

    /// A slice or `str` in this type that isn't right behind a reference
    /// (a shared one, for `str`), if there's one: where it would be a value
    /// of its own.
    pub fn unsized_part(&self) -> Option<&Type> {
        match self {
            Type::Slice(_) | Type::Str => Some(self),
            Type::Ref(_, _, inner) if matches!(**inner, Type::Slice(_)) => {
                inner.parts().find_map(Type::unsized_part)
            }
            Type::Ref(_, false, inner) if matches!(**inner, Type::Str) => None,
            t => t.parts().find_map(Type::unsized_part),
        }
    }

    /// This type, with each of its parts replaced by `f(part)`. Its regions
    /// stay as they are.
    pub fn map(&self, mut f: impl FnMut(&Type) -> Type) -> Type {
        match self {
            Type::Fn(sig) => Type::Fn(FnType {
                params: sig.params.iter().map(&mut f).collect(),
                ret: Box::new(f(&sig.ret)),
                bound: sig.bound.clone(),
                effects: Box::new(Type::effects([f(&sig.effects)])),
            }),
            Type::Effects(items) => Type::effects(items.iter().map(f)),
            Type::Adt(name, regions, args) => {
                Type::Adt(name.clone(), regions.clone(), args.iter().map(f).collect())
            }
            Type::Opaque(name, regions, args) => {
                Type::Opaque(name.clone(), regions.clone(), args.iter().map(f).collect())
            }
            Type::Ref(r, m, t) => Type::Ref(r.clone(), *m, Box::new(f(t))),
            Type::Box(t) => Type::Box(Box::new(f(t))),
            Type::Slice(t) => Type::Slice(Box::new(f(t))),
            Type::Proj(t, tr, name) => Type::Proj(Box::new(f(t)), tr.clone(), name.clone()),
            Type::Int(_)
            | Type::Bool
            | Type::Char
            | Type::Str
            | Type::Unit
            | Type::Never
            | Type::Param(_)
            | Type::Var(_)
            | Type::Alloc
            | Type::Panic
            | Type::Unsafe => self.clone(),
        }
    }

    /// Do `self` and `other` have the same outermost shape, so that they're
    /// equal exactly when their `parts` are? `fn(A) -> B` and `fn(C) -> D`
    /// do, `Option<A>` and `Option<B>` do, `i64` and `bool` don't. Regions
    /// don't count: `&'a i64` and `&'b i64` have the same shape, and type
    /// checking treats them as the same type.
    pub fn same_shape(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Fn(f), Type::Fn(g)) => f.params.len() == g.params.len(),
            (Type::Adt(a, _, xs), Type::Adt(b, _, ys))
            | (Type::Opaque(a, _, xs), Type::Opaque(b, _, ys)) => a == b && xs.len() == ys.len(),
            (Type::Ref(_, m, _), Type::Ref(_, n, _)) => m == n,
            (Type::Box(_), Type::Box(_)) | (Type::Slice(_), Type::Slice(_)) => true,
            (Type::Proj(_, a, x), Type::Proj(_, b, y)) => a == b && x == y,
            (Type::Var(_), _) | (_, Type::Var(_)) => false,
            // Sets aren't equal element by element (see `unify`).
            (Type::Effects(_), _) | (_, Type::Effects(_)) => false,
            (a, b) => a == b,
        }
    }

    /// This type with every region in it replaced by `f(region)`, but for
    /// those a function type in it is generic over.
    pub fn map_regions(&self, f: &mut impl FnMut(&Region) -> Region) -> Type {
        self.map_free_regions(f, &mut Vec::new())
    }

    /// `map_regions`, inside function types that are generic over `bound`.
    fn map_free_regions(
        &self,
        f: &mut impl FnMut(&Region) -> Region,
        bound: &mut Vec<String>,
    ) -> Type {
        let free =
            |r: &Region, bound: &[String]| !matches!(r, Region::Named(n) if bound.contains(n));
        match self {
            Type::Ref(r, m, t) => {
                let r = if free(r, bound) { f(r) } else { r.clone() };
                Type::Ref(r, *m, Box::new(t.map_free_regions(f, bound)))
            }
            Type::Adt(name, regions, args) => {
                let regions = regions
                    .iter()
                    .map(|r| if free(r, bound) { f(r) } else { r.clone() })
                    .collect();
                let args = args.iter().map(|a| a.map_free_regions(f, bound)).collect();
                Type::Adt(name.clone(), regions, args)
            }
            Type::Opaque(name, regions, args) => {
                let regions = regions
                    .iter()
                    .map(|r| if free(r, bound) { f(r) } else { r.clone() })
                    .collect();
                let args = args.iter().map(|a| a.map_free_regions(f, bound)).collect();
                Type::Opaque(name.clone(), regions, args)
            }
            Type::Fn(sig) => {
                let outer = bound.len();
                bound.extend(sig.bound.iter().cloned());
                let params = sig
                    .params
                    .iter()
                    .map(|p| p.map_free_regions(f, bound))
                    .collect();
                let ret = sig.ret.map_free_regions(f, bound);
                let effects = sig.effects.map_free_regions(f, bound);
                bound.truncate(outer);
                Type::Fn(FnType {
                    params,
                    ret: Box::new(ret),
                    bound: sig.bound.clone(),
                    effects: Box::new(effects),
                })
            }
            t => t.map(|p| p.map_free_regions(f, bound)),
        }
    }

    /// This type with no regions in it: each is `Region::Erased`.
    pub fn erased(&self) -> Type {
        self.map_regions(&mut |_| Region::Erased)
    }

    /// Every region in this type, in order, at any depth, but for those a
    /// function type in it is generic over.
    pub fn regions(&self) -> Vec<&Region> {
        let mut out = Vec::new();
        self.each_region(&mut |r| out.push(r), &mut Vec::new());
        out
    }

    fn each_region<'t>(&'t self, f: &mut impl FnMut(&'t Region), bound: &mut Vec<&'t String>) {
        let mut free = |r: &'t Region| {
            if !matches!(r, Region::Named(n) if bound.contains(&n)) {
                f(r)
            }
        };
        match self {
            Type::Ref(r, _, _) => free(r),
            Type::Adt(_, regions, _) | Type::Opaque(_, regions, _) => regions.iter().for_each(free),
            _ => {}
        }
        let outer = bound.len();
        if let Type::Fn(sig) = self {
            bound.extend(&sig.bound);
        }
        for p in self.parts() {
            p.each_region(f, bound);
        }
        bound.truncate(outer);
    }

    /// Does this type mention a region, or a type parameter, which may
    /// stand for a type that does? Then a value of it may hold a reference,
    /// or be a function that's only good with references of a certain
    /// lifetime. Not if the regions are those a function type is generic
    /// over: `fn(&i64) -> i64` takes references of any lifetime.
    pub fn may_borrow(&self) -> bool {
        fn has_param(t: &Type) -> bool {
            matches!(t, Type::Param(_) | Type::Proj(..)) || t.parts().any(has_param)
        }
        !self.regions().is_empty() || has_param(self)
    }
}

impl Type {
    /// Like `to_string`, but with the symbols of structs and enums rather
    /// than their names, so that two types of the same name show
    /// differently.
    ///
    /// Regions aren't shown: monomorphisation names copies with this, and
    /// a copy doesn't depend on them.
    pub fn symbols(&self) -> String {
        match self {
            Type::Fn(sig) => {
                let params: Vec<String> = sig.params.iter().map(Type::symbols).collect();
                let effects: Vec<String> = sig
                    .effects
                    .effect_items()
                    .iter()
                    .map(Type::symbols)
                    .collect();
                let effects = match &effects[..] {
                    [] => String::new(),
                    [one] => format!(" with {one}"),
                    all => format!(" with {{{}}}", all.join(", ")),
                };
                format!(
                    "fn({}) -> {}{effects}",
                    params.join(", "),
                    sig.ret.symbols()
                )
            }
            Type::Effects(items) => {
                let items: Vec<String> = items.iter().map(Type::symbols).collect();
                format!("{{{}}}", items.join(", "))
            }
            Type::Adt(name, _, args) if args.is_empty() => name.clone(),
            Type::Adt(name, _, args) => {
                let args: Vec<String> = args.iter().map(Type::symbols).collect();
                format!("{name}<{}>", args.join(", "))
            }
            Type::Ref(_, m, t) => format!("&{}{}", if *m { "mut " } else { "" }, t.symbols()),
            Type::Box(t) => format!("Box<{}>", t.symbols()),
            Type::Slice(t) => format!("[{}]", t.symbols()),
            Type::Proj(t, tr, name) => format!("<{} as {tr}>::{name}", t.symbols()),
            Type::Opaque(name, _, args) => {
                let args: Vec<String> = args.iter().map(Type::symbols).collect();
                format!("{{{name}}}<{}>", args.join(", "))
            }
            t => t.to_string(),
        }
    }
}

/// An integer type: the integers from its least to its greatest. Every
/// operator on one gives the same number as in the integers, ℤ, or panics:
/// none wraps around (see docs/11-integers.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

impl IntTy {
    pub const ALL: [IntTy; 10] = [
        IntTy::I8,
        IntTy::I16,
        IntTy::I32,
        IntTy::I64,
        IntTy::Isize,
        IntTy::U8,
        IntTy::U16,
        IntTy::U32,
        IntTy::U64,
        IntTy::Usize,
    ];

    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::Isize => "isize",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::Usize => "usize",
        }
    }

    /// The integer type called `name`, if there's one.
    pub fn named(name: &str) -> Option<IntTy> {
        IntTy::ALL.into_iter().find(|t| t.name() == name)
    }

    pub fn signed(self) -> bool {
        self < IntTy::U8
    }

    /// Its least and greatest values, unless it's `isize` or `usize`, whose
    /// size depends on the machine.
    pub fn range(self) -> Option<(i128, i128)> {
        let bits = match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            IntTy::I64 | IntTy::U64 => 64,
            IntTy::Isize | IntTy::Usize => return None,
        };
        Some(match self.signed() {
            true => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
            false => (0, (1i128 << bits) - 1),
        })
    }

    /// Its range on a 64-bit machine, which the interpreter is.
    pub fn range_64(self) -> (i128, i128) {
        match self {
            IntTy::Isize => IntTy::I64.range_64(),
            IntTy::Usize => IntTy::U64.range_64(),
            t => t.range().expect("a fixed size"),
        }
    }

    /// "a `u8`", "an `i8`".
    pub fn a(self) -> String {
        match self.signed() {
            true => format!("an `{}`", self.name()),
            false => format!("a `{}`", self.name()),
        }
    }
}

/// Is `name` a built-in type's: an integer type's, `bool`, `char`, `str`
/// or `Box`?
pub fn is_builtin_type(name: &str) -> bool {
    IntTy::named(name).is_some() || matches!(name, "bool" | "char" | "str" | "Box")
}

/// `a, b, c`
pub fn comma_list<T: fmt::Display>(items: &[T]) -> String {
    let items: Vec<String> = items.iter().map(T::to_string).collect();
    items.join(", ")
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int(t) => f.write_str(t.name()),
            Type::Bool => f.write_str("bool"),
            Type::Char => f.write_str("char"),
            Type::Str => f.write_str("str"),
            Type::Unit => f.write_str("()"),
            Type::Never => f.write_str("!"),
            // Effects not worked out yet aren't shown: what's known is.
            Type::Fn(sig) => {
                write!(f, "fn({}) -> {}", comma_list(&sig.params), sig.ret)?;
                let known = Type::Effects(
                    (sig.effects.effect_items().iter())
                        .filter(|e| !matches!(e, Type::Var(_)))
                        .cloned()
                        .collect(),
                );
                match known.is_no_effects() {
                    true => Ok(()),
                    false => write!(f, " with {}", effects_text(&known)),
                }
            }
            Type::Effects(items) => write!(f, "{{{}}}", comma_list(items)),
            Type::Alloc => f.write_str("alloc"),
            Type::Panic => f.write_str("panic"),
            Type::Unsafe => f.write_str("unsafe"),
            // Its type parameters are those in scope where it's written,
            // which say nothing to a reader.
            Type::Adt(name, ..) if is_closure(name) => f.write_str("{closure}"),
            Type::Adt(name, regions, args) => {
                f.write_str(&source_name(name))?;
                // Lifetimes only if some were written.
                let mut all: Vec<String> = match regions.iter().all(hidden) {
                    true => Vec::new(),
                    false => regions.iter().map(Region::to_string).collect(),
                };
                all.extend(args.iter().map(Type::to_string));
                match all.is_empty() {
                    true => Ok(()),
                    false => write!(f, "<{}>", all.join(", ")),
                }
            }
            Type::Ref(r, m, t) => {
                f.write_str("&")?;
                if !hidden(r) {
                    write!(f, "{r} ")?;
                }
                if *m {
                    f.write_str("mut ")?;
                }
                write!(f, "{t}")
            }
            Type::Box(t) => write!(f, "Box<{t}>"),
            Type::Slice(t) => write!(f, "[{t}]"),
            // `T::Item` is what's written, and says enough in a message.
            Type::Proj(t, _, name) if matches!(**t, Type::Param(_)) => write!(f, "{t}::{name}"),
            Type::Proj(t, tr, name) => write!(f, "<{t} as {tr}>::{name}"),
            Type::Param(name) => f.write_str(name),
            // As written, which is where it's from: what it's at says little.
            Type::Opaque(name, ..) => f.write_str(opaque_text(name)),
            // Like Rust: "some type, not known yet".
            Type::Var(_) => f.write_str("_"),
        }
    }
}

/// The type and effect parameters of something generic, in order, as
/// type arguments for them are. Which kind each is was declared, with
/// `effect e` for an effect parameter, so it's known by the declaration,
/// never by how the name is spelled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params {
    names: Vec<String>,
    /// Which of `names` are effect parameters.
    effects: Vec<String>,
}

impl Params {
    /// Type parameters `names`, of which `effects` are effect parameters.
    pub fn new(names: Vec<String>, effects: Vec<String>) -> Params {
        debug_assert!(effects.iter().all(|e| names.contains(e)));
        Params { names, effects }
    }

    /// Only type parameters.
    pub fn types(names: Vec<String>) -> Params {
        Params::new(names, Vec::new())
    }

    /// Is `name` one of them, and an effect parameter?
    pub fn is_effect(&self, name: &str) -> bool {
        self.effects.iter().any(|e| e == name)
    }

    /// Is `name` one of them, and a type parameter?
    pub fn is_type(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name) && !self.is_effect(name)
    }

    /// These, then `more`.
    pub fn concat(&self, more: &Params) -> Params {
        let mut p = self.clone();
        p.names.extend(more.names.iter().cloned());
        p.effects.extend(more.effects.iter().cloned());
        p
    }

    /// Adds a type parameter at the end.
    pub fn push_type(&mut self, name: String) {
        self.names.push(name);
    }

    /// Adds a type parameter at the front, as `Self` is for a trait's
    /// methods.
    pub fn insert_type(&mut self, name: String) {
        self.names.insert(0, name);
    }

    pub fn as_slice(&self) -> &[String] {
        &self.names
    }

    /// All but `name`.
    pub fn without(&self, name: &str) -> Params {
        let keep = |n: &&String| *n != name;
        Params {
            names: self.names.iter().filter(keep).cloned().collect(),
            effects: self.effects.iter().filter(keep).cloned().collect(),
        }
    }

    /// Those from the `n`th on.
    pub fn skip(&self, n: usize) -> Params {
        let names = self.names[n..].to_vec();
        let effects = self
            .effects
            .iter()
            .filter(|e| names.contains(e))
            .cloned()
            .collect();
        Params { names, effects }
    }

    /// Parameter `name` as a type argument: `Type::Param`, or, for an
    /// effect parameter, the set of just it, as effect arguments are always
    /// sets.
    pub fn arg(&self, name: &str) -> Type {
        match self.is_effect(name) {
            true => Type::Effects(vec![Type::Param(name.to_string())]),
            false => Type::Param(name.to_string()),
        }
    }

    /// Each as a type argument, which makes something generic over them
    /// what it is.
    pub fn args(&self) -> Vec<Type> {
        self.names.iter().map(|p| self.arg(p)).collect()
    }
}

impl std::ops::Deref for Params {
    type Target = [String];

    fn deref(&self) -> &[String] {
        &self.names
    }
}

/// Is `symbol` a closure's type's? See `traits::ClosureDef`.
pub fn is_closure(symbol: &str) -> bool {
    symbol.starts_with("{closure#")
}

/// The symbol of the opaque type written as `text` at offset `at`: the
/// text, which is how it's shown, and where, which tells apart two written
/// the same.
pub fn opaque_symbol(text: &str, at: usize) -> String {
    format!("{text}#{at}")
}

/// How the opaque type of `symbol` is shown: as written.
pub fn opaque_text(symbol: &str) -> &str {
    symbol.rsplit_once('#').map_or(symbol, |(text, _)| text)
}

/// Is region `r` one that types are shown without? Those that nobody wrote.
fn hidden(r: &Region) -> bool {
    matches!(r, Region::Erased | Region::Var(_))
        || r.is_elided()
        || matches!(r, Region::Named(n) if n.starts_with("'^"))
}

/// The type of a function, which may be generic: `∀T. fn(T) -> T`
/// is `Scheme { params: ["T"], ty: fn(T) -> T }`. Each use of the function
/// *instantiates* it, picking a type for each parameter.
///
/// Only functions have schemes, and they're the only place a `∀` can be:
/// at the very front, around a whole function type. Everything else has a
/// plain `Type`, with no `∀` inside. That's *rank-1* (or *prenex*)
/// polymorphism (see docs/04-generics.md).
///
/// It's generic over lifetimes too: `fn first<'a>(x: &'a P) -> &'a i64`
/// has `lifetimes: ["'a"]`, as does `fn first(x: &P) -> &i64`, where
/// elision named it `'_0`. Type checking ignores those; the borrow checker
/// instantiates them at each use.
#[derive(Debug, Clone)]
pub struct Scheme {
    pub params: Params,
    pub lifetimes: Vec<String>,
    pub bounds: Vec<Bound>,
    /// The traits it needs its type parameters to implement: `T: Show` in
    /// `fn f<T: Show>`. Every use has to show they do, for the type
    /// arguments it picks.
    pub preds: Vec<Pred>,
    /// What its equality bounds say projections are: `(T::Item, i64)` for
    /// `fn f<T: Iter<Item = i64>>`. Every use has to show they are.
    pub eqs: Vec<(Type, Type)>,
    pub ty: Type,
}

impl Scheme {
    /// A scheme with no parameters.
    pub fn mono(ty: Type) -> Scheme {
        Scheme {
            params: Params::default(),
            lifetimes: Vec::new(),
            bounds: Vec::new(),
            preds: Vec::new(),
            eqs: Vec::new(),
            ty,
        }
    }
}
