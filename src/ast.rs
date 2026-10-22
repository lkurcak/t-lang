//! The abstract syntax tree: what the parser produces and the checker reads.
//!
//! Note that `TypeExpr` (a type as *written* in the source) is separate from
//! `types::Type` (a type as the checker *understands* it). The checker turns
//! one into the other and rejects names that don't mean anything.

use crate::error::Span;
use crate::types::comma_list;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Program {
    /// Every module: the crate root first, which is the whole file, then
    /// each `mod` in the order its name is written. Every item says which
    /// one it's in by its index here (see docs/06d-modules.md).
    pub mods: Vec<ModDecl>,
    /// The root module of the core library, by its index in `mods`, once
    /// it's loaded (see `load.rs`). It's a package of its own: a module
    /// tree beside the crate root's, whose items' symbols start `core::`.
    pub core: Option<usize>,
    /// The root module of the alloc library, the same way, loaded after
    /// the core library's: its items' symbols start `alloc::`.
    pub alloc: Option<usize>,
    /// The `use`s of every module, in order.
    pub uses: Vec<UseDecl>,
    pub types: Vec<TypeDecl>,
    pub traits: Vec<TraitDecl>,
    pub impls: Vec<ImplDecl>,
    pub fns: Vec<FnDecl>,
    /// Doc comments that aren't before anything they can document, in
    /// order. The checker warns about each.
    pub stray_docs: Vec<DocComment>,
    /// The id the next node made gets (see `NodeId`).
    pub next_id: u32,
    /// The operators the checker proved can't panic, by node (see
    /// `check/refine.rs`): empty until it has, and kept by `lift` and
    /// `mono` for the interpreter, which checks it.
    pub proved: std::collections::HashSet<NodeId>,
}

impl Program {
    /// An id no node of the program has yet, for a node made after parsing.
    pub fn fresh_id(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id - 1)
    }

    /// Is module `m` the root of a library built into the compiler, a
    /// package of its own rather than a module of the crate root's?
    pub fn is_library_root(&self, m: usize) -> bool {
        self.core == Some(m) || self.alloc == Some(m)
    }
}

/// Which node of the AST an `Expr`, `Pattern`, `Ident`, `TypeExpr` or
/// `Lambda` is, unique in the program: the parser numbers each as it makes
/// it, and whatever makes one later takes a fresh one
/// (`Program::fresh_id`). What the checker learns about a node is by its
/// id, not its span, which a node made up by desugaring may share with
/// others, nor its address, which only lasts as long as the AST it's in.
/// `lift` and monomorphisation copy ids: an id names a node as written, so
/// each copy of a generic function's body for a type has the facts of the
/// one checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

/// A `///` doc comment, of one or more lines in a row, before the `fn`,
/// `struct`, `enum`, field or variant it documents.
#[derive(Debug, Clone)]
pub struct DocComment {
    pub lines: Vec<DocLine>,
}

#[derive(Debug, Clone)]
pub struct DocLine {
    /// What's after the `///`, as written: `/// Adds one.` is `" Adds one."`.
    pub text: String,
    /// The whole line, `///` and all.
    pub span: Span,
}

impl DocComment {
    /// From the start of its first line to the end of its last.
    pub fn span(&self) -> Span {
        let (first, last) = (&self.lines[0], &self.lines[self.lines.len() - 1]);
        first.span.to(last.span)
    }

    /// The text as Markdown: the lines, without the indentation they all
    /// share, as rustdoc does.
    pub fn markdown(&self) -> String {
        let indent = self
            .lines
            .iter()
            .map(|l| &l.text)
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let lines: Vec<&str> = self
            .lines
            .iter()
            .map(|l| l.text.get(indent..).unwrap_or(""))
            .collect();
        lines.join("\n")
    }
}

/// `#[allow(dead_code, unused_variables)]` before a `fn`, `struct`, `enum`,
/// field or variant: no warnings of those lints in it. The only attribute
/// there is. Which names are lints isn't checked until `allow::apply`.
#[derive(Debug, Clone)]
pub struct Allow {
    /// From the `#` to the `]`.
    pub span: Span,
    pub lints: Vec<Ident>,
}

/// The crate root's index in `Program::mods`.
pub const ROOT: usize = 0;

/// `mod geometry { .. }`: a module, a namespace of its own for the items in
/// it. Those items are in the flat lists of `Program`, each saying which
/// module it's in, as is each `mod` in it.
#[derive(Debug, Clone)]
pub struct ModDecl {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the `pub` or `mod` to the `}` or `;`; for the crate root, the
    /// whole file.
    pub span: Span,
    /// `crate` for the crate root.
    pub name: Ident,
    /// The module it's in: `None` only for the crate root.
    pub parent: Option<usize>,
    /// Where the `pub` is, if it's public.
    pub public: Option<Span>,
    /// Is it `mod name;`, in a file of its own? Its items are in that file
    /// (see `load.rs`).
    pub in_file: bool,
}

/// `use a::b;`, `pub use a::{b, c as d};`: names from other modules,
/// usable in this one as if declared in it.
#[derive(Debug, Clone)]
pub struct UseDecl {
    /// From the `pub` or `use` to the `;`.
    pub span: Span,
    /// The module it's in.
    pub module: usize,
    pub public: Option<Span>,
    pub tree: UseTree,
}

/// What a `use` names: `a::b`, `a::b as c`, or `a::{b, c::d}`, a path
/// before a group of more.
#[derive(Debug, Clone)]
pub struct UseTree {
    /// The segments as written: `crate`, `super` and `self` among them.
    pub path: Vec<Ident>,
    pub kind: UseKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum UseKind {
    /// The path names one item, the last segment, maybe with `as` and
    /// another name for it here.
    Name(Option<Ident>),
    /// `{b, c as d}` after the path and its `::`.
    Group(Vec<UseTree>),
}

impl UseTree {
    /// Each name it brings in: the path to it, and the name it's given.
    pub fn leaves(&self) -> Vec<(Vec<&Ident>, &Ident)> {
        let mut out = Vec::new();
        self.collect(&mut Vec::new(), &mut out);
        out
    }

    fn collect<'a>(
        &'a self,
        prefix: &mut Vec<&'a Ident>,
        out: &mut Vec<(Vec<&'a Ident>, &'a Ident)>,
    ) {
        let len = prefix.len();
        prefix.extend(&self.path);
        match &self.kind {
            UseKind::Name(alias) => {
                if let Some(last) = prefix.last().copied() {
                    out.push((prefix.clone(), alias.as_ref().unwrap_or(last)));
                }
            }
            UseKind::Group(trees) => trees.iter().for_each(|t| t.collect(prefix, out)),
        }
        prefix.truncate(len);
    }
}

/// `struct Point { x: i64, y: i64 }` or `enum Shape { Circle(i64), Empty }`.
#[derive(Debug, Clone)]
pub enum TypeDecl {
    Struct(StructDecl),
    Enum(EnumDecl),
}

impl TypeDecl {
    pub fn name(&self) -> &Ident {
        match self {
            TypeDecl::Struct(s) => &s.name,
            TypeDecl::Enum(e) => &e.name,
        }
    }

    pub fn doc(&self) -> Option<&DocComment> {
        match self {
            TypeDecl::Struct(s) => s.doc.as_ref(),
            TypeDecl::Enum(e) => e.doc.as_ref(),
        }
    }

    pub fn allows(&self) -> &[Allow] {
        match self {
            TypeDecl::Struct(s) => &s.allows,
            TypeDecl::Enum(e) => &e.allows,
        }
    }

    /// The module it's in.
    pub fn module(&self) -> usize {
        match self {
            TypeDecl::Struct(s) => s.module,
            TypeDecl::Enum(e) => e.module,
        }
    }

    /// It, moved to the crate root.
    pub fn at_root(self) -> TypeDecl {
        match self {
            TypeDecl::Struct(s) => TypeDecl::Struct(StructDecl { module: ROOT, ..s }),
            TypeDecl::Enum(e) => TypeDecl::Enum(EnumDecl { module: ROOT, ..e }),
        }
    }

    /// Where its `pub` is, if it's public.
    pub fn public(&self) -> Option<Span> {
        match self {
            TypeDecl::Struct(s) => s.public,
            TypeDecl::Enum(e) => e.public,
        }
    }

    /// From the `struct` or `enum` to the `}`.
    pub fn span(&self) -> Span {
        match self {
            TypeDecl::Struct(s) => s.span,
            TypeDecl::Enum(e) => e.span,
        }
    }

    pub fn generics(&self) -> &Generics {
        match self {
            TypeDecl::Struct(s) => &s.generics,
            TypeDecl::Enum(e) => &e.generics,
        }
    }

    pub fn type_params(&self) -> &[Ident] {
        &self.generics().types
    }

    pub fn derives(&self) -> &[Derive] {
        match self {
            TypeDecl::Struct(s) => &s.derives,
            TypeDecl::Enum(e) => &e.derives,
        }
    }

    /// The `#[derive(..)]` of trait `name` on it, if it has one.
    pub fn derive_of(&self, name: &str) -> Option<&Ident> {
        self.derives()
            .iter()
            .flat_map(|d| &d.names)
            .find(|n| n.name == name)
    }

    /// What the `#[derive(..)]` of trait `name` says it needs, if it says:
    /// `where T: Discard` after it.
    pub fn derive_where(&self, name: &str) -> Option<&Where> {
        self.derives().iter().find_map(|d| {
            let i = d.names.iter().position(|n| n.name == name)?;
            d.wheres[i].as_ref()
        })
    }
}

/// `<'a, 'b: 'a, T: Show + 'a>` after the name of a function, struct or
/// enum, or after `impl`: lifetime parameters first, then type parameters,
/// each maybe with lifetimes it must outlive and, for a type parameter,
/// traits it must implement.
#[derive(Debug, Clone, Default)]
pub struct Generics {
    /// With their `'`: `'a`.
    pub lifetimes: Vec<Ident>,
    /// Type parameters, and effect parameters among them, told apart by
    /// case: the `e` of `fn apply<e, F: Fn() with e>(f: F) with e` is
    /// lowercase (see `parser::is_effect_name`). An effect parameter is a
    /// type parameter of a kind of its own, standing for a set of effects
    /// (`Type::Effects`), so it's instantiated, substituted and
    /// monomorphised as one is.
    pub types: Vec<Ident>,
    /// `'b: 'a` and `T: Show + 'a`, in the order written, each with what's
    /// before its `:` and the lifetimes and traits after it. Lifetimes start
    /// with a `'`, and traits don't.
    pub bounds: Vec<(Ident, Vec<BoundItem>)>,
    /// From the `<` to the `>`, if they're there.
    pub span: Option<Span>,
    /// A function's or impl's `where` clause: `where Self: Discard, I::Item: Eq`,
    /// each bound with the type before its `:`, which can be more than a
    /// type parameter's name.
    pub wheres: Vec<(TypeExpr, Vec<BoundItem>)>,
    /// From the `where` to the end of its last bound, if it's there.
    pub where_span: Option<Span>,
}

impl Generics {
    pub fn is_empty(&self) -> bool {
        self.lifetimes.is_empty() && self.types.is_empty()
    }

    /// The bounds on `name`, a lifetime or type parameter: lifetimes and
    /// traits.
    pub fn bounds_of(&self, name: &str) -> Vec<&BoundItem> {
        self.bounds
            .iter()
            .filter(|(p, _)| p.name == name)
            .flat_map(|(_, b)| b)
            .collect()
    }
}

/// What's after a `:` in a bound: a lifetime, or a trait, maybe saying
/// something of its associated types, as in `Iter<Item = i64>` or
/// `Iter<Item: Discard>`.
#[derive(Debug, Clone)]
pub struct BoundItem {
    pub name: Ident,
    pub assoc: Vec<AssocBound>,
    /// `(i64, bool) -> i64` after `Fn`, `FnMut` or `FnOnce`: the
    /// parameters and result of a call, as in a function type.
    pub sig: Option<(Vec<TypeExpr>, Option<Box<TypeExpr>>)>,
    /// `with e` after an `Fn` bound's signature: what its calls may do, at
    /// most. With `sig` but without this, they can do nothing.
    pub effects: Option<EffectSet>,
    /// From the name to the `>`, or just the name.
    pub span: Span,
}

impl BoundItem {
    /// The types written in it: what its equality bounds say, what an
    /// `Fn` bound's calls take and give, and the types whose associated
    /// effects its effects are.
    pub fn types(&self) -> Vec<&TypeExpr> {
        let eqs = self.assoc.iter().filter_map(|a| match &a.kind {
            AssocBoundKind::Eq(t) => Some(t),
            AssocBoundKind::Bounds(_) => None,
        });
        let sig = self
            .sig
            .iter()
            .flat_map(|(ps, ret)| ps.iter().chain(ret.as_deref()));
        let effects = self.effects.iter().flat_map(EffectSet::types);
        eqs.chain(sig).chain(effects).collect()
    }

    /// `types`, to change.
    pub fn types_mut(&mut self) -> Vec<&mut TypeExpr> {
        let eqs = self.assoc.iter_mut().filter_map(|a| match &mut a.kind {
            AssocBoundKind::Eq(t) => Some(t),
            AssocBoundKind::Bounds(_) => None,
        });
        let sig = self
            .sig
            .iter_mut()
            .flat_map(|(ps, ret)| ps.iter_mut().chain(ret.as_deref_mut()));
        let effects = self.effects.iter_mut().flat_map(EffectSet::types_mut);
        eqs.chain(sig).chain(effects).collect()
    }

    /// A lifetime, or a trait with nothing said of its associated types.
    pub fn plain(name: Ident) -> BoundItem {
        BoundItem {
            span: name.span,
            name,
            assoc: Vec::new(),
            sig: None,
            effects: None,
        }
    }
}

/// As written: `Show`, `Iter<Item = i64>`, `Fn(i64) -> i64`.
impl fmt::Display for BoundItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.name.name)?;
        if let Some((params, ret)) = &self.sig {
            write!(f, "({})", comma_list(params))?;
            if let Some(ret) = ret {
                write!(f, " -> {ret}")?;
            }
        }
        if let Some(e) = &self.effects {
            write!(f, " {e}")?;
        }
        if !self.assoc.is_empty() {
            let assoc: Vec<String> = self
                .assoc
                .iter()
                .map(|a| match &a.kind {
                    AssocBoundKind::Eq(t) => format!("{} = {t}", a.name.name),
                    AssocBoundKind::Bounds(ts) => {
                        let ts: Vec<&str> = ts.iter().map(|t| t.name.as_str()).collect();
                        format!("{}: {}", a.name.name, ts.join(" + "))
                    }
                })
                .collect();
            write!(f, "<{}>", assoc.join(", "))?;
        }
        Ok(())
    }
}

/// `Item = i64` or `Item: Discard + Show` in a trait bound.
#[derive(Debug, Clone)]
pub struct AssocBound {
    pub name: Ident,
    pub kind: AssocBoundKind,
}

#[derive(Debug, Clone)]
pub enum AssocBoundKind {
    /// `Item = i64`: an *equality bound*.
    Eq(TypeExpr),
    /// `Item: Discard + Show`: the traits it implements.
    Bounds(Vec<Ident>),
}

/// `#[derive(Discard, Copy, Eq)]` before a struct or enum: implementations
/// of those traits, worked out from its fields. They're the only traits
/// that can be derived.
#[derive(Debug, Clone)]
pub struct Derive {
    /// From the `#` to the `]`.
    pub span: Span,
    pub names: Vec<Ident>,
    /// For each name, the `where` after it, if there is one.
    pub wheres: Vec<Option<Where>>,
}

/// `where T: Discard, U: Eq` after a derived trait: what its impl needs,
/// written down rather than worked out from the fields.
#[derive(Debug, Clone)]
pub struct Where {
    /// From the `where` to the end of the last bound.
    pub span: Span,
    /// As in `Generics::bounds`, but only of type parameters.
    pub bounds: Vec<(Ident, Vec<BoundItem>)>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the `struct` to the `}`, without the doc comment or attributes.
    pub span: Span,
    /// The module it's in.
    pub module: usize,
    pub public: Option<Span>,
    pub name: Ident,
    /// Its lifetime parameters, type parameters and bounds.
    pub generics: Generics,
    /// `#[derive(Copy)]`, say: its values are copied rather than moved.
    pub derives: Vec<Derive>,
    pub fields: Vec<Param>,
    /// Where its `#[intrinsic]` is, if it's one: a type a built-in library
    /// declares with no fields, whose values only the compiler makes and
    /// looks inside, by the intrinsic methods of its impl (see
    /// `check::INTRINSICS`).
    pub intrinsic: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the `enum` to the `}`, without the doc comment or attributes.
    pub span: Span,
    pub module: usize,
    pub public: Option<Span>,
    pub name: Ident,
    pub generics: Generics,
    pub derives: Vec<Derive>,
    pub variants: Vec<Variant>,
}

/// One case of an enum: `Rect(i64, i64)`, or `Empty` with no fields.
#[derive(Debug, Clone)]
pub struct Variant {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the name to the `)` after the fields, if it has any.
    pub span: Span,
    pub name: Ident,
    pub fields: Vec<TypeExpr>,
}

/// A function: at the top level, in a block, where it's *nested*, or in a
/// `trait` or `impl`, where it's a *method*. A nested `fn` means what it
/// would at the top level, so it can't use the locals around it; only its
/// name is scoped, to the block it's in.
///
/// A method's first parameter can be `self`, `mut self`, `&self` or
/// `&mut self` (or `&'a self`), its *receiver*: a parameter named `self`,
/// of type `Self`, `&Self` or `&mut Self`.
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the `fn` to the end of the body, without the doc comment or
    /// attributes.
    pub span: Span,
    /// The module it's in, or the one the function it's nested in is in.
    pub module: usize,
    /// Where its `pub` is, if it's public: only one in a module, or in an
    /// inherent impl, can be.
    pub public: Option<Span>,
    pub name: Ident,
    /// `<'a, T>` in `fn id<'a, T>(x: &'a T) -> &'a T`.
    pub generics: Generics,
    pub params: Vec<Param>,
    /// From the `(` before the parameters to the `)` after them.
    pub parens: Span,
    pub ret: TypeExpr,
    /// `with panic` after its result: what calling it may do, or `None`
    /// if nothing's written, which means nothing may happen.
    pub effects: Option<EffectSet>,
    pub body: Expr,
    /// Where its `#[intrinsic]` is, if it's one: a function `core` declares
    /// with no body, which the compiler provides. Its body here is empty.
    pub intrinsic: Option<Span>,
}

impl FnDecl {
    /// Its receiver, if it's a method that has one.
    pub fn receiver(&self) -> Option<&Param> {
        self.params.first().filter(|p| p.name.name == "self")
    }
}

/// `trait Show { fn show(&self); }`: a set of methods that types can
/// implement, each in an `impl Show for T`.
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the `trait` to the `}`.
    pub span: Span,
    pub module: usize,
    pub public: Option<Span>,
    pub name: Ident,
    /// `Eq` in `trait Ord: Eq`: traits a type must implement to implement
    /// this one, its *supertraits*.
    pub supers: Vec<Ident>,
    /// `type Item: Discard;`: its associated types.
    pub assoc: Vec<AssocDecl>,
    /// `effect e;`: its associated effects.
    pub effects: Vec<EffectDecl>,
    pub methods: Vec<TraitMethod>,
}

/// `type Item;` or `type Item: Discard + Show;` in a trait: a type each
/// impl picks, which the traits after the `:` say it must implement.
#[derive(Debug, Clone)]
pub struct AssocDecl {
    pub doc: Option<DocComment>,
    /// From the `type` to the `;`.
    pub span: Span,
    pub name: Ident,
    pub bounds: Vec<Ident>,
}

/// `type Item = i64;` in an impl: what the trait's associated type is for
/// the impl's type.
#[derive(Debug, Clone)]
pub struct AssocDef {
    pub doc: Option<DocComment>,
    /// From the `type` to the `;`.
    pub span: Span,
    pub name: Ident,
    pub ty: TypeExpr,
}

/// `effect e;` in a trait: effects each impl picks, which the trait's
/// methods can say they have, as `with Self::e`.
#[derive(Debug, Clone)]
pub struct EffectDecl {
    pub doc: Option<DocComment>,
    /// From the `effect` to the `;`.
    pub span: Span,
    pub name: Ident,
}

/// `effect e = {I::e, f};` in an impl: what the trait's associated effect
/// is for the impl's type.
#[derive(Debug, Clone)]
pub struct EffectDef {
    pub doc: Option<DocComment>,
    /// From the `effect` to the `;`.
    pub span: Span,
    pub name: Ident,
    pub effects: EffectSet,
}

/// `with panic`, or `with {alloc, panic}`: what calling a function may do,
/// its *effects*, as a set. `{}` is none.
#[derive(Debug, Clone)]
pub struct EffectSet {
    pub items: Vec<Effect>,
    /// From the `with`, or the `{` of a set after an `=`, to the end.
    pub span: Span,
    /// Was it written in braces?
    pub braced: bool,
    /// Is there a `with` before it? There is everywhere but after `effect
    /// e =`.
    pub with: bool,
}

/// One effect: `panic`, `alloc`, an effect parameter `e`, or an associated
/// effect `I::e`, `<I as Iterator>::e`.
#[derive(Debug, Clone)]
pub struct Effect {
    pub kind: EffectKind,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Debug, Clone)]
pub enum EffectKind {
    /// `panic`, `alloc`, or an effect parameter.
    Named(Ident),
    /// `I::e` or `<I as Iterator>::e`: what the impl of the trait for the
    /// type says its associated effect is.
    Proj(Box<TypeExpr>, Option<Ident>, Ident),
}

impl EffectSet {
    /// The types whose associated effects it names: the `I` of `I::e`.
    pub fn types(&self) -> impl Iterator<Item = &TypeExpr> {
        self.items.iter().filter_map(|e| match &e.kind {
            EffectKind::Proj(t, ..) => Some(&**t),
            EffectKind::Named(_) => None,
        })
    }

    /// `types`, to change.
    pub fn types_mut(&mut self) -> impl Iterator<Item = &mut TypeExpr> {
        self.items.iter_mut().filter_map(|e| match &mut e.kind {
            EffectKind::Proj(t, ..) => Some(&mut **t),
            EffectKind::Named(_) => None,
        })
    }
}

/// As written: `with panic`, `with {alloc, e}`, or `{}` after `effect e =`.
impl fmt::Display for EffectSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.with {
            f.write_str("with ")?;
        }
        let items: Vec<String> = self.items.iter().map(Effect::to_string).collect();
        match self.braced || items.len() != 1 {
            true => write!(f, "{{{}}}", items.join(", ")),
            false => f.write_str(&items[0]),
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            EffectKind::Named(n) => f.write_str(&n.name),
            EffectKind::Proj(t, Some(tr), name) => {
                write!(f, "<{t} as {}>::{}", tr.name, name.name)
            }
            EffectKind::Proj(t, None, name) => write!(f, "{t}::{}", name.name),
        }
    }
}

/// A method of a trait: `fn show(&self);`, which each impl must have, or
/// `fn twice(&self) { .. }` with a *default* body, which an impl may leave
/// out to use that.
#[derive(Debug, Clone)]
pub struct TraitMethod {
    /// Without a default, its body is an empty block where the `;` is,
    /// which means nothing.
    pub decl: FnDecl,
    /// Does it have a default?
    pub provided: bool,
}

/// `impl Show for Point { .. }`, an implementation of a trait for a type,
/// or `impl Point { .. }`, an *inherent* impl: methods of the type itself.
/// `impl<T: Show> Show for Option<T> { .. }` implements it for many types.
#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// From the `impl` to the `}`.
    pub span: Span,
    pub module: usize,
    pub generics: Generics,
    /// The trait, if it implements one.
    pub trait_name: Option<Ident>,
    /// The type it's for, which `Self` means in it.
    pub self_ty: TypeExpr,
    /// `type Item = i64;`: what the trait's associated types are.
    pub assoc: Vec<AssocDef>,
    /// `effect e = panic;`: what the trait's associated effects are.
    pub effects: Vec<EffectDef>,
    pub methods: Vec<FnDecl>,
}

/// Which `impl` or `trait` a method is in, by its index in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodOf {
    Impl(usize),
    Trait(usize),
}

impl Program {
    /// Every method with a body: those of each impl in order, then the
    /// defaults of each trait.
    pub fn methods(&self) -> Vec<(&FnDecl, MethodOf)> {
        let impls = self
            .impls
            .iter()
            .enumerate()
            .flat_map(|(i, d)| d.methods.iter().map(move |f| (f, MethodOf::Impl(i))));
        let traits = self.traits.iter().enumerate().flat_map(|(i, t)| {
            t.methods
                .iter()
                .filter(|m| m.provided)
                .map(move |m| (&m.decl, MethodOf::Trait(i)))
        });
        impls.chain(traits).collect()
    }

    /// Every function, nested ones too: the top-level ones in order, then
    /// the methods (as `methods` gives them), then the ones nested in
    /// those, and so on. Each comes with the index in this list of the
    /// function it's nested in, if it's nested.
    pub fn all_fns(&self) -> Vec<(&FnDecl, Option<usize>)> {
        let mut out: Vec<(&FnDecl, Option<usize>)> = self.fns.iter().map(|f| (f, None)).collect();
        out.extend(self.methods().into_iter().map(|(f, _)| (f, None)));
        let mut i = 0;
        while i < out.len() {
            let mut fns = Vec::new();
            nested(&out[i].0.body, Stmt::as_fn, &mut fns);
            out.extend(fns.into_iter().map(|f| (f, Some(i))));
            i += 1;
        }
        out
    }

    /// Every struct and enum, local ones too: the top-level ones in order,
    /// then the ones declared in each function in `all_fns`, in that order.
    /// Each comes with the index in `all_fns` of the function it's in, if
    /// it's local.
    pub fn all_types(&self) -> Vec<(&TypeDecl, Option<usize>)> {
        let mut out: Vec<(&TypeDecl, Option<usize>)> =
            self.types.iter().map(|d| (d, None)).collect();
        for (i, (f, _)) in self.all_fns().into_iter().enumerate() {
            let mut local = Vec::new();
            nested(&f.body, Stmt::as_type, &mut local);
            out.extend(local.into_iter().map(|d| (d, Some(i))));
        }
        out
    }
}

/// Adds what `item` finds among the statements of the blocks in `e` to
/// `out`, in order, but not in the functions nested in them.
fn nested<'a, T>(e: &'a Expr, item: fn(&'a Stmt) -> Option<&'a T>, out: &mut Vec<&'a T>) {
    if let ExprKind::Block(stmts, _) = &e.kind {
        out.extend(stmts.iter().filter_map(item));
    }
    for c in e.children() {
        nested(c, item, out);
    }
}

impl Expr {
    /// The expressions directly inside this one, in order. Not the bodies
    /// of the functions declared in it, which are separate functions.
    pub fn children(&self) -> Vec<&Expr> {
        match &self.kind {
            ExprKind::Int(..)
            | ExprKind::Bool(_)
            | ExprKind::Unit
            | ExprKind::Var(..)
            | ExprKind::Continue(_) => Vec::new(),
            ExprKind::Unary(_, x)
            | ExprKind::As(x, _)
            | ExprKind::Field(x, _)
            | ExprKind::Loop(_, x)
            | ExprKind::Unsafe(x)
            | ExprKind::Move(x) => vec![x],
            ExprKind::Return(x) | ExprKind::Break(_, x) => x.iter().map(|x| &**x).collect(),
            ExprKind::Binary(_, l, r) => vec![l, r],
            ExprKind::Call(f, args) | ExprKind::MethodCall(f, _, _, args) => {
                std::iter::once(&**f).chain(args).collect()
            }
            ExprKind::If(c, then, els) => [&**c, then].into_iter().chain(els.as_deref()).collect(),
            ExprKind::Block(stmts, tail) => stmts
                .iter()
                .filter_map(|s| match s {
                    Stmt::Let { init, .. } => init.as_ref(),
                    Stmt::Expr(e) | Stmt::BlockLike(e) => Some(e),
                    Stmt::Fn(_) | Stmt::Type(_) => None,
                })
                .chain(tail.as_deref())
                .collect(),
            ExprKind::StructLit(_, _, inits) => inits.iter().map(|i| &i.value).collect(),
            ExprKind::Variant(_, _, _, args, ..) => args.iter().collect(),
            ExprKind::Match(scrutinee, arms) => std::iter::once(&**scrutinee)
                .chain(arms.iter().map(|a| &a.body))
                .collect(),
            ExprKind::Lambda(l) => vec![&l.body],
            ExprKind::Assign(_, l, r) | ExprKind::While(_, l, r) | ExprKind::Range(l, r) => {
                vec![l, r]
            }
            ExprKind::For(f) => vec![&f.iter, &f.body],
        }
    }

    /// Does `name` appear anywhere in this, in the bodies of functions
    /// declared in it too? Whatever it refers to there.
    pub fn mentions(&self, name: &str) -> bool {
        match &self.kind {
            ExprKind::Var(n, _) => n == name,
            ExprKind::Block(stmts, _)
                if stmts
                    .iter()
                    .filter_map(Stmt::as_fn)
                    .any(|f| f.body.mentions(name)) =>
            {
                true
            }
            _ => self.children().iter().any(|c| c.mentions(name)),
        }
    }
}

#[derive(Debug, Clone)]
/// `name: type`. Used for function parameters and struct fields.
pub struct Param {
    /// Only a struct field can have one, or attributes.
    pub doc: Option<DocComment>,
    pub allows: Vec<Allow>,
    /// `mut name: type`, with where the `mut` is: only a function parameter
    /// can be `mut`.
    pub mutable: Option<Span>,
    /// `pub name: type`: only a struct field can be public.
    pub public: Option<Span>,
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Debug, Clone)]
pub enum TypeExprKind {
    /// `i64`, `bool`, a type parameter, a struct or enum with its type
    /// arguments (`Option<i64>`), or a name that doesn't exist (caught by the
    /// checker).
    Named(String, Vec<TypeExpr>),
    /// `()`
    Unit,
    /// `fn(i64, bool) -> i64`, or `fn(i64) -> i64 with panic` with the
    /// effects its calls may have.
    Fn(Vec<TypeExpr>, Box<TypeExpr>, Option<EffectSet>),
    /// `&T`, `&'a T`, `&mut T` or `&'a mut T`: a reference, with its
    /// lifetime if it's written, and whether it's `mut`.
    Ref(Option<Ident>, bool, Box<TypeExpr>),
    /// `[T]`, a slice: only right after `&` or `&mut` (see
    /// docs/10-slices.md).
    Slice(Box<TypeExpr>),
    /// `'a`, as a lifetime argument: the `'a` of `Holder<'a, i64>`. Lifetime
    /// arguments come before type arguments, and can only be there.
    Lifetime(String),
    /// `!`, the type with no values.
    Never,
    /// `<T as Iter>::Item`, or `T::Item` without the trait: a *projection*,
    /// the type an impl of the trait picks for its associated type, for the
    /// type before the `as`.
    Proj(Box<TypeExpr>, Option<Ident>, Ident),
    /// `impl Fn(i64) -> i64 + Discard`, in a function's return type: some
    /// type the function picks, of which callers only know that it
    /// implements these traits (see docs/06c.1-opaque-types.md).
    Impl(Vec<BoundItem>),
    /// `{ v: i64 | v > 0 }`: a *refinement type*, the values of the base
    /// type `.1` that make the predicate `.2` true of `.0`, the name it
    /// gives the value (see docs/08-refinement-types.md). Only as the whole
    /// type of a parameter, a result, a `let` or a struct's field
    /// (docs/08.1-field-refinements.md).
    Refined(Ident, Box<TypeExpr>, Box<Expr>),
}

impl TypeExpr {
    /// What it refines, if it's a refinement type, or else itself.
    pub fn unrefined(&self) -> &TypeExpr {
        match &self.kind {
            TypeExprKind::Refined(_, base, _) => base,
            _ => self,
        }
    }

    /// Its binder and predicate, if it's a refinement type.
    pub fn refinement(&self) -> Option<(&Ident, &Expr)> {
        match &self.kind {
            TypeExprKind::Refined(v, _, p) => Some((v, p)),
            _ => None,
        }
    }
}

/// As written: `Option<i64>`, `fn(i64) -> bool`, `()`.
impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            TypeExprKind::Named(name, args) if args.is_empty() => f.write_str(name),
            TypeExprKind::Named(name, args) => write!(f, "{name}<{}>", comma_list(args)),
            TypeExprKind::Unit => f.write_str("()"),
            TypeExprKind::Fn(params, ret, effects) => {
                write!(f, "fn({}) -> {ret}", comma_list(params))?;
                match effects {
                    Some(e) => write!(f, " {e}"),
                    None => Ok(()),
                }
            }
            TypeExprKind::Ref(lifetime, mutable, t) => {
                f.write_str("&")?;
                if let Some(l) = lifetime {
                    write!(f, "{} ", l.name)?;
                }
                if *mutable {
                    f.write_str("mut ")?;
                }
                write!(f, "{t}")
            }
            TypeExprKind::Slice(t) => write!(f, "[{t}]"),
            TypeExprKind::Lifetime(name) => f.write_str(name),
            TypeExprKind::Never => f.write_str("!"),
            TypeExprKind::Proj(t, Some(tr), name) => {
                write!(f, "<{t} as {}>::{}", tr.name, name.name)
            }
            TypeExprKind::Proj(t, None, name) => write!(f, "{t}::{}", name.name),
            TypeExprKind::Impl(bounds) => {
                let bounds: Vec<String> = bounds.iter().map(BoundItem::to_string).collect();
                write!(f, "impl {}", bounds.join(" + "))
            }
            TypeExprKind::Refined(v, base, pred) => {
                write!(
                    f,
                    "{{ {}: {base} | {} }}",
                    v.name,
                    crate::pretty::expr_text(pred)
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    /// An integer literal, `42`: its digits. Its type is worked out from
    /// how it's used; monomorphisation writes it here, for the interpreter.
    Int(u64, Option<crate::types::IntTy>),
    Bool(bool),
    Unit,
    /// `x`, or `id::<i64>` with type arguments (a *turbofish*) for a generic
    /// function. Every list of type arguments here and below is empty if none
    /// were written, and then the checker infers them.
    Var(String, Vec<TypeExpr>),
    Unary(UnOp, Box<Expr>),
    /// `x as u8`: the same integer, as another integer type.
    As(Box<Expr>, Box<TypeExpr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    /// `x.m(a, b)`, or `x.m::<i64>(a, b)` with type arguments: a call of
    /// method `m` of what `x` is, which gets `x` as its receiver.
    MethodCall(Box<Expr>, Ident, Vec<TypeExpr>, Vec<Expr>),
    /// `if c { .. } else { .. }`; the `else` is optional.
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// `{ stmt; stmt; tail }`; without a tail the block's value is `()`.
    Block(Vec<Stmt>, Option<Box<Expr>>),
    /// `unsafe { .. }`, of a `Block`: it keeps the obligations of the
    /// unsafe operations in it, so it doesn't have the effect `unsafe`
    /// they do (see docs/09-unsafe.md).
    Unsafe(Box<Expr>),
    /// `Point { x: 1, y: 2 }`, or `Pair::<i64, bool> { .. }`
    StructLit(Ident, Vec<TypeExpr>, Vec<FieldInit>),
    /// `p.x`
    Field(Box<Expr>, Ident),
    /// `Shape::Rect(2, 3)`, `Shape::Empty` with no arguments, or
    /// `Option::<i64>::None` with type arguments. Or a *path*, which looks
    /// the same: `Point::new(1, 2)` calls a method of `Point`, `Show::show(x)`
    /// one of trait `Show`, and `Point::len` with no `(..)` is that method,
    /// as a function value. Only the checker can tell which it is. The
    /// `bool` says whether the `(..)` is there.
    Variant(Ident, Vec<TypeExpr>, Ident, Vec<Expr>, bool),
    /// `match e { pat => e, .. }`
    Match(Box<Expr>, Vec<Arm>),
    /// `|x, y: i64| x + y`. Shared, because the closure values the
    /// interpreter makes from it point to it.
    Lambda(Arc<Lambda>),
    /// `place = value`, or `place += value` and the like with an operator.
    /// The place is a variable, a field or a `*`.
    Assign(Option<BinOp>, Box<Expr>, Box<Expr>),
    /// `while c { .. }`, or `'a: while c { .. }` with a label.
    While(Option<Label>, Box<Expr>, Box<Expr>),
    /// `loop { .. }`: runs its body again and again, until a `break`.
    Loop(Option<Label>, Box<Expr>),
    /// `for p in it { .. }`. Only until paths are resolved, which rewrites
    /// it as the `loop` it means (see `check/modules.rs`).
    For(Box<ForLoop>),
    /// `a..b`. Only until paths are resolved, which rewrites it as a call
    /// of `core::ops::Range::new`.
    Range(Box<Expr>, Box<Expr>),
    /// `return e`, or `return` for `return ()`.
    Return(Option<Box<Expr>>),
    /// `break e` out of a `loop`, giving it the value of `e`, or `break`
    /// out of any loop. Out of the innermost, or with a label, `break 'a`,
    /// out of the one with that label.
    Break(Option<Ident>, Option<Box<Expr>>),
    /// `continue`: on to the next time round the loop, the innermost or
    /// `continue 'a`, the one labelled `'a`.
    Continue(Option<Ident>),
    /// The value at a place, moved out of it, rather than copied. Only
    /// from `lift` on, which writes one around each place the borrow
    /// checker moved out of, as it decided by the types it knew: in
    /// generic code, a value of a type parameter that isn't known to be
    /// `Copy` moves, whatever type it turns out to be (see
    /// `Info::moves`). A place used by value without one is copied.
    Move(Box<Expr>),
}

/// `for pat in iter { body }`, or with a label.
#[derive(Debug, Clone)]
pub struct ForLoop {
    pub label: Option<Label>,
    pub pat: Pattern,
    /// Where the `in` is.
    pub in_span: Span,
    pub iter: Expr,
    pub body: Expr,
}

/// `'a:` before a `loop` or `while`, for a `break` or `continue` in it to
/// say which loop it's for.
#[derive(Debug, Clone)]
pub struct Label {
    /// With its `'`: `'a`.
    pub name: Ident,
    /// From the label to the `loop` or `while`, `:` and all.
    pub head: Span,
}

impl Expr {
    /// Is it a *place*: something that names where a value is, rather than
    /// computing a new one? A variable, a field of a place, or what a `*`
    /// points to. Others are *values*. (A variable may name a function,
    /// which isn't a place; only the checker can tell.)
    pub fn is_place(&self) -> bool {
        match &self.kind {
            ExprKind::Var(..) | ExprKind::Unary(UnOp::Deref, _) => true,
            ExprKind::Field(x, _) => x.is_place(),
            _ => false,
        }
    }

    /// What a `Move` moves out of, or else this.
    pub fn unmoved(&self) -> &Expr {
        match &self.kind {
            ExprKind::Move(x) => x,
            _ => self,
        }
    }
}

/// An anonymous function: `|x| x + 1`, `|x: i64| -> i64 { x + 1 }`, or `|| 1`
/// with no parameters. Any type left out is inferred. `move |x| ..` takes
/// what it captures by value.
#[derive(Debug, Clone)]
pub struct Lambda {
    pub id: NodeId,
    /// Where the `move` is, if it's there.
    pub moves: Option<Span>,
    pub params: Vec<LambdaParam>,
    pub ret: Option<TypeExpr>,
    pub body: Expr,
    /// The names the body uses but doesn't bind itself: its *free
    /// variables*, which a closure made from it has to capture. Some may be
    /// functions, which need no capturing.
    pub captures: Vec<String>,
    /// How it captures each of the variables it does, in order, once the
    /// checker has worked that out, and for one captured by value, whether
    /// that moves it rather than copying it, as for `ExprKind::Move`:
    /// `lift` writes them here, for the interpreter. Empty before.
    pub modes: Vec<(String, Capture, bool)>,
}

/// How a closure holds a variable it captures: a reference to it, `&` or
/// `&mut`, or the value, moved or copied in. `Unique` is a `&mut` to a
/// reference that the closure changes what it points to through, which, as
/// for any such reborrow, needs the variable to be unique but not `mut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    Ref,
    Unique,
    Mut,
    Value,
}

impl Lambda {
    pub fn new(id: NodeId, params: Vec<LambdaParam>, ret: Option<TypeExpr>, body: Expr) -> Lambda {
        let mut bound: Vec<String> = params.iter().map(|p| p.name.name.clone()).collect();
        let mut captures = Vec::new();
        free_vars(&body, &mut bound, &mut captures);
        Lambda {
            id,
            moves: None,
            params,
            ret,
            body,
            captures,
            modes: Vec::new(),
        }
    }
}

/// Adds the variables `e` uses that aren't in `bound` to `out`, once each.
/// `bound` grows and shrinks with the scopes inside `e`, so a `let` or
/// pattern in there shadows an outer name only where it's in scope.
fn free_vars(e: &Expr, bound: &mut Vec<String>, out: &mut Vec<String>) {
    let mut go = |e: &Expr, bound: &mut Vec<String>| free_vars(e, bound, out);
    match &e.kind {
        ExprKind::Int(..) | ExprKind::Bool(_) | ExprKind::Unit | ExprKind::Continue(_) => {}
        // A path, `a::f`, names a function, which needs no capturing.
        ExprKind::Var(name, _) if name.contains("::") => {}
        ExprKind::Var(name, _) => {
            if !bound.contains(name) && !out.contains(name) {
                out.push(name.clone());
            }
        }
        ExprKind::Unary(_, x)
        | ExprKind::As(x, _)
        | ExprKind::Field(x, _)
        | ExprKind::Loop(_, x)
        | ExprKind::Unsafe(x)
        | ExprKind::Move(x) => go(x, bound),
        ExprKind::Return(x) | ExprKind::Break(_, x) => x.iter().for_each(|x| go(x, bound)),
        ExprKind::Binary(_, l, r) => {
            go(l, bound);
            go(r, bound);
        }
        ExprKind::Call(f, args) | ExprKind::MethodCall(f, _, _, args) => {
            go(f, bound);
            args.iter().for_each(|a| go(a, bound));
        }
        ExprKind::If(c, then, els) => {
            go(c, bound);
            go(then, bound);
            els.iter().for_each(|e| go(e, bound));
        }
        ExprKind::Block(stmts, tail) => {
            let scope = bound.len();
            // A nested `fn`'s name is in scope in the whole block, and its
            // body is a function of its own, which can't capture anything.
            let fns = stmts.iter().filter_map(Stmt::as_fn);
            bound.extend(fns.map(|f| f.name.name.clone()));
            for stmt in stmts {
                match stmt {
                    Stmt::Let { pat, ty, init, .. } => {
                        init.iter().for_each(|e| go(e, bound));
                        // What a refinement names is read when it's
                        // checked, at run time by the interpreter.
                        if let Some((v, pred)) = ty.as_ref().and_then(|t| t.refinement()) {
                            bound.push(v.name.clone());
                            go(pred, bound);
                            bound.pop();
                        }
                        pattern_binds(pat, bound);
                    }
                    Stmt::Expr(e) | Stmt::BlockLike(e) => go(e, bound),
                    Stmt::Fn(_) | Stmt::Type(_) => {}
                }
            }
            tail.iter().for_each(|t| go(t, bound));
            bound.truncate(scope);
        }
        ExprKind::StructLit(_, _, inits) => inits.iter().for_each(|i| go(&i.value, bound)),
        ExprKind::Variant(_, _, _, args, ..) => args.iter().for_each(|a| go(a, bound)),
        ExprKind::Match(scrutinee, arms) => {
            go(scrutinee, bound);
            for arm in arms {
                let scope = bound.len();
                pattern_binds(&arm.pat, bound);
                go(&arm.body, bound);
                bound.truncate(scope);
            }
        }
        ExprKind::Lambda(l) => {
            let scope = bound.len();
            bound.extend(l.params.iter().map(|p| p.name.name.clone()));
            go(&l.body, bound);
            bound.truncate(scope);
        }
        ExprKind::Assign(_, l, r) | ExprKind::While(_, l, r) | ExprKind::Range(l, r) => {
            go(l, bound);
            go(r, bound);
        }
        ExprKind::For(f) => {
            go(&f.iter, bound);
            let scope = bound.len();
            pattern_binds(&f.pat, bound);
            go(&f.body, bound);
            bound.truncate(scope);
        }
    }
}

/// Adds the variables `p` binds to `out`.
fn pattern_binds(p: &Pattern, out: &mut Vec<String>) {
    match &p.kind {
        PatternKind::Bind(name, _) => out.push(name.clone()),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
            pats.iter().for_each(|p| pattern_binds(p, out));
        }
        PatternKind::Move(p) => pattern_binds(p, out),
        PatternKind::Struct(_, _, fields, _) => {
            fields.iter().for_each(|f| pattern_binds(&f.pat, out))
        }
        PatternKind::Wild
        | PatternKind::Int(_)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}

#[derive(Debug, Clone)]
pub struct LambdaParam {
    /// Where the `mut` is, if it's `mut`.
    pub mutable: Option<Span>,
    pub name: Ident,
    pub ty: Option<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct Arm {
    pub pat: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let pat: ty = init;`. The pattern must be irrefutable: `let x = ..`,
    /// `let Point { x, y } = ..`, but not `let Shape::Empty = ..`.
    Let {
        pat: Pattern,
        /// Boxed, since it's rare and would make every `Stmt` bigger.
        ty: Option<Box<TypeExpr>>,
        /// `None` for `let x;`, a variable given its value later, which
        /// only a variable can be.
        init: Option<Expr>,
        /// From the `let` to the `;`.
        span: Span,
    },
    /// `e;`: any type, value discarded.
    Expr(Expr),
    /// An `if`, `match` or block statement with no `;` after it, like
    /// `if c { print(1) }`. As in Rust, it must have type `()`.
    BlockLike(Expr),
    /// `fn helper() { .. }`: a nested function. Its name is in scope in the
    /// whole block, before the `fn` as well as after. Boxed, like `Let`'s
    /// type.
    Fn(Box<FnDecl>),
    /// `struct Point { .. }` or `enum Shape { .. }`: a local type. Like a
    /// nested function, it means what it would at the top level, and its
    /// name is in scope in the whole block.
    Type(Box<TypeDecl>),
}

impl Stmt {
    pub fn as_fn(&self) -> Option<&FnDecl> {
        match self {
            Stmt::Fn(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_type(&self) -> Option<&TypeDecl> {
        match self {
            Stmt::Type(d) => Some(d),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
    pub id: NodeId,
}

impl Pattern {
    /// Where the name is, in a binding: `x` in `mut x`.
    pub fn name_span(&self) -> Span {
        match &self.kind {
            PatternKind::Bind(name, _) => Span::new(self.span.end - name.len(), self.span.end),
            _ => self.span,
        }
    }

    /// The variables it binds.
    pub fn binds(&self) -> Vec<String> {
        let mut out = Vec::new();
        pattern_binds(self, &mut out);
        out
    }
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    /// `_`: matches anything, binds nothing.
    Wild,
    /// `x`: matches anything and names it. `mut x` if the `bool` is set:
    /// then the variable can be changed.
    Bind(String, bool),
    Int(i128),
    /// An integer range, with both bounds inclusive: `1..=9` is
    /// `Range(Some(1), Some(9), true)`, `1..10` is `Range(Some(1), Some(9),
    /// false)`, `10..` runs to the greatest value of the type matched, with
    /// no upper bound, and `..=-1` from the least. The `bool` says whether
    /// it was written with `..=`, so suggestions can keep the style. `lo >
    /// hi` means the range is empty, which the checker rejects.
    Range(Option<i128>, Option<i128>, bool),
    Bool(bool),
    Unit,
    /// `Shape::Rect(w, _)`, or `Option::<i64>::Some(x)` with type arguments.
    Variant(Ident, Vec<TypeExpr>, Ident, Vec<Pattern>),
    /// `Point { x, y: 0 }`, or `Point { x, .. }` to ignore the other fields
    /// (the `bool`).
    Struct(Ident, Vec<TypeExpr>, Vec<FieldPat>, bool),
    /// `Shape::Circle(r) | Shape::Rect(r, _)`: matches if any alternative
    /// does. Every alternative must bind the same variables, at the same types.
    Or(Vec<Pattern>),
    /// A pattern that moves out of what it matches, as `ExprKind::Move`
    /// does, and only from `lift` on: a binding by value of what the
    /// borrow checker didn't know to be `Copy`, which moves it into the
    /// variable, or a struct or enum pattern taking apart, by value, what
    /// it didn't know to be `Discard`, which uses it up. Without one, a
    /// binding copies, and a struct or enum pattern leaves what it takes
    /// apart where it is.
    Move(Box<Pattern>),
}

/// `y: 0` in a struct pattern. Plain `x` is short for `x: x`.
#[derive(Debug, Clone)]
pub struct FieldPat {
    pub name: Ident,
    pub pat: Pattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    /// `*x`: what reference or box `x` points to.
    Deref,
    /// `&x`: a shared reference to `x`.
    Ref,
    /// `&mut x`: a unique, mutable reference to `x`.
    RefMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    /// `&`: of integers, bit by bit.
    BitAnd,
    /// `|`: of integers, bit by bit.
    BitOr,
    /// `^`: of integers, bit by bit.
    BitXor,
    /// `<<`: times 2 to the power of the right side, a `u32`.
    Shl,
    /// `>>`: divided by 2 to the power of the right side, rounding down.
    Shr,
}

impl BinOp {
    /// Whether it's one of `&`, `|`, `^`, `<<` and `>>`.
    pub fn bitwise(self) -> bool {
        matches!(
            self,
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr
        )
    }

    /// Whether it's one of `+`, `-`, `*`, `/` and `%`.
    pub fn arith(self) -> bool {
        matches!(
            self,
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem
        )
    }

    /// Whether `self` may be an operand of `outer` without parentheses.
    /// Comparisons and `&&` and `||` take anything that binds more
    /// tightly; arithmetic, by its precedence. But how a bitwise operator
    /// or a shift goes with another, or with arithmetic, has no precedence
    /// to remember: it's said with parentheses, `(a & b) | c`. A chain of
    /// one is fine: `a | b | c`.
    pub fn nests_in(self, outer: BinOp) -> bool {
        if outer.bitwise() {
            self == outer
        } else if outer.arith() {
            !self.bitwise()
        } else {
            true
        }
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
            UnOp::Deref => "*",
            UnOp::Ref => "&",
            UnOp::RefMut => "&mut ",
        })
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
        })
    }
}
