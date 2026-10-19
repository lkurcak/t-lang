//! Types as the checker understands them.

use crate::check::source_name;
use crate::traits::{Pred, Projections};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Bool,
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
    /// or `Option<int>`. Its definition lives in an `Adts` table. The
    /// symbol is its name, unless it's a local type (see `check/names.rs`).
    /// `Display` shows the name, as written.
    ///
    /// Types are compared by *name* (nominal typing): `struct A { x: int }`
    /// and `struct B { x: int }` are different types even though they look
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
    /// `<T as Iter>::Item`: a *projection*, the type that the impl of trait
    /// `.1` for type `.0` picks for its associated type `.2` (see
    /// docs/06b-associated-types.md). Once the type's outermost shape is
    /// known, an impl is found, and the projection *normalises* to what it
    /// says (`Projections::normalise`). What's left is *rigid*: on a type
    /// parameter, in generic code, where it's a type that's equal only to
    /// itself, like the parameter; or on a type variable, until it's known.
    Proj(Box<Type>, String, String),
    /// `impl Fn(int) -> int`, as a function's return type: an *opaque*
    /// type, standing for one type the function picks, its *hidden* type,
    /// of which the rest of the program knows only what the bounds say
    /// (see docs/06c.1-opaque-types.md). By its symbol, with every lifetime
    /// and type parameter in scope where it's written, which its hidden
    /// type may mention: it's equal only to itself at the same ones. Its
    /// definition is a `traits::OpaqueDef`. Monomorphisation replaces it
    /// with the hidden type.
    Opaque(String, Vec<Region>, Vec<Type>),
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
    /// those left out where it's written. `fn(&int) -> int` takes a
    /// reference of *any* lifetime, as `for<'x> fn(&'x int) -> int` would
    /// say in Rust: it's *higher-ranked*. Each call picks the lifetimes.
    pub bound: Vec<String>,
}

impl Type {
    /// `fn(params) -> ret`.
    pub fn func(params: Vec<Type>, ret: Type) -> Type {
        Type::Fn(FnType {
            params,
            ret: Box::new(ret),
            bound: Vec::new(),
        })
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
}

/// What a struct or enum is made of.
#[derive(Debug, Clone)]
pub struct AdtDef {
    /// Its type parameters: `["T"]` for `enum Option<T>`. Field types use
    /// them as `Type::Param`.
    pub params: Vec<String>,
    /// Its lifetime parameters, with their `'`. Field types use them as
    /// `Region::Named`.
    pub lifetimes: Vec<String>,
    /// The bounds it declares, like `T: 'a`.
    pub bounds: Vec<Bound>,
    /// The traits it needs its type parameters to implement, like
    /// `T: Show`: every use of it must show they do.
    pub preds: Vec<Pred>,
    /// What its equality bounds say, like `(T::Item, int)`: every use
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
// arguments in their place: the field of `Option<int>::Some` is an `int`.

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
/// with `T := int` gives `fn(int) -> Option<int>`.
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
        let (params, ret): (&[Type], Option<&Type>) = match self {
            Type::Fn(f) => (&f.params, Some(&f.ret)),
            Type::Adt(_, _, args) | Type::Opaque(_, _, args) => (args, None),
            Type::Ref(_, _, t) | Type::Box(t) | Type::Proj(t, ..) => (&[], Some(t)),
            Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Param(_) | Type::Var(_) => {
                (&[], None)
            }
        };
        params.iter().chain(ret)
    }

    /// This type, with each of its parts replaced by `f(part)`. Its regions
    /// stay as they are.
    pub fn map(&self, mut f: impl FnMut(&Type) -> Type) -> Type {
        match self {
            Type::Fn(sig) => Type::Fn(FnType {
                params: sig.params.iter().map(&mut f).collect(),
                ret: Box::new(f(&sig.ret)),
                bound: sig.bound.clone(),
            }),
            Type::Adt(name, regions, args) => {
                Type::Adt(name.clone(), regions.clone(), args.iter().map(f).collect())
            }
            Type::Opaque(name, regions, args) => {
                Type::Opaque(name.clone(), regions.clone(), args.iter().map(f).collect())
            }
            Type::Ref(r, m, t) => Type::Ref(r.clone(), *m, Box::new(f(t))),
            Type::Box(t) => Type::Box(Box::new(f(t))),
            Type::Proj(t, tr, name) => Type::Proj(Box::new(f(t)), tr.clone(), name.clone()),
            Type::Int | Type::Bool | Type::Unit | Type::Never | Type::Param(_) | Type::Var(_) => {
                self.clone()
            }
        }
    }

    /// Do `self` and `other` have the same outermost shape, so that they're
    /// equal exactly when their `parts` are? `fn(A) -> B` and `fn(C) -> D`
    /// do, `Option<A>` and `Option<B>` do, `int` and `bool` don't. Regions
    /// don't count: `&'a int` and `&'b int` have the same shape, and type
    /// checking treats them as the same type.
    pub fn same_shape(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Fn(f), Type::Fn(g)) => f.params.len() == g.params.len(),
            (Type::Adt(a, _, xs), Type::Adt(b, _, ys))
            | (Type::Opaque(a, _, xs), Type::Opaque(b, _, ys)) => a == b && xs.len() == ys.len(),
            (Type::Ref(_, m, _), Type::Ref(_, n, _)) => m == n,
            (Type::Box(_), Type::Box(_)) => true,
            (Type::Proj(_, a, x), Type::Proj(_, b, y)) => a == b && x == y,
            (Type::Var(_), _) | (_, Type::Var(_)) => false,
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
                bound.truncate(outer);
                Type::Fn(FnType {
                    params,
                    ret: Box::new(ret),
                    bound: sig.bound.clone(),
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
    /// over: `fn(&int) -> int` takes references of any lifetime.
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
                format!("fn({}) -> {}", params.join(", "), sig.ret.symbols())
            }
            Type::Adt(name, _, args) if args.is_empty() => name.clone(),
            Type::Adt(name, _, args) => {
                let args: Vec<String> = args.iter().map(Type::symbols).collect();
                format!("{name}<{}>", args.join(", "))
            }
            Type::Ref(_, m, t) => format!("&{}{}", if *m { "mut " } else { "" }, t.symbols()),
            Type::Box(t) => format!("Box<{}>", t.symbols()),
            Type::Proj(t, tr, name) => format!("<{} as {tr}>::{name}", t.symbols()),
            Type::Opaque(name, _, args) => {
                let args: Vec<String> = args.iter().map(Type::symbols).collect();
                format!("{{{name}}}<{}>", args.join(", "))
            }
            t => t.to_string(),
        }
    }
}

/// `a, b, c`
pub fn comma_list<T: fmt::Display>(items: &[T]) -> String {
    let items: Vec<String> = items.iter().map(T::to_string).collect();
    items.join(", ")
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => f.write_str("int"),
            Type::Bool => f.write_str("bool"),
            Type::Unit => f.write_str("()"),
            Type::Never => f.write_str("!"),
            Type::Fn(sig) => write!(f, "fn({}) -> {}", comma_list(&sig.params), sig.ret),
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
/// It's generic over lifetimes too: `fn first<'a>(x: &'a P) -> &'a int`
/// has `lifetimes: ["'a"]`, as does `fn first(x: &P) -> &int`, where
/// elision named it `'_0`. Type checking ignores those; the borrow checker
/// instantiates them at each use.
#[derive(Debug, Clone)]
pub struct Scheme {
    pub params: Vec<String>,
    pub lifetimes: Vec<String>,
    pub bounds: Vec<Bound>,
    /// The traits it needs its type parameters to implement: `T: Show` in
    /// `fn f<T: Show>`. Every use has to show they do, for the type
    /// arguments it picks.
    pub preds: Vec<Pred>,
    /// What its equality bounds say projections are: `(T::Item, int)` for
    /// `fn f<T: Iter<Item = int>>`. Every use has to show they are.
    pub eqs: Vec<(Type, Type)>,
    pub ty: Type,
}

impl Scheme {
    /// A scheme with no parameters.
    pub fn mono(ty: Type) -> Scheme {
        Scheme {
            params: Vec::new(),
            lifetimes: Vec::new(),
            bounds: Vec::new(),
            preds: Vec::new(),
            eqs: Vec::new(),
            ty,
        }
    }
}
