//! Refinement types: `{ d: i64 | d != 0 }`, the `i64`s that aren't 0 (see
//! docs/08-refinement-types.md, docs/08.1-field-refinements.md,
//! docs/53-refined-type-arguments.md, docs/55-free-theorems.md and
//! docs/60-refined-type-arguments.md).
//!
//! A refinement is the whole type of a parameter, a result, a `let` or a
//! struct's field, or a type argument there, at any depth: of an enum's
//! (`Option<{ d: u8 | d < 10 }>`: of each field of a variant whose type is
//! that parameter, if the value is that variant; `Checker::refined_arg`),
//! or of anything else, *carried*: what every value at that position is,
//! which code generic in the parameter keeps by parametricity (`targs`:
//! each value's objects and each place say it of their positions, and each
//! call works out what its type parameters are from its arguments). It's
//! never part of any other type. It isn't in `Type` at all: unification, traits and
//! monomorphisation never see one. What it says is checked by a pass of its
//! own over each body, once its types are known and the borrow checker has
//! checked it (`Checker::refine`, see docs/64-phase-order.md), which
//! keeps track of what's known at each point (*facts*) and proves from them
//! what each refinement needs, with the procedure in `refine/logic.rs`.
//! The pass walks the MIR the borrow checker built of the body
//! (`refine/mir.rs`, docs/65-refinements-on-mir.md and
//! docs/66-one-walk.md), where every place is a local and the fields,
//! variants' fields and `*`s from it:
//!
//! - an argument has its parameter's refinement, a result its function's,
//!   and a value given to a refined `let`, or assigned to one, its;
//! - a struct's literal has its fields' refinements, and so does it after
//!   an assignment to a field one of them is about;
//! - an operator can't overflow or divide by zero: one proved not to has
//!   no `panic`, which is how refinements make arithmetic total.
//!
//! Facts come from refined parameters and `let`s, from the refinements of
//! the fields of every struct value, from what calls of refined functions
//! return, from the conditions of `if`, `while`, `&&`, `||` and the
//! patterns of a `match` on an `i64`, from what a `let` or an assignment
//! gives a *place* (a variable, or a field or `*` of one: `self.start`),
//! and from operators that returned, as they'd have panicked otherwise.
//! What an operator gives is exact where it's linear (`+`, `-`, `!` of an
//! integer, `*`, `/`, `%`, `<<` and `>>` by a constant), and otherwise a
//! new value of which some lemmas say something (`bitwise`).
//! They're about *values*: each time a place gets a new one, it's a new
//! variable of the logic (*static single assignment*), so a fact, once
//! true, stays true, and only what a place stands for changes.
//!
//! Soundness: every fact in the context holds of the values the program
//! has at that point, which is an induction on how the program runs. A fact
//! is added only where it's been proved, where a condition is true, after
//! an operator returned, or of a struct value, which has its fields'
//! refinements (below). A place stands for a new variable of the logic
//! whenever it may change: when it, or what it's in, is assigned; at the
//! start of each time round a loop that assigns it, or lends it to a call
//! (though the measures the loop's frame keeps of it stay what they were:
//! `Walk::frames_of`);
//! after a branch that does, where it's whichever value the branch taken
//! gave it; and after a call it's lent to `&mut`, when nothing can keep
//! the reference after the call. Lent `&mut` any other way, by a `&mut`
//! that may outlive where it's made, a closure's capture or a pattern's
//! binding, it's forgotten where it's lent: nothing can read or write it
//! while the loan lives, so it's next read after, as a value nothing is
//! known of but its refinements, which every write through the loan,
//! inside a closure too, must keep, and the measures a closure's body
//! keeps of it (docs/69-loans.md). A `&` can't change what it
//! points to, as nothing in T can be changed through one, so `*r` for a
//! `let r = &p;` is `p` (`Walk::alias`). A refined variable, a field a
//! refinement is about, and what a refined reference points to can't be
//! borrowed `&mut` but straight to a call-scoped parameter (`call_scoped`),
//! after which their refinements, and the struct's about the field, are
//! proved again, of what the call left, before anything assumes them
//! (`Walk::post_now`, `Walk::reproving`); a place in a refined variable
//! (through fields and boxes) can't be at all. So the refinements of a
//! place hold of its value whenever it can be read: every way of making or
//! changing one checks them, a call lent one runs while nothing can read it
//! (and nothing unwinds), and anything else, generic code included, only
//! moves whole values.
//!
//! Postconditions (`ensures`, a result's refinement) are proved by the
//! callee at each return, of what's behind its call-scoped `&mut`s then,
//! and learned by the caller of the places it lent, once they're forgotten
//! (`Walk::post_now`): nothing made from such a parameter outlives the call, so
//! what's behind it at return is what's there when the caller next reads
//! it. An intrinsic's may be about a `&mut` whose borrow outlives the call,
//! its value when that ends (a prophecy, trusted), which is also when the
//! caller next reads the place. A refinement's predicate
//! names only what can't change. What a call returns has its function's
//! refinement by induction on how deep the calls go, as for effects: a call
//! that never returns gives nothing to count on. And what's called through
//! a value can't have refined parameters, nor a trait's method, nor an
//! impl's of one, so every call that skips a check is of a function with
//! nothing to check. An impl's method of a trait can refine its result and
//! ensure more, which a call learns only where it's known to be of that
//! impl (`Walk::callee_at`): monomorphisation picks the same one.
//!
//! A predicate can name a struct's field, of anything it names, at any
//! depth (`s.head.len()`; `Checker::pred_place`): to the logic a struct's
//! value is its fields' values, so a field is a function of the value, as a
//! measure is, read as the value it's of is. Behind a `&mut` it follows a
//! measure's rules (`Checker::pred_mutable`); in a signature or a field's
//! refinement it's seen as far as that is (`Checker::field_reach`), so no
//! proof outside a module hangs on what the module keeps to itself. Equal
//! values have equal measures (`Walk::same_measures`), where branches meet
//! and where a free theorem says which value a result is.
//!
//! An enum's value has *parts* (`Parts`): which variant it is and each
//! variant's fields, functions of the value as measures are; a refinement
//! of a type argument is a fact about a field under the condition that the
//! value is that variant (`Walk::part`), so one that can't hold of a
//! variant that isn't there says nothing.
//!
//! A call of a generic function says, of each of its *free* type
//! parameters (only markers bound one, and it's only whole or in enums'
//! fields in the signature: `refine/free.rs`), that each value of it in the
//! result is one of those in the arguments, of the variants they're of
//! (`Walk::free_theorems`): it can't make one (parametricity, lemma P1 of
//! docs/55-free-theorems.md), which the interpreter checks at each call.

mod diagnostics;
mod free;
mod logic;
pub use logic::{Work, work};
mod mir;
mod targs;

use super::Checker;
use super::typed::Res;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::traits::{INT, SIGNED, UNSIGNED};
use crate::types::{IntTy, Type, all_fields, struct_fields};
use diagnostics::Invalidation;
pub(super) use diagnostics::ProofFailure;
use logic::{Failure, Formula, Kind, Lin, Var};
use std::collections::{HashMap, HashSet};

/// The refinements written in a type that can have them, the whole
/// value's and those of its type arguments (see
/// docs/53-refined-type-arguments.md and docs/60-refined-type-arguments.md),
/// whose names are in `Checker::pred_names`.
#[derive(Debug, Clone)]
pub(super) struct Refinement {
    parts: Vec<Part>,
    /// The type it refines, without them, if it's of a value.
    base: Option<Type>,
}

/// The way to a part of a value: the variant and field of each enum on
/// the way to it.
type Way = Vec<(usize, usize)>;

/// For each parameter of a function, if it's lend-framed (`lend_framed`),
/// the type parameters its result can write values of.
pub(super) type Framed = Vec<Option<Vec<String>>>;

/// What a change to a place keeps: of each place in it, the measures it
/// leaves as they were (`Walk::frames_of`).
type Keeps = Vec<(Place, Vec<String>)>;

/// One predicate of a refinement, and the parts of the value it's of.
#[derive(Debug, Clone)]
struct Part {
    pred: Expr,
    /// Where each part of the value it's of is, as the variant and field
    /// of each enum on the way to it: one way, `[]`, for the whole value;
    /// for the `T` of `Option<T>`, `[(1, 0)]`, the field of `Some`. None
    /// if it's `carried`.
    ways: Vec<Way>,
    /// Where it is in the type, as `TypeExpr::refinements` says, but with
    /// a type's lifetime arguments not counted: a position (`targs::Pos`).
    path: Vec<usize>,
    /// Is it somewhere a way through enums doesn't get to: in a struct's
    /// type arguments, a tuple's, behind a reference or a box, or a slice's
    /// or array's elements? Then it's what every value there has, which
    /// generic code keeps by parametricity (`targs`).
    carried: bool,
}

impl Part {
    fn text(&self) -> String {
        crate::pretty::expr_text(&self.pred)
    }
}

impl Refinement {
    /// One of the whole value: a postcondition's, say.
    fn whole(pred: Expr) -> Refinement {
        Refinement {
            parts: vec![Part {
                pred,
                ways: vec![Vec::new()],
                path: Vec::new(),
                carried: false,
            }],
            base: None,
        }
    }

    /// Does it say anything a way through enums gets to: of the whole
    /// value, or a type argument of an enum's?
    fn has_ways(&self) -> bool {
        self.parts.iter().any(|p| !p.carried)
    }

    /// Does it say anything of what every value somewhere a way doesn't
    /// get to is (`Part::carried`)?
    fn has_carried(&self) -> bool {
        self.parts.iter().any(|p| p.carried)
    }

    /// It without its carried parts, if it has any others.
    fn ways_only(&self) -> Option<Refinement> {
        self.has_ways().then(|| Refinement {
            parts: self.parts.iter().filter(|p| !p.carried).cloned().collect(),
            base: self.base.clone(),
        })
    }

    /// Its predicates.
    fn preds(&self) -> impl Iterator<Item = &Expr> {
        self.parts.iter().map(|p| &p.pred)
    }
}

/// The symbol of `str::len`, the measure.
const STR_LEN: &str = "<str>::len";

/// What a predicate's part is, in the logic: an integer, of ℤ, whatever
/// the integer types of what it names, or a `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sort {
    Int,
    Bool,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Sort::Int => "an integer",
            Sort::Bool => "`bool`",
        })
    }
}

/// The refinements in a function's signature, by symbol (`Checker::refined`).
#[derive(Debug, Clone)]
pub(super) struct RefinedSig {
    name: String,
    /// Its parameters' names.
    names: Vec<String>,
    /// Its type parameters, and its impl's, in order, as its type arguments
    /// are: what its const parameters are at a call.
    type_params: Vec<String>,
    params: Vec<Option<Refinement>>,
    ret: Option<Refinement>,
    /// What it `ensures`, with names as in `ret`.
    ensures: Vec<Refinement>,
    /// Which parameters are call-scoped `&mut`s (`call_scoped`): a call
    /// lending a place to one can learn what `ensures` says of it, and a
    /// field a refinement is about can be lent to one. Lend-framed ones
    /// count, once the signature's checked: what they ensure is true when
    /// the borrow ends.
    scoped: Vec<bool>,
    /// The lend-framed parameters (`lend_framed`), each with the type
    /// parameters its result can write values of.
    framed: Framed,
}

/// The refinements of a struct's fields, by the struct's symbol
/// (`Checker::field_refined`). They're kept by every value of it.
#[derive(Debug, Clone)]
pub(super) struct RefinedFields {
    /// The struct's name.
    name: String,
    /// Its fields' names.
    names: Vec<String>,
    refs: Vec<Option<Refinement>>,
    /// The fields each field's refinement names, other than itself.
    named: Vec<Vec<usize>>,
}

impl RefinedFields {
    /// The refined fields whose refinements are about field `i`: its own,
    /// and those that name it.
    fn about(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.refs.len())
            .filter(move |&j| self.refs[j].is_some() && (j == i || self.named[j].contains(&i)))
    }

    /// Is field `i`'s refinement about it alone: naming no other field,
    /// and named by no other's? Then it's what's declared of the field's
    /// place (`targs::field_decl`), which a `&mut` to it says, so what's
    /// written through that has it (D6 of docs/67-refinement-aliases.md).
    fn alone(&self, i: usize) -> bool {
        self.refs.get(i).is_some_and(Option::is_some)
            && self.named[i].is_empty()
            && self.about(i).all(|j| j == i)
    }

    /// Why field `i` would have to be proved again after a call it's
    /// lent to, if it would: a refinement is about it.
    fn constrained(&self, i: usize) -> Option<String> {
        let j = self.about(i).next()?;
        Some(self.why(i, j))
    }

    /// Why field `i` can't be borrowed `&mut` but straight to a call, if
    /// it can't: a refinement about it that isn't about it alone, which a
    /// reference to it couldn't say.
    fn refused(&self, i: usize) -> Option<String> {
        match self.alone(i) {
            true => None,
            false => self.constrained(i),
        }
    }

    /// Why field `i` of a struct at no place, a temporary's, can't be lent
    /// `&mut` even to a call, if it can't: a refinement about it would
    /// have to be proved again after the call, of the struct, which is
    /// nowhere (lemma W4 of docs/63-what-is-declared.md). Not its own
    /// refinement if that's all carried and names no field: what's written
    /// through the reference keeps it, as the reference says it.
    fn constrained_unplaced(&self, i: usize) -> Option<String> {
        let j = self.about(i).find(|&j| {
            j != i
                || !self.named[j].is_empty()
                || self.refs[j].as_ref().is_some_and(Refinement::has_ways)
        })?;
        Some(self.why(i, j))
    }

    fn why(&self, i: usize, j: usize) -> String {
        match j == i {
            true => format!(
                "field `{}` of `{}` has a refinement",
                self.names[i], self.name
            ),
            false => format!(
                "the refinement of field `{}` of `{}` names it",
                self.names[j], self.name
            ),
        }
    }
}

/// The refinements of an enum's variants' fields, by the enum's symbol
/// (`Checker::variant_refined`): every value of a variant has its
/// fields' (docs/67-refinement-aliases.md).
#[derive(Debug, Clone)]
pub(super) struct RefinedVariants {
    /// The enum's name.
    name: String,
    variants: Vec<RefinedVariant>,
}

/// One variant's, of `RefinedVariants`.
#[derive(Debug, Clone)]
struct RefinedVariant {
    name: String,
    refs: Vec<Option<Refinement>>,
    /// The fields before it each field's refinement names, by their
    /// binders.
    named: Vec<Vec<usize>>,
}

impl RefinedVariant {
    /// Is field `j`'s refinement about it alone: naming no other field, and
    /// named by no other's? Then it's what's declared of the field's place
    /// (`targs::variant_decl`), which a `&mut` to it says.
    fn alone(&self, j: usize) -> bool {
        self.named.get(j).is_some_and(Vec::is_empty) && !self.named.iter().any(|n| n.contains(&j))
    }

    /// Why field `j` can't be borrowed `&mut`, if it can't: another
    /// field's refinement names it, or its own names another, which a
    /// write through the reference couldn't keep.
    fn constrained(&self, enum_name: &str, j: usize) -> Option<String> {
        if let Some(k) = (0..self.named.len()).find(|&k| self.named[k].contains(&j)) {
            return Some(format!(
                "the refinement of field {k} of `{enum_name}::{}` names it",
                self.name
            ));
        }
        (!self.named.get(j)?.is_empty()).then(|| {
            format!(
                "field {j} of `{enum_name}::{}` has a refinement that names another field",
                self.name
            )
        })
    }
}

/// A measure: a method of `&self` marked `#[measure]`, which a predicate
/// can call (see docs/21-measures.md). What it gives depends on the value
/// alone, and it always gives something: so it's a function of the value,
/// in the logic as in the program.
#[derive(Debug, Clone)]
pub(super) struct Measure {
    name: String,
    /// What it gives: an integer type or `bool`.
    ret: Type,
    /// The module it's in, where it's its body; elsewhere it's only what's
    /// been learned of it.
    module: usize,
    /// Its body, a term of the logic over `self`'s fields and measures, or
    /// `None` for an intrinsic.
    body: Option<Expr>,
    /// Is it of a slice or a string, so of the reference to one?
    unsized_self: bool,
    /// Is it an array's intrinsic, its length?
    of_array: bool,
    /// The measures its body uses, once it's been checked.
    uses: Vec<String>,
    /// Does its body read a field? If not, it's only other measures, so
    /// it can be its body anywhere, without seeing what its module hides.
    reads_fields: bool,
}

/// A signature's or a struct's refinements, to be checked once every
/// impl is collected, as a predicate may call a measure of any.
#[derive(Debug, Clone)]
pub(super) enum Pending {
    Sig {
        f: Box<FnDecl>,
        params: Vec<Type>,
        ret: Type,
        /// May its parameters have refinements? Not in a trait's methods,
        /// nor an impl's of one.
        allowed: bool,
        /// May its result, and `ensures`? Not in a trait's methods.
        results: bool,
        owner: (String, String),
        /// Its const parameters, and its impl's, as declared.
        consts: Vec<Ident>,
    },
    Fields {
        s: Box<StructDecl>,
        symbol: String,
    },
    Variants {
        e: Box<EnumDecl>,
        symbol: String,
    },
    /// A refinement alias, by symbol: what it stands for is checked where
    /// it's declared, for any arguments (D1 of docs/67-refinement-aliases.md).
    Alias(String),
}

/// What a name in a predicate is.
#[derive(Debug, Clone)]
pub(super) enum PredName {
    /// The refinement's own, the value refined.
    Binder,
    /// A field of the struct, by index, in a field's refinement.
    Field(usize),
    /// The function's parameter, by index.
    Param(usize),
    /// A local variable, by id.
    Local(NodeId),
    /// A const parameter of the function, or of the struct, in a field's
    /// refinement, by name.
    Const(String),
    /// A const item, by symbol: its value (see `Walk::const_item`).
    Item(String),
}

/// What a name a predicate is checked in names: a value, of this type,
/// declared there, or a `mut` variable.
enum Named {
    Value(Type, PredName, Span),
    Mut,
    /// One it can't name, and why.
    Not(String),
}

/// What a path in a predicate names (`Checker::pred_place`): of what type,
/// from which name, written `name`, and whether it's behind a `&mut` on
/// the way, so that what's there can change.
struct PredPlace {
    ty: Type,
    named: PredName,
    name: String,
    through_mut: bool,
}

impl Checker {
    /// Checks the refinements in the signature of `f`, whose parameters
    /// and result have the types `params` and `ret` without them, and
    /// keeps them. `allowed` is false in a trait's methods and an impl's
    /// for a trait, which can't have any.
    pub(super) fn refined_signature(
        &mut self,
        f: &FnDecl,
        params: &[Type],
        ret: &Type,
        allowed: bool,
    ) -> Result<(), Error> {
        // `Self` is the impl's type, in an impl's method.
        let (params, ret) = match &self.sig_self {
            Some(t) => {
                let s = crate::types::Subst::new(&["Self".to_string()], std::slice::from_ref(t));
                (params.iter().map(|p| s.apply(p)).collect(), s.apply(ret))
            }
            None => (params.to_vec(), ret.clone()),
        };
        let (params, ret) = (&params[..], &ret);
        if f.measure.is_some() {
            self.measure_signature(f, params, ret)?;
        }
        let written: Vec<TypeExpr> = (f.params.iter().map(|p| &p.ty).chain([&f.ret]))
            .map(|t| self.written_out(t))
            .collect();
        if written.iter().all(|t| t.refinements().is_empty()) && f.ensures.is_empty() {
            return Ok(());
        }
        // Its predicates may call measures of impls not collected yet, so
        // they're checked once all are.
        let mut consts = self.const_decls.clone();
        consts.extend(
            f.generics
                .types
                .iter()
                .filter(|p| f.generics.is_const(p))
                .cloned(),
        );
        // An impl's method of a trait can say more of what it gives than
        // the trait does, for the calls known to be of it.
        let results = allowed || self.sig_self.is_some();
        self.pending_refined.push(Pending::Sig {
            f: Box::new(f.clone()),
            params: params.to_vec(),
            ret: ret.clone(),
            allowed,
            results,
            owner: self.sig_owner.clone(),
            consts,
        });
        Ok(())
    }

    /// Which parameters of function `symbol` are call-scoped `&mut`s
    /// (`call_scoped`); none, if it isn't one the checker knows.
    pub(super) fn scoped_params(&self, symbol: &str) -> Vec<bool> {
        if let Some(sig) = self.refined.get(symbol) {
            return sig.scoped.clone();
        }
        let Some(f) = self.fns.iter().find(|f| f.symbol == symbol) else {
            return Vec::new();
        };
        match &f.scheme.ty {
            Type::Fn(sig) => call_scoped(&sig.params, &sig.ret, &f.scheme.bounds),
            _ => Vec::new(),
        }
    }

    /// The parameters of function `symbol` its refinements name, by index.
    pub(super) fn named_params(&self, symbol: &str) -> Vec<usize> {
        let mut out = Vec::new();
        let Some(sig) = self.refined.get(symbol) else {
            return out;
        };
        for r in sig
            .params
            .iter()
            .chain([&sig.ret])
            .flatten()
            .chain(&sig.ensures)
        {
            for pred in r.preds() {
                each_expr(pred, &mut |e| {
                    if let Some(PredName::Param(j)) = self.pred_names.get(&e.id) {
                        out.push(*j);
                    }
                });
            }
        }
        out
    }

    /// Checks the refinements whose checks waited for every impl to be
    /// collected (see `Pending`).
    pub(super) fn pending_refinements(&mut self) -> Result<(), Error> {
        for p in std::mem::take(&mut self.pending_refined) {
            match p {
                Pending::Sig {
                    f,
                    params,
                    ret,
                    allowed,
                    results,
                    owner,
                    consts,
                } => {
                    let id = self.fns.iter().position(|d| d.symbol == owner.0);
                    self.pred_owner = id.map(super::scope::FnId);
                    self.sig_owner = owner;
                    self.module = f.module;
                    // What its type parameters implement: a `T: Int`'s
                    // values are integers to a predicate.
                    let env = match id {
                        Some(id) => self.fns[id].scheme.preds.clone(),
                        None => Vec::new(),
                    };
                    let outer = std::mem::replace(&mut self.adts.proj.env, env);
                    let checked =
                        self.sig_refinements(&f, &params, &ret, (allowed, results), &consts);
                    self.adts.proj.env = outer;
                    checked?;
                }
                Pending::Fields { s, symbol } => {
                    self.pred_owner = None;
                    self.module = s.module;
                    self.fields_now(&s, &symbol)?;
                }
                Pending::Variants { e, symbol } => {
                    self.pred_owner = None;
                    self.module = e.module;
                    self.variants_now(&e, &symbol)?;
                }
                Pending::Alias(symbol) => {
                    self.pred_owner = None;
                    self.alias_now(&symbol)?;
                }
            }
        }
        Ok(())
    }

    /// The free type parameters of function `symbol` (see
    /// `free::free_params`): of its scheme, and if it's a method of an impl
    /// of a trait, the impl's.
    pub(super) fn free_of(&self, symbol: &str) -> Vec<crate::ast::Free> {
        let own = (self.traits.impls.iter())
            .find(|d| d.methods.iter().any(|(_, s)| s == symbol))
            .and_then(|d| Some((&d.self_ty, d.trait_name.as_deref()?)));
        let consumer = |t: &str| targs::consumer(&self.traits, t, &mut Vec::new());
        free::free_params(&self.adts, self.scheme(symbol), own, &consumer)
    }

    /// The free type parameters of every function that has any, by the
    /// node of its name: what the interpreter checks at each call.
    pub(super) fn free_table(&self) -> HashMap<NodeId, Vec<crate::ast::Free>> {
        let own: HashMap<&str, (&Type, &str)> = (self.traits.impls.iter())
            .filter_map(|d| Some((d, d.trait_name.as_deref()?)))
            .flat_map(|(d, tr)| {
                d.methods
                    .iter()
                    .map(move |(_, s)| (s.as_str(), (&d.self_ty, tr)))
            })
            .collect();
        (self.fns.iter())
            .map(|f| {
                let own = own.get(f.symbol.as_str()).copied();
                let consumer = |t: &str| targs::consumer(&self.traits, t, &mut Vec::new());
                (
                    f.name.id,
                    free::free_params(&self.adts, &f.scheme, own, &consumer),
                )
            })
            .filter(|(_, free)| !free.is_empty())
            .collect()
    }

    /// Checks the refinements of `f`'s signature, and keeps them: of its
    /// parameters only if `allowed.0`, and of its result, and what it
    /// ensures, only if `allowed.1` (see `Pending::Sig`).
    fn sig_refinements(
        &mut self,
        f: &FnDecl,
        params: &[Type],
        ret: &Type,
        allowed: (bool, bool),
        consts: &[Ident],
    ) -> Result<(), Error> {
        if !allowed.1
            && let Some(e) = f.ensures.first()
        {
            return Err(Error::ty(
                e.span,
                "`ensures` can't be on a trait's methods yet: an impl's would have to promise no \
                 less",
            ));
        }
        let params_written: Vec<TypeExpr> =
            f.params.iter().map(|p| self.written_out(&p.ty)).collect();
        let ret_written = self.written_out(&f.ret);
        let refined = |t: &&TypeExpr| !t.refinements().is_empty();
        // Where it's written, which may be an alias's use.
        let param = (f.params.iter().zip(&params_written))
            .find(|(_, w)| refined(w))
            .map(|(p, _)| &p.ty);
        let result = refined(&&ret_written).then_some(&f.ret);
        if param.is_none() && result.is_none() && f.ensures.is_empty() {
            return Ok(());
        }
        if let (false, Some(first)) = (allowed.1, param.or(result)) {
            return Err(Error::ty(
                first.span,
                "refinements can't be in a trait's methods yet: an impl's would have to ask no \
                 more than the trait's, and promise no less",
            ));
        }
        if let (false, Some(first)) = (allowed.0, param) {
            return Err(Error::ty(
                first.span,
                "an impl's method of a trait can't have refined parameters: a call through the \
                 trait wouldn't check them (its result can say more, and it can ensure more)",
            ));
        }
        let type_params = match self.pred_owner {
            Some(id) => self.fns[id.0].scheme.params.to_vec(),
            None => Vec::new(),
        };
        let bounds = match self.pred_owner {
            Some(id) => self.fns[id.0].scheme.bounds.clone(),
            None => Vec::new(),
        };
        let mut sig = RefinedSig {
            name: f.name.name.clone(),
            names: f.params.iter().map(|p| p.name.name.clone()).collect(),
            type_params,
            params: Vec::new(),
            ret: None,
            ensures: Vec::new(),
            scoped: call_scoped(params, ret, &bounds),
            framed: Vec::new(),
        };
        // An intrinsic's postconditions are trusted, and may say what's
        // behind a `&mut` whose borrow outlives the call: what it is when
        // the borrow ends, a *prophecy*, which the caller learns of it
        // there, as the place's next read is after.
        if f.intrinsic.is_some() {
            for (i, t) in params.iter().enumerate() {
                if let Type::Ref(_, true, _) = t {
                    sig.scoped[i] = true;
                }
            }
        }
        // Another function's may say so of what its parameters' types and
        // its result's show nothing done through the result can change:
        // the measures free in what it can write (`lend_framed`), which is
        // proved, not trusted.
        if f.intrinsic.is_none()
            && let Some(id) = self.pred_owner
        {
            let symbol = self.fns[id.0].symbol.clone();
            let own = (self.traits.impls.iter())
                .find(|d| d.methods.iter().any(|(_, s)| *s == symbol))
                .and_then(|d| Some((&d.self_ty, d.trait_name.as_deref()?)));
            sig.framed = lend_framed(&self.adts, &self.fns[id.0].scheme, own, &sig.scoped);
        }
        // What it's seen as far as, which the fields its refinements name
        // must be too: its own visibility, or for an impl's method of a
        // trait, everywhere, as an impl is seen wherever its trait and type
        // are named.
        self.pred_reach = match allowed.0 {
            true => match f.vis {
                Vis::Private => None,
                Vis::Pub(_) => Some((
                    format!("`{}` is public, but its refinement names", f.name.name),
                    Level::Package,
                )),
                Vis::Export(_) => Some((
                    format!("`{}` is exported, but its refinement names", f.name.name),
                    Level::Export,
                )),
            },
            false => Some((
                format!(
                    "`{}` is an impl's method of a trait, seen wherever the impl is, but its \
                     refinement names",
                    f.name.name
                ),
                Level::Export,
            )),
        };
        let checked = self.sig_refinements_reached(f, params, ret, consts, &mut sig);
        self.pred_reach = None;
        checked?;
        let symbol = self.sig_owner.0.clone();
        self.refined.insert(symbol, sig);
        Ok(())
    }

    /// The rest of `sig_refinements`, with `pred_reach` set: checks the
    /// refinements of `f`'s parameters, result and `ensures` into `sig`.
    fn sig_refinements_reached(
        &mut self,
        f: &FnDecl,
        params: &[Type],
        ret: &Type,
        consts: &[Ident],
        sig: &mut RefinedSig,
    ) -> Result<(), Error> {
        for (i, p) in f.params.iter().enumerate() {
            let scope = |name: &str| {
                f.params[..i]
                    .iter()
                    .position(|q| q.name.name == name)
                    .map(|j| match f.params[j].mutable {
                        Some(_) => Named::Mut,
                        None => Named::Value(
                            params[j].clone(),
                            PredName::Param(j),
                            f.params[j].name.span,
                        ),
                    })
                    .or_else(|| const_named(consts, name))
            };
            self.pred_again = p.mutable.map(|_| "a `mut` parameter");
            let r = self.refinement(&p.ty, &params[i], &scope);
            self.pred_again = None;
            sig.params.push(r?);
        }
        let scope = |name: &str| {
            f.params
                .iter()
                .position(|q| q.name.name == name)
                .map(|j| match f.params[j].mutable {
                    Some(_) => Named::Mut,
                    None => {
                        Named::Value(params[j].clone(), PredName::Param(j), f.params[j].name.span)
                    }
                })
                .or_else(|| const_named(consts, name))
        };
        // A result's refinement and `ensures` are postconditions: what's
        // behind a call-scoped `&mut` is its value at return, and `old(..)`
        // what it was at the call.
        let post = |what| Some((what, sig.scoped.clone(), sig.framed.clone()));
        self.pred_post = post("the result's refinement");
        let r = self.refinement(&f.ret, ret, &scope);
        self.pred_post = None;
        sig.ret = r?;
        for e in &f.ensures {
            self.pred_post = post("`ensures`");
            let got = self.pred_type(e, &scope);
            self.pred_post = None;
            if got? != Sort::Bool {
                return Err(Error::ty(
                    e.span,
                    format!("expected `bool`, found {}", Sort::Int),
                ));
            }
            sig.ensures.push(Refinement::whole(e.clone()));
        }
        // What's ensured of a lend-framed parameter is true when its
        // borrow ends, which is when the caller next reads it: as of a
        // call-scoped one at return.
        for (i, w) in sig.framed.iter().enumerate() {
            if w.is_some() {
                sig.scoped[i] = true;
            }
        }
        Ok(())
    }

    /// Checks the refinements of the fields of enum `e`'s variants, whose
    /// symbol is `symbol`, and keeps them, once every impl is collected. A
    /// field's refinement names the fields before it in its variant, by
    /// their binders, as a parameter's names the parameters before it:
    /// positional fields have no other names.
    pub(super) fn variant_refinements(&mut self, e: &EnumDecl, symbol: &str) {
        let any = (e.variants.iter().flat_map(|v| &v.fields))
            .any(|f| !self.written_out(f).refinements().is_empty());
        if any {
            self.pending_refined.push(Pending::Variants {
                e: Box::new(e.clone()),
                symbol: symbol.to_string(),
            });
        }
    }

    fn variants_now(&mut self, e: &EnumDecl, symbol: &str) -> Result<(), Error> {
        let types: Vec<Vec<Type>> = match self.adts.get(symbol).map(|d| &d.kind) {
            Some(crate::types::AdtKind::Enum(variants)) => {
                variants.iter().map(|(_, ts)| ts.clone()).collect()
            }
            _ => unreachable!("enum `{symbol}` collected"),
        };
        let consts: Vec<Ident> = (e.generics.types.iter())
            .filter(|p| e.generics.is_const(p))
            .cloned()
            .collect();
        let mut out = RefinedVariants {
            name: e.name.name.clone(),
            variants: Vec::new(),
        };
        for (v, variant) in e.variants.iter().enumerate() {
            let written: Vec<TypeExpr> =
                variant.fields.iter().map(|f| self.written_out(f)).collect();
            // Each field's binder, if its refinement is of the whole of it.
            let binders: Vec<Option<Ident>> = (written.iter())
                .map(|w| w.refinement().map(|(b, _)| b.clone()))
                .collect();
            let mut rv = RefinedVariant {
                name: variant.name.name.clone(),
                refs: Vec::new(),
                named: Vec::new(),
            };
            for (j, f) in variant.fields.iter().enumerate() {
                let scope = |name: &str| {
                    let at = |k: usize| binders[k].as_ref().is_some_and(|b| b.name == name);
                    if let Some(k) = (0..j).rev().find(|&k| at(k)) {
                        let span = binders[k].as_ref().expect("found").span;
                        return Some(Named::Value(types[v][k].clone(), PredName::Field(k), span));
                    }
                    if (j + 1..binders.len()).any(at) {
                        return Some(Named::Not(format!(
                            "a field's refinement can only name the fields before it, and \
                             `{name}` comes after field {j}"
                        )));
                    }
                    const_named(&consts, name)
                };
                self.pred_again = Some("a variant's field");
                self.pred_reach = match e.vis {
                    Vis::Private => None,
                    Vis::Pub(_) => Some((
                        format!(
                            "`{}` is public, but the refinement of its variant's field names",
                            e.name.name
                        ),
                        Level::Package,
                    )),
                    Vis::Export(_) => Some((
                        format!(
                            "`{}` is exported, but the refinement of its variant's field names",
                            e.name.name
                        ),
                        Level::Export,
                    )),
                };
                let r = self.refinement(f, &types[v][j], &scope);
                self.pred_again = None;
                self.pred_reach = None;
                let r = r?;
                let mut named = Vec::new();
                for pred in r.iter().flat_map(Refinement::preds) {
                    each_expr(pred, &mut |e| {
                        if let Some(PredName::Field(k)) = self.pred_names.get(&e.id)
                            && !named.contains(k)
                        {
                            named.push(*k);
                        }
                    });
                }
                rv.refs.push(r);
                rv.named.push(named);
            }
            out.variants.push(rv);
        }
        self.variant_refined.insert(symbol.to_string(), out);
        Ok(())
    }

    /// Checks the refinements in what alias `symbol` stands for, where it's
    /// declared: for every argument, as its parameters have no bounds, and
    /// naming only their binders, its const parameters and the consts in
    /// its module (R7 of docs/67-refinement-aliases.md), and only what's
    /// seen as far as it is (R5). A use checks the copy it gets again
    /// (`Checker::written_out`), as written there: which can't fail where
    /// this didn't, as each is this with types for its parameters.
    fn alias_now(&mut self, symbol: &str) -> Result<(), Error> {
        let def = self.aliases[symbol].clone();
        let base = self.alias_body(&def)?;
        let consts: Vec<Ident> = (def.params.iter().zip(&def.param_spans).zip(&def.kinds))
            .filter(|(_, k)| **k == ArgKind::Length)
            .map(|((p, span), _)| Ident {
                name: p.clone(),
                span: *span,
                id: NodeId(0),
            })
            .collect();
        let scope = |name: &str| const_named(&consts, name);
        let module = std::mem::replace(&mut self.module, def.module);
        let origin = super::written::Origin {
            alias: def.name.clone(),
            module: def.module,
            vis: def.vis,
        };
        self.pred_reach = alias_reach(&origin);
        let r = self.refinement(&def.body, &base, &scope);
        self.pred_reach = None;
        self.module = module;
        r.map(|_| ())
    }

    /// Checks the refinements of the fields of struct `s`, whose symbol
    /// is `symbol`, and keeps them. A field's names the fields before it,
    /// as visible as it is.
    pub(super) fn field_refinements(&mut self, s: &StructDecl, symbol: &str) -> Result<(), Error> {
        if (s.fields.iter()).all(|f| self.written_out(&f.ty).refinements().is_empty()) {
            return Ok(());
        }
        self.pending_refined.push(Pending::Fields {
            s: Box::new(s.clone()),
            symbol: symbol.to_string(),
        });
        Ok(())
    }

    fn fields_now(&mut self, s: &StructDecl, symbol: &str) -> Result<(), Error> {
        let ty = Type::Adt(symbol.to_string(), Vec::new(), Vec::new());
        let types: Vec<Type> = match self.adts.get(symbol).map(|d| &d.kind) {
            Some(crate::types::AdtKind::Struct(fields)) => {
                fields.iter().map(|(_, t)| t.clone()).collect()
            }
            _ => unreachable!("struct `{ty}` collected"),
        };
        let mut fields = RefinedFields {
            name: s.name.name.clone(),
            names: s.fields.iter().map(|f| f.name.name.clone()).collect(),
            refs: Vec::new(),
            named: Vec::new(),
        };
        let consts: Vec<Ident> = (s.generics.types.iter())
            .filter(|p| s.generics.is_const(p))
            .cloned()
            .collect();
        for (i, f) in s.fields.iter().enumerate() {
            let level = |j: usize| s.fields[j].vis.level();
            let scope = |name: &str| {
                let Some(j) = s.fields.iter().position(|g| g.name.name == name) else {
                    return const_named(&consts, name);
                };
                let is = |l: Level| match l {
                    Level::Module => "is private",
                    Level::Package => "is `pub`",
                    Level::Export => "is exported",
                };
                Some(if j > i {
                    Named::Not(format!(
                        "a field's refinement can only name the fields before it, and `{name}` \
                         comes after `{}`",
                        f.name.name
                    ))
                } else if level(j) != level(i) {
                    Named::Not(format!(
                        "the refinement of `{}`, which {}, can't name `{name}`, which {}: a \
                         field's refinement can only name fields exactly as visible as it",
                        f.name.name,
                        is(level(i)),
                        is(level(j)),
                    ))
                } else {
                    Named::Value(types[j].clone(), PredName::Field(j), s.fields[j].name.span)
                })
            };
            self.pred_again = Some("a field");
            let what = format!("field `{}` of `{}` is", f.name.name, s.name.name);
            self.pred_reach = match f.vis {
                Vis::Private => None,
                Vis::Pub(_) => Some((
                    format!("{what} public, but its refinement names"),
                    Level::Package,
                )),
                Vis::Export(_) => Some((
                    format!("{what} exported, but its refinement names"),
                    Level::Export,
                )),
            };
            let r = self.refinement(&f.ty, &types[i], &scope);
            self.pred_again = None;
            self.pred_reach = None;
            let r = r?;
            // A field's own value is its binder, to a refinement of it; one
            // of what's at a position in it (carried) can name the field,
            // the whole value those are in (lemma A6).
            for part in r.iter().flat_map(|r| &r.parts).filter(|p| !p.carried) {
                let mut own = None;
                each_expr(&part.pred, &mut |e| {
                    if let Some(PredName::Field(j)) = self.pred_names.get(&e.id)
                        && *j == i
                    {
                        own = Some(e.span);
                    }
                });
                if let Some(span) = own {
                    let written = self.written_out(&f.ty);
                    let (_, binder, _) = written.refinements().into_iter().next().expect("refined");
                    return Err(Error::ty(
                        span,
                        format!(
                            "a field's refinement names the field's own value by its binder, \
                             `{}`, not `{}`",
                            binder.name, f.name.name
                        ),
                    ));
                }
            }
            let mut named = Vec::new();
            for pred in r.iter().flat_map(Refinement::preds) {
                each_expr(pred, &mut |e| {
                    if let Some(PredName::Field(j)) = self.pred_names.get(&e.id) {
                        named.push(*j);
                    }
                });
            }
            fields.refs.push(r);
            fields.named.push(named);
        }
        self.field_refined.insert(symbol.to_string(), fields);
        Ok(())
    }

    /// Checks the refinement of a `let` of `pat`, of type `t` (the type
    /// without it), written `ann`, in the scope of the locals so far.
    pub(super) fn let_refinement(
        &mut self,
        pat: &Pattern,
        ann: &TypeExpr,
        t: &Type,
        init: bool,
    ) -> Result<(), Error> {
        if self.written_out(ann).refinements().is_empty() {
            return Ok(());
        }
        if !matches!(pat.kind, PatternKind::Bind(..)) {
            return Err(Error::ty(
                ann.span,
                "a refinement can only be the type of a `let` of one variable",
            ));
        }
        if !init {
            return Err(Error::ty(
                ann.span,
                "a `let` with a refinement needs a value, which must have it",
            ));
        }
        let locals: Vec<(String, Type, bool, NodeId, Span)> = self.locals[self.barrier..]
            .iter()
            .map(|l| (l.name.clone(), l.ty.clone(), l.mutable, l.id(), l.def()))
            .collect();
        let scope = |name: &str| {
            locals
                .iter()
                .rev()
                .find(|l| l.0 == name)
                .map(|l| match l.2 {
                    true => Named::Mut,
                    false => Named::Value(l.1.clone(), PredName::Local(l.3), l.4),
                })
        };
        if let PatternKind::Bind(_, true) = pat.kind {
            self.pred_again = Some("a `mut` variable");
        }
        self.pred_owner = self.current;
        let r = self.refinement(ann, t, &scope);
        self.pred_again = None;
        let r = r?.expect("refined");
        // What it names, it uses.
        let named: HashSet<NodeId> = (self.pred_names.values())
            .filter_map(|n| match n {
                PredName::Local(id) => Some(*id),
                _ => None,
            })
            .collect();
        for l in &mut self.locals[self.barrier..] {
            if named.contains(&l.id()) {
                l.used = true;
            }
        }
        let bind = pat.id;
        self.refined_lets.insert(bind, r);
        Ok(())
    }

    /// What the predicate being checked is, for messages: "a refinement",
    /// or "an invariant".
    fn pred_a(&self) -> &'static str {
        match self.pred_invariant {
            true => "an invariant",
            false => "a refinement",
        }
    }

    /// The same, after "this".
    fn pred_noun(&self) -> &'static str {
        match self.pred_invariant {
            true => "invariant",
            false => "refinement",
        }
    }

    /// Checks loop invariant `p`, in the scope of the locals at the loop:
    /// a predicate, as a refinement's, but of `mut` variables too, as it's
    /// of their values each time the loop is about to go round, where the
    /// walk reads them (docs/54-loop-invariants.md). Measures behind a
    /// `&mut` too, for the same reason.
    pub(super) fn loop_invariant(&mut self, p: &Expr) -> Result<(), Error> {
        let locals: Vec<(String, Type, NodeId, Span)> = self.locals[self.barrier..]
            .iter()
            .map(|l| (l.name.clone(), l.ty.clone(), l.id(), l.def()))
            .collect();
        let scope = |name: &str| {
            (locals.iter().rev().find(|l| l.0 == name))
                .map(|l| Named::Value(l.1.clone(), PredName::Local(l.2), l.3))
        };
        self.pred_owner = self.current;
        self.pred_invariant = true;
        let got = self.pred_type(p, &scope);
        self.pred_invariant = false;
        let got = got?;
        if got != Sort::Bool {
            return Err(Error::ty(p.span, format!("expected `bool`, found {got}")));
        }
        // What it names, it uses.
        let mut named = HashSet::new();
        each_expr(p, &mut |e| {
            if let Some(PredName::Local(id)) = self.pred_names.get(&e.id) {
                named.insert(*id);
            }
        });
        for l in &mut self.locals[self.barrier..] {
            if named.contains(&l.id()) {
                l.used = true;
            }
        }
        Ok(())
    }

    /// The refinements written in `t`, whose base type is `base`, if there
    /// are any, checked: each of the whole value or of a type argument of
    /// an enum (`refined_arg`), with a predicate that's a `bool` in the
    /// logic, naming what `scope` says.
    fn refinement(
        &mut self,
        t: &TypeExpr,
        base: &Type,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Result<Option<Refinement>, Error> {
        // With the aliases in it written out, which it says the refinements
        // of (`Checker::written_out`).
        let t = &self.written_out(t);
        let written = t.refinements();
        if written.is_empty() {
            return Ok(None);
        }
        let base = self.solved(base);
        let mut parts = Vec::new();
        for (written_path, binder, pred) in written {
            let (of, ways, path, carried) = self.refined_arg(t, &base, &written_path)?;
            let copy = self.copied.contains_key(&pred.id);
            let notes = std::mem::replace(&mut self.copy_notes, copy);
            let kind = super::NoteKind::Var(binder.name.clone());
            self.note(binder.span, kind, Some(of.clone()), Some(binder.span));
            // One written out of an alias names what it does where the
            // alias is declared: its binder, and consts there (R7).
            let origin = self.alias_preds.get(&pred.id).cloned();
            let scope = |name: &str| match name == binder.name {
                true => Some(Named::Value(of.clone(), PredName::Binder, binder.span)),
                false if origin.is_some() => None,
                false => scope(name),
            };
            let outer = origin.as_ref().map(|o| {
                let reach = alias_reach(o);
                (
                    std::mem::replace(&mut self.module, o.module),
                    std::mem::replace(&mut self.pred_reach, reach),
                )
            });
            let got = self.pred_type(pred, &scope);
            if let Some((module, reach)) = outer {
                self.module = module;
                self.pred_reach = reach;
            }
            self.copy_notes = notes;
            let got = got?;
            if got != Sort::Bool {
                return Err(Error::ty(
                    pred.span,
                    format!("expected `bool`, found {got}"),
                ));
            }
            // A carried one isn't of any part a way gets to: the
            // interpreter checks it of every value there (`Measures::reach`).
            if !path.is_empty() {
                let named = match carried {
                    false => self.named_ways(&base, &ways),
                    true => Vec::new(),
                };
                self.info.measures.ways.insert(pred.id, named);
            }
            if carried {
                let reach = targs::reach(&self.adts, &base, &path);
                self.info.measures.reach.insert(pred.id, reach);
            }
            parts.push(Part {
                pred: pred.clone(),
                ways,
                path,
                carried,
            });
        }
        Ok(Some(Refinement {
            parts,
            base: Some(base),
        }))
    }

    /// `ways` into a value of type `t`, with each variant by name, for the
    /// interpreter.
    fn named_ways(&self, t: &Type, ways: &[Way]) -> Vec<Vec<(String, usize)>> {
        let mut out = Vec::new();
        for way in ways {
            let mut ty = t.clone();
            let mut named = Vec::new();
            for &(v, f) in way {
                let Type::Adt(symbol, ..) = &ty else {
                    unreachable!("a way is through enums")
                };
                let crate::types::AdtKind::Enum(variants) = &self.adts[symbol].kind else {
                    unreachable!("a way is through enums")
                };
                named.push((variants[v].0.clone(), f));
                ty = self.solved(&crate::types::variant_fields(&self.adts, &ty, v)[f]);
            }
            out.push(named);
        }
        out
    }

    /// What the refinement at `path` in `t`, a type written whose base is
    /// `base`, is of: the type there; the ways to each part of a value of
    /// `base` that has it, if each step of `path` is a type argument of an
    /// enum whose variants have the parameter it's for only as whole fields
    /// (so the parts it's of are fields, which every value of the enum is
    /// made with and every pattern reads, and the refinement is what each of
    /// them is, whatever the enum's methods do: lemma E2 of
    /// docs/53-refined-type-arguments.md); the position (`targs::Pos`); and
    /// whether it's *carried*: somewhere else, a struct's type argument, a
    /// tuple's element, what a reference or a box points to, a slice's or an
    /// array's elements. Then it's what every value there is, kept by
    /// parametricity (lemma A2 of docs/60-refined-type-arguments.md).
    fn refined_arg(
        &self,
        t: &TypeExpr,
        base: &Type,
        path: &[usize],
    ) -> Result<(Type, Vec<Way>, Vec<usize>, bool), Error> {
        fn mentions(t: &Type, p: &str) -> bool {
            matches!(t, Type::Param(q, _) if q == p) || t.parts().any(|u| mentions(u, p))
        }
        let mut ways = vec![Vec::new()];
        let mut carried = false;
        let mut pos = Vec::new();
        let mut ty = base.clone();
        let mut written = t.unrefined();
        for &k in path {
            let arg = written.refinable()[k];
            let not = |why: String| Error::ty(arg.span, why);
            // Which of the type's positions it is: a named type's lifetime
            // arguments are written among its type arguments.
            let (j, next) = match (&written.kind, &ty) {
                (TypeExprKind::Named(_, args), Type::Adt(_, _, targs)) => {
                    let j = (args[..k].iter())
                        .filter(|a| !matches!(a.kind, TypeExprKind::Lifetime(_)))
                        .count();
                    match targs.get(j) {
                        Some(next) => (j, next.clone()),
                        None => return Err(not(format!("`{ty}` has no type argument here"))),
                    }
                }
                (TypeExprKind::Named(..), Type::Box(inner))
                | (TypeExprKind::Ref(..), Type::Ref(_, _, inner))
                | (TypeExprKind::Slice(_), Type::Slice(inner))
                | (TypeExprKind::Array(..), Type::Array(inner, _))
                    if k == 0 =>
                {
                    (0, (**inner).clone())
                }
                (TypeExprKind::Tuple(_), Type::Tuple(ts)) if k < ts.len() => (k, ts[k].clone()),
                _ => {
                    return Err(not(format!(
                        "a refinement can't be a type argument of `{ty}`, which isn't a struct, \
                         an enum, a tuple, a reference, a box, a slice or an array"
                    )));
                }
            };
            pos.push(j);
            // Through an enum whose variants have the parameter only as
            // whole fields, it's still of parts: of those fields.
            let enum_steps = match &ty {
                Type::Adt(symbol, ..) if !carried => match self.adts.get(symbol) {
                    Some(def) => match &def.kind {
                        crate::types::AdtKind::Enum(variants) => {
                            let param = def.params.as_slice()[j].clone();
                            let mut steps = Vec::new();
                            let mut whole = true;
                            for (v, (_, fields)) in variants.iter().enumerate() {
                                for (f, ft) in fields.iter().enumerate() {
                                    match ft {
                                        Type::Param(p, _) if *p == param => steps.push((v, f)),
                                        ft if mentions(ft, &param) => whole = false,
                                        _ => {}
                                    }
                                }
                            }
                            whole.then_some(steps)
                        }
                        crate::types::AdtKind::Struct(_) => None,
                    },
                    None => None,
                },
                _ => None,
            };
            match enum_steps {
                Some(steps) => {
                    ways = (ways.iter())
                        .flat_map(|w| {
                            steps.iter().map(move |s| {
                                let mut w = w.clone();
                                w.push(*s);
                                w
                            })
                        })
                        .collect();
                }
                None => {
                    carried = true;
                    ways.clear();
                }
            }
            ty = self.solved(&next);
            written = arg.unrefined();
        }
        Ok((ty, ways, pos, carried))
    }

    /// What name `name`, at `e` in a predicate, names in `scope`: but if
    /// it's what an alias's const parameter was given, written out of the
    /// alias (`Checker::written_out`), what that named where it was given.
    fn pred_name(
        &self,
        e: &Expr,
        name: &str,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Option<Named> {
        match self.pred_fixed.get(&e.id) {
            Some(super::written::Fixed::Param(m, at)) => Some(Named::Value(
                Type::Int(crate::types::IntTy::Usize),
                PredName::Const(m.clone()),
                *at,
            )),
            // A const, which `pred_const` finds by its symbol.
            Some(super::written::Fixed::Item(_)) => None,
            None => scope(name),
        }
    }

    /// Checks `r`, what a predicate or a measure divides by with `op`, is
    /// a constant, but 0: a quotient by one is linear, but not by an
    /// unknown (docs/72-division.md).
    fn divisor(what: &str, op: &BinOp, r: &Expr) -> Result<(), Error> {
        match constant(r) {
            Some(0) => Err(Error::ty(r.span, format!("{what} can't divide by zero"))),
            Some(_) => Ok(()),
            None => Err(Error::ty(
                r.span,
                format!(
                    "{what} can use `{op}` only by a constant: `{}` isn't one",
                    crate::pretty::expr_text(r)
                ),
            )),
        }
    }

    /// The sort of predicate (part) `e`, which must be in the logic:
    /// linear arithmetic on integers, of any integer types, which are all
    /// the integers in it, ℤ, compared, and `bool`s.
    fn pred_type(
        &mut self,
        e: &Expr,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Result<Sort, Error> {
        let want = |cx: &mut Checker, e: &Expr, t: Sort| -> Result<(), Error> {
            let got = cx.pred_type(e, scope)?;
            if got != t {
                return Err(Error::ty(e.span, format!("expected {t}, found {got}")));
            }
            Ok(())
        };
        match &e.kind {
            ExprKind::Int(..) => Ok(Sort::Int),
            ExprKind::Bool(_) => Ok(Sort::Bool),
            ExprKind::Var(name, targs) if targs.is_empty() => {
                match self.pred_name(e, name, scope) {
                    Some(Named::Value(t, what, def)) => {
                        let sort = match self.scalar_sort(&t) {
                            Some(sort) => sort,
                            None => return Err(self.not_scalar(e, name, &self.solved(&t))),
                        };
                        if let PredName::Const(c) = &what {
                            self.info.consts.insert(e.id, c.clone());
                        }
                        self.pred_names.insert(e.id, what.clone());
                        let kind = match what {
                            PredName::Field(_) => super::NoteKind::Field(name.clone()),
                            PredName::Const(_) => super::NoteKind::ConstParam(name.clone()),
                            _ => super::NoteKind::Var(name.clone()),
                        };
                        self.note(e.span, kind, Some(self.solved(&t)), Some(def));
                        Ok(sort)
                    }
                    Some(Named::Not(why)) => Err(Error::ty(e.span, why)),
                    Some(Named::Mut) => Err(Error::ty(
                        e.span,
                        format!(
                            "{} can't name `{name}`, which is `mut`: it would be unclear which of \
                         its values it meant",
                            self.pred_a()
                        ),
                    )),
                    // A const, of an integer type or `bool`.
                    None if let Some((symbol, t, def)) = self.pred_const(name, e.id) => {
                        let sort = match t {
                            Type::Int(_) | Type::Char => Sort::Int,
                            Type::Bool => Sort::Bool,
                            t => {
                                return Err(Error::ty(
                                    e.span,
                                    format!(
                                        "{} can't name `{name}`, a const of type `{}`: only integers \
                                     and `bool`s are in its logic",
                                        self.pred_a(),
                                        t.erased()
                                    ),
                                ));
                            }
                        };
                        self.info.pred_consts.insert(e.id, symbol.clone());
                        self.pred_names.insert(e.id, PredName::Item(symbol));
                        let kind = super::NoteKind::Const(name.clone());
                        self.note(e.span, kind, Some(t), Some(def));
                        Ok(sort)
                    }
                    None => Err(Error::ty(
                        e.span,
                        format!("cannot find value `{name}` in this {}", self.pred_noun()),
                    )),
                }
            }
            ExprKind::Unary(UnOp::Neg, x) => want(self, x, Sort::Int).map(|()| Sort::Int),
            ExprKind::Unary(UnOp::Not, x) => want(self, x, Sort::Bool).map(|()| Sort::Bool),
            ExprKind::Binary(op, l, r) => match op {
                BinOp::Add | BinOp::Sub => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    Ok(Sort::Int)
                }
                BinOp::Mul => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    let d = degree(e);
                    if d > 2 {
                        return Err(Error::ty(
                            e.span,
                            format!(
                                "{} can multiply at most two unknowns: `{}` multiplies {}",
                                self.pred_a(),
                                crate::pretty::expr_text(e),
                                match d {
                                    3 => "three".to_string(),
                                    4 => "four".to_string(),
                                    d => d.to_string(),
                                }
                            ),
                        ));
                    }
                    Ok(Sort::Int)
                }
                // By a constant, linear (docs/72-division.md).
                BinOp::Div | BinOp::Rem => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    Self::divisor(self.pred_a(), op, r)?;
                    Ok(Sort::Int)
                }
                BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                    Err(Error::ty(
                        e.span,
                        format!(
                            "{} can't use `{op}` yet: only `+`, `-`, `*`, and `/` and `%` by a \
                             constant",
                            self.pred_a()
                        ),
                    ))
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    want(self, l, Sort::Int)?;
                    want(self, r, Sort::Int)?;
                    Ok(Sort::Bool)
                }
                BinOp::Eq | BinOp::Ne => {
                    let t = self.pred_type(l, scope)?;
                    want(self, r, t)?;
                    Ok(Sort::Bool)
                }
                BinOp::And | BinOp::Or => {
                    want(self, l, Sort::Bool)?;
                    want(self, r, Sort::Bool)?;
                    Ok(Sort::Bool)
                }
            },
            ExprKind::Call(callee, args)
                if matches!(&callee.kind, ExprKind::Var(n, t) if n == "old" && t.is_empty())
                    && args.len() == 1 =>
            {
                if self.pred_post.is_none() {
                    return Err(Error::ty(
                        e.span,
                        "`old(..)` can only be in a postcondition: `ensures`, or a result's \
                         refinement",
                    ));
                }
                if self.pred_in_old {
                    return Err(Error::ty(e.span, "`old(..)` inside `old(..)`"));
                }
                self.pred_in_old = true;
                let got = self.pred_type(&args[0], scope);
                self.pred_in_old = false;
                got
            }
            ExprKind::Field(..) => {
                let at = self.pred_place(e, scope)?;
                let t = self.solved(&at.ty);
                let sort = match self.scalar_sort(&t) {
                    Some(sort) => sort,
                    None => return Err(self.not_scalar(e, &crate::pretty::expr_text(e), &t)),
                };
                self.pred_mutable(e.span, &at, "a field", None)?;
                if at.through_mut {
                    self.info.measures.behind_mut.insert(e.id);
                }
                Ok(sort)
            }
            ExprKind::MethodCall(x, m, targs, args) if targs.is_empty() && args.is_empty() => {
                let mut at = self.pred_place(x, scope)?;
                let (symbol, derefs, through_mut) = self.find_measure(&at.ty, m)?;
                at.through_mut |= through_mut;
                self.pred_mutable(x.span, &at, "a measure", Some(&symbol))?;
                if at.through_mut {
                    self.info.measures.behind_mut.insert(e.id);
                }
                let ret = self.measures[&symbol].ret.clone();
                if let Some(i) = self.fns.iter().position(|f| f.symbol == symbol) {
                    let (ty, def) = (self.fns[i].scheme.ty.clone(), self.fns[i].name.span);
                    let kind = super::NoteKind::Fn(m.name.clone());
                    self.note(m.span, kind, Some(ty), Some(def));
                    // What names it uses it, or else the struct does.
                    match self.pred_owner {
                        Some(owner) => {
                            self.uses[owner.0].fns.insert(super::scope::FnId(i));
                        }
                        None => self.kept_measures.push(super::scope::FnId(i)),
                    }
                }
                let call = (symbol.clone(), derefs, Some(false));
                self.info.measures.calls.insert(e.id, call);
                self.pred_measures.insert(e.id, (symbol, derefs));
                Ok(match ret {
                    Type::Bool => Sort::Bool,
                    _ => Sort::Int,
                })
            }
            _ => Err(Error::ty(
                e.span,
                format!(
                    "{} can only use literals, names, fields, measures, `+`, `-`, `*`, `/` and \
                     `%` by a constant, comparisons, `&&`, `||` and `!`",
                    self.pred_a()
                ),
            )),
        }
    }

    /// The sort of a value of type `t` in the logic, if it's in it: an
    /// integer, of any integer type, one not known yet or a `T: Int`'s, or
    /// a `bool`.
    fn scalar_sort(&self, t: &Type) -> Option<Sort> {
        match self.solved(t) {
            _ if self.int_var(t) => Some(Sort::Int),
            Type::Int(_) => Some(Sort::Int),
            t @ Type::Param(_, _)
                if (self.traits)
                    .implements(&t, INT, &self.adts.proj.env, &self.adts.proj.local)
                    .is_ok() =>
            {
                Some(Sort::Int)
            }
            Type::Bool => Some(Sort::Bool),
            _ => None,
        }
    }

    /// The error for `e`, written `text`, of type `t`, which isn't an
    /// integer or a `bool`, named whole in a predicate.
    fn not_scalar(&self, e: &Expr, text: &str, t: &Type) -> Error {
        let mut peeled = t;
        while let Type::Ref(_, _, inner) | Type::Box(inner) = peeled {
            peeled = inner;
        }
        let by = match struct_fields(&self.adts, peeled) {
            Some(_) => format!(
                "by its fields and measures: `{text}.f`, or `{text}.m()`, where `m` is a \
                 `#[measure]`"
            ),
            None => format!("by its measures: `{text}.m()`, where `m` is a `#[measure]`"),
        };
        Error::ty(
            e.span,
            format!(
                "{} can only name `{text}`, of type `{t}`, {by}",
                self.pred_a()
            ),
        )
    }

    /// What a path in a predicate names: a name, or a field of what one
    /// is or points to, at any depth, `s.head` (see `PredPlace`).
    fn pred_place(
        &mut self,
        e: &Expr,
        scope: &dyn Fn(&str) -> Option<Named>,
    ) -> Result<PredPlace, Error> {
        match &e.kind {
            ExprKind::Var(name, targs) if targs.is_empty() => {
                match self.pred_name(e, name, scope) {
                    Some(Named::Value(t, what, def)) => {
                        self.pred_names.insert(e.id, what.clone());
                        let kind = match what {
                            PredName::Field(_) => super::NoteKind::Field(name.clone()),
                            PredName::Const(_) => super::NoteKind::ConstParam(name.clone()),
                            _ => super::NoteKind::Var(name.clone()),
                        };
                        self.note(e.span, kind, Some(self.solved(&t)), Some(def));
                        Ok(PredPlace {
                            ty: t,
                            named: what,
                            name: name.clone(),
                            through_mut: false,
                        })
                    }
                    // A name that isn't a value: the error it is alone; or a
                    // const, an integer or a `bool`, which has neither.
                    _ => {
                        self.pred_type(e, scope)?;
                        let t = match self.pred_const(name, e.id) {
                            Some((_, t, _)) => t.erased().to_string(),
                            None => "an integer or `bool`".to_string(),
                        };
                        Err(Error::ty(
                            e.span,
                            format!(
                                "`{name}` is a const of type `{t}`, which has no fields or measures"
                            ),
                        ))
                    }
                }
            }
            ExprKind::Field(x, field) => {
                let mut at = self.pred_place(x, scope)?;
                let mut t = self.solved(&at.ty);
                let mut derefs = 0;
                loop {
                    t = match t {
                        Type::Ref(_, mutable, inner) => {
                            at.through_mut |= mutable;
                            self.solved(&inner)
                        }
                        Type::Box(inner) => self.solved(&inner),
                        _ => break,
                    };
                    derefs += 1;
                }
                let fields = struct_fields(&self.adts, &t).unwrap_or_default();
                let Some((i, (_, ft))) =
                    (fields.into_iter().enumerate()).find(|(_, (f, _))| *f == field.name)
                else {
                    return Err(Error::ty(
                        field.span,
                        format!("`{t}` has no field `{}`", field.name),
                    ));
                };
                // A tuple's fields are all exported (lemma T1 of
                // docs/58-tuples.md): seen wherever it is.
                if let Type::Adt(symbol, ..) = &t {
                    self.field_visible(symbol, field)?;
                    self.field_reach(symbol, field)?;
                    self.note_field(&symbol.clone(), field, &ft);
                }
                self.pred_fields.insert(e.id, (derefs, i));
                at.ty = ft;
                Ok(at)
            }
            _ => Err(Error::ty(
                e.span,
                format!(
                    "{} can only call a measure of a name, or of a field of one: `s.len()`, \
                     `s.head.len()`",
                    self.pred_a()
                ),
            )),
        }
    }

    /// Fails if field `field` of struct `symbol`, named in a refinement of
    /// a signature or a field, isn't seen as far as that is (`pred_reach`):
    /// what callers prove can't hang on what they can't see.
    fn field_reach(&self, symbol: &str, field: &Ident) -> Result<(), Error> {
        let Some((what, want)) = &self.pred_reach else {
            return Ok(());
        };
        let Some(def) = self.types.iter().find(|d| d.symbol == symbol) else {
            return Ok(());
        };
        let Some(&(_, level)) = def.fields.iter().find(|(name, _)| *name == field.name) else {
            return Ok(());
        };
        if level >= *want {
            return Ok(());
        }
        let why = match level {
            Level::Module => format!("which is private to {}", self.modules.describe(def.module)),
            _ => "which isn't exported".to_string(),
        };
        Err(Error::ty(
            field.span,
            format!(
                "{what} field `{}` of `{}`, {why}",
                field.name,
                super::source_name(symbol)
            ),
        ))
    }

    /// Fails if what `at`, a path of a predicate at `span` that `noun`
    /// ("a field", "a measure") is of, is behind a `&mut`, where what's
    /// there may have changed by when the predicate is about: when it's
    /// checked again (`pred_again`), or at return in a postcondition, but
    /// of the binder or a call-scoped parameter, or a lend-framed one if
    /// it's a field (a scalar, which can't be of a free type parameter) or
    /// `measure` is free in what the result can write
    /// (`Checker::measure_free_in`).
    fn pred_mutable(
        &self,
        span: Span,
        at: &PredPlace,
        noun: &str,
        measure: Option<&str>,
    ) -> Result<(), Error> {
        if !at.through_mut {
            return Ok(());
        }
        let name = &at.name;
        if let Some(what) = self.pred_again {
            return Err(Error::ty(
                span,
                format!(
                    "the refinement of {what} can't name {noun} of what's behind `&mut` \
                     `{name}`, which can change"
                ),
            ));
        }
        let framed = |j: usize| match (self.pred_post.as_ref(), measure) {
            (Some((_, _, framed)), m) => match (framed.get(j), m) {
                (Some(Some(_)), None) => true,
                (Some(Some(w)), Some(m)) => self.measure_free_in(m, &at.ty, w),
                _ => false,
            },
            _ => false,
        };
        if !self.pred_in_old
            && let Some((what, scoped, _)) = &self.pred_post
            && !matches!(at.named, PredName::Param(j) if scoped[j] || framed(j))
            && !matches!(at.named, PredName::Binder)
        {
            return Err(Error::ty(
                span,
                format!(
                    "{what} can't name `{name}` at return: the borrow it's lent can outlive the \
                     call, as the lifetime of `{name}` is in the signature elsewhere; write \
                     `old(..)`"
                ),
            ));
        }
        Ok(())
    }

    /// Is measure `symbol`, of a value of type `ty` (or what it points
    /// to), free in type parameters `w`: is each of its own type
    /// parameters that `ty` has one of `w` in free in its scheme? Then its
    /// value is the same whatever values of `w`'s types are in the value
    /// (lemma L2 of docs/59-lends.md): parametricity, as the measure can
    /// make, compare or look into none.
    fn measure_free_in(&self, symbol: &str, ty: &Type, w: &[String]) -> bool {
        let Some(f) = self.fns.iter().find(|f| f.symbol == symbol) else {
            return false;
        };
        let Type::Fn(sig) = &f.scheme.ty else {
            return false;
        };
        let Some(Type::Ref(_, false, pat)) = sig.params.first() else {
            return false;
        };
        let mut t = self.solved(ty);
        while let Type::Ref(_, _, inner) | Type::Box(inner) = t {
            t = self.solved(&inner);
        }
        let params: Vec<String> = (f.scheme.params.as_slice().iter())
            .filter(|p| f.scheme.params.is_type(p))
            .cloned()
            .collect();
        let mut found = HashMap::new();
        if !crate::traits::matches(pat, &t, &params, &mut found) {
            return false;
        }
        found.iter().all(|(p, u)| {
            !w.iter().any(|x| free::mentions(u, x)) || free::is_free(&f.scheme, p, None)
        })
    }

    /// The measure `m` of a value of type `ty`, or of what it points to,
    /// through however many `&`s, `&mut`s and boxes: its symbol, how many
    /// `*`s it's through, and whether one of them is a `&mut`'s, so that
    /// what it's of may change. A slice's or a string's measure is of the
    /// reference to it, which says how long it is, so the last `*` before
    /// one doesn't count: what's written through a `&mut [T]` can't change
    /// that.
    fn find_measure(&self, ty: &Type, m: &Ident) -> Result<(String, usize, bool), Error> {
        let mut t = self.solved(ty);
        let mut muts: Vec<bool> = Vec::new();
        loop {
            if let Some(symbol) = self.inherent_named(&t, &m.name) {
                let Some(measure) = self.measures.get(&symbol) else {
                    break;
                };
                let counted = match measure.unsized_self {
                    true => &muts[..muts.len().saturating_sub(1)],
                    false => &muts[..],
                };
                let through_mut = counted.iter().any(|m| *m);
                let fn_def = self.fns.iter().find(|f| f.symbol == symbol);
                if let Some(f) = fn_def
                    && !self.modules.visible(f.module, f.level, self.module)
                {
                    return Err(Error::ty(
                        m.span,
                        format!(
                            "measure `{}` of `{t}` {}",
                            m.name,
                            self.modules.hidden_by(f.module, f.level)
                        ),
                    ));
                }
                return Ok((symbol, muts.len(), through_mut));
            }
            t = match t {
                Type::Ref(_, mutable, inner) => {
                    muts.push(mutable);
                    self.solved(&inner)
                }
                Type::Box(inner) => {
                    muts.push(false);
                    self.solved(&inner)
                }
                _ => break,
            };
        }
        Err(Error::ty(
            m.span,
            format!(
                "`{}` isn't a measure: {} can only call methods marked `#[measure]`",
                m.name,
                self.pred_a()
            ),
        ))
    }

    /// Checks the signature of measure `f`, whose parameters and result
    /// have types `params` and `ret`, and keeps it.
    fn measure_signature(&mut self, f: &FnDecl, params: &[Type], ret: &Type) -> Result<(), Error> {
        let of = match (f.receiver(), params) {
            (Some(_), [Type::Ref(_, false, of)]) => self.solved(of),
            _ => {
                return Err(Error::ty(f.parens, "a measure takes only `&self`"));
            }
        };
        if !f.generics.is_empty() {
            return Err(Error::ty(
                f.name.span,
                "a measure has no type parameters of its own",
            ));
        }
        if let Some(e) = &f.effects {
            return Err(Error::ty(
                e.span,
                "a measure can't have effects: what it gives is all it does",
            ));
        }
        let ret = self.solved(ret);
        // An integer of a type parameter's is one of some integer type's.
        let int_param = matches!(ret, Type::Param(_, _))
            && (self.traits)
                .implements(&ret, INT, &self.adts.proj.env, &self.adts.proj.local)
                .is_ok();
        if !matches!(ret, Type::Int(_) | Type::Bool) && !int_param {
            return Err(Error::ty(
                f.ret.span,
                format!("a measure gives an integer or a `bool`, not `{ret}`"),
            ));
        }
        let body = f.intrinsic.is_none().then(|| f.body.clone());
        let symbol = self.sig_owner.0.clone();
        self.info.measures.bodies.insert(symbol, body);
        let measure = Measure {
            name: f.name.name.clone(),
            ret,
            module: f.module,
            body: f.intrinsic.is_none().then(|| f.body.clone()),
            unsized_self: matches!(of, Type::Slice(_) | Type::Str),
            of_array: matches!(of, Type::Array(..)) && f.intrinsic.is_some(),
            uses: Vec::new(),
            reads_fields: true,
        };
        self.measures.insert(self.sig_owner.0.clone(), measure);
        Ok(())
    }

    /// Checks the body of measure `symbol`, `body`, is a term of the logic:
    /// `self`'s fields, measures, literals and what a predicate can do with
    /// them; and that it isn't defined by itself, through other measures.
    pub(super) fn measure_body(&mut self, symbol: &str, body: &Expr) -> Result<(), Error> {
        let mut uses = Vec::new();
        let term = match &body.kind {
            ExprKind::Block(stmts, Some(tail)) if stmts.is_empty() => &**tail,
            _ => body,
        };
        self.measure_term(term, false, &mut uses)?;
        uses.dedup();
        let mut reads_fields = false;
        each_expr(term, &mut |e| {
            reads_fields |= matches!(e.kind, ExprKind::Field(..))
        });
        let m = self.measures.get_mut(symbol).expect("a measure");
        m.uses = uses;
        m.reads_fields = reads_fields;
        // Measures are checked one at a time, so a cycle is found once the
        // last of it is.
        let mut path = vec![symbol.to_string()];
        if let Some(through) = self.measure_cycle(symbol, &mut path) {
            let name = |s: &str| self.measures[s].name.clone();
            return Err(Error::ty(
                body.span,
                format!(
                    "measure `{}` is defined by itself, through `{}`",
                    name(symbol),
                    name(&through)
                ),
            ));
        }
        Ok(())
    }

    /// A measure on the way from the last of `path` back to `start`, if
    /// there's a way.
    fn measure_cycle(&self, start: &str, path: &mut Vec<String>) -> Option<String> {
        let last = path.last().expect("not empty").clone();
        for u in &self.measures[&last].uses {
            if u == start {
                return Some(path.get(1).unwrap_or(&last).clone());
            }
            if path.contains(u) {
                continue;
            }
            path.push(u.clone());
            if let Some(t) = self.measure_cycle(start, path) {
                return Some(t);
            }
            path.pop();
        }
        None
    }

    /// Checks `e`, in a measure's body, is a term of the logic, keeping
    /// the measures it uses in `uses`. Only a `receiver` may be other than
    /// an integer or a `bool`: `self`, or a field of it, whose measure
    /// it's of.
    fn measure_term(
        &mut self,
        e: &Expr,
        receiver: bool,
        uses: &mut Vec<String>,
    ) -> Result<(), Error> {
        let bad = || {
            Error::ty(
                e.span,
                "a measure's body can only use `self`'s fields, measures, literals, `+`, `-`, `*`, \
                 `/` and `%` by a constant, comparisons, `&&`, `||` and `!`",
            )
        };
        let ty = self.typed.exprs.get(&e.id).map(|t| self.solved(t));
        let scalar = match &ty {
            Some(Type::Int(_) | Type::Bool) => true,
            // One of some integer type, `T: Int`.
            Some(t @ Type::Param(_, _)) => {
                (self
                    .traits
                    .implements(t, INT, &self.adts.proj.env, &self.adts.proj.local))
                .is_ok()
            }
            _ => false,
        };
        if !receiver && !scalar {
            return Err(bad());
        }
        match &e.kind {
            ExprKind::Int(..) | ExprKind::Bool(_) => Ok(()),
            ExprKind::Var(name, _) if name == "self" && receiver => Ok(()),
            ExprKind::Field(x, _) => self.measure_term(x, true, uses),
            ExprKind::MethodCall(x, m, targs, args) if targs.is_empty() && args.is_empty() => {
                let symbol = match self.typed.methods.get(&e.id) {
                    Some((Res::Fn(s, _), ..)) if self.measures.contains_key(s) => s.clone(),
                    _ => {
                        return Err(Error::ty(
                            m.span,
                            format!(
                                "`{}` isn't a measure: a refinement can only call methods marked \
                                 `#[measure]`",
                                m.name
                            ),
                        ));
                    }
                };
                let (_, derefs, by) = self.typed.methods[&e.id].clone();
                self.info
                    .measures
                    .calls
                    .insert(e.id, (symbol.clone(), derefs, by));
                uses.push(symbol);
                self.measure_term(x, true, uses)
            }
            ExprKind::Unary(UnOp::Neg | UnOp::Not, x) => self.measure_term(x, false, uses),
            // Which variant `self`, or a field of it, is (docs/59-lends.md):
            // its tag, a function of the value, as a field is.
            ExprKind::Match(x, arms) => {
                self.measure_term(x, true, uses)?;
                for a in arms {
                    if variant_test(&a.pat).is_none() {
                        return Err(Error::ty(
                            a.pat.span,
                            "a measure's `match` can only say which variant a value is: its \
                             patterns can't bind or test what's in one",
                        ));
                    }
                    self.measure_term(&a.body, false, uses)?;
                }
                Ok(())
            }
            ExprKind::Binary(op @ (BinOp::Div | BinOp::Rem), l, r) => {
                Self::divisor("a measure's body", op, r)?;
                self.measure_term(l, false, uses)?;
                self.measure_term(r, false, uses)
            }
            ExprKind::Binary(op, l, r) => {
                if matches!(
                    op,
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr
                ) || (*op == BinOp::Mul && degree(e) > 2)
                    || self.is_ord_call(e.id)
                {
                    return Err(bad());
                }
                self.measure_term(l, false, uses)?;
                self.measure_term(r, false, uses)
            }
            _ => Err(bad()),
        }
    }

    /// Checks what the refinements in body `f` need, and works out which
    /// of its operators can't panic, taking those out of its effects: by
    /// the walk over its MIR (docs/65-refinements-on-mir.md).
    pub(super) fn refine(&mut self, f: &FnDecl) -> Result<(), Error> {
        let id = self.current.expect("in a function");
        let symbol = self.fns[id.0].symbol.clone();
        if self.measures.contains_key(&symbol) {
            self.measure_body(&symbol, &f.body)?;
        }
        let done = self.walk_mir(f);
        self.apply(&symbol, done)
    }

    /// Keeps what a walk of the body of function `symbol` found: the
    /// proofs that failed, which don't stop checking (`Walk::failed`), the
    /// const's value, and which operators were proved not to panic, which
    /// are taken out of its effects.
    fn apply(&mut self, symbol: &str, done: Walked) -> Result<(), Error> {
        let Walked {
            mut failed,
            result,
            known,
            proved,
            failures,
            ..
        } = done;
        self.proof_errors.append(&mut failed);
        result?;
        if let Some(k) = known {
            self.const_values.insert(symbol.to_string(), k);
        }
        for (node, failure) in failures {
            let span = self
                .effect_sites
                .iter()
                .filter(|s| s.node == Some(node))
                .map(|s| s.span)
                .chain(
                    self.withins
                        .iter()
                        .filter(|s| s.node == Some(node))
                        .map(|s| s.span),
                )
                .next();
            if let Some(span) = span {
                self.proof_failures.insert(span, failure);
            }
        }
        let keep = |node: Option<NodeId>| node.is_none_or(|n| !proved.contains(&n));
        self.effect_sites.retain(|s| keep(s.node));
        self.withins.retain(|w| keep(w.node));
        self.info.proved.extend(proved);
        self.implicit_panics();
        Ok(())
    }

    /// A walk of the body of `f`, the function being checked, at its start.
    fn new_walk(&self, f: &FnDecl) -> Walk<'_> {
        let id = self.current.expect("in a function");
        Walk {
            cx: self,
            kinds: Vec::new(),
            facts: Vec::new(),
            laws: Vec::new(),
            quotients: HashMap::new(),
            reachable: true,
            cur: HashMap::new(),
            objs: HashMap::new(),
            next_obj: 0,
            measured: HashMap::new(),
            parts: HashMap::new(),
            trees: HashMap::new(),
            decl: HashMap::new(),
            lent_trees: HashMap::new(),
            coerced_args: Vec::new(),
            no_place_reads: false,
            closure_params: HashMap::new(),
            closure_decls: HashMap::new(),
            closures_refused: HashSet::new(),
            module: self.fns[id.0].module,
            invariants: HashMap::new(),
            alias: HashMap::new(),
            declared: Vec::new(),
            proved: HashSet::new(),
            params: f.params.iter().map(|p| p.name.id).collect(),
            entry: Vec::new(),
            ret: None,
            ensures: None,
            proving_fields: false,
            reproving: Vec::new(),
            closures: 0,
            size: None,
            bounds: HashMap::new(),
            consts: HashMap::new(),
            failures: HashMap::new(),
            invalidated: Vec::new(),
            errors: Vec::new(),
            slots: HashMap::new(),
            pinned: Vec::new(),
            deferred: HashMap::new(),
            known: None,
            dead: HashSet::new(),
        }
    }

    /// Warns of each operator of the body just walked, its closures' too,
    /// whose check wasn't proved to pass, so that it may panic
    /// (`implicit_panics`, off unless asked for), with why the proof
    /// failed.
    fn implicit_panics(&mut self) {
        let sites = (self.effect_sites.iter())
            .filter(|s| s.node.is_some())
            .map(|s| (s.span, s.what.clone()));
        let closures = (self.withins.iter()).filter_map(|w| match (&w.why, w.node) {
            (super::effects::Why::Closure(what), Some(_)) => Some((w.span, what.clone())),
            _ => None,
        });
        let mut found: Vec<(Span, String)> = sites.chain(closures).collect();
        found.sort_by_key(|(span, _)| (span.start, span.end));
        found.dedup_by_key(|(span, _)| *span);
        for (span, what) in found {
            let notes = match self.proof_failures.get(&span) {
                Some(failure) => failure.notes(),
                None => Vec::new(),
            };
            self.warnings.push(crate::error::Warning {
                lint: crate::error::Lint::ImplicitPanics,
                span,
                message: format!("{what} may panic: it wasn't proved not to"),
                fixes: Vec::new(),
                notes,
            });
        }
    }
}

/// What a walk of a body found (`Checker::refine`).
struct Walked {
    failed: Vec<Error>,
    result: Result<(), Error>,
    known: Option<Known>,
    proved: HashSet<NodeId>,
    failures: HashMap<NodeId, ProofFailure>,
}

/// The operators a walk proved can't panic, and why each other one
/// wasn't.
type Found = (HashSet<NodeId>, HashMap<NodeId, ProofFailure>);

/// Which of parameters `params` (of a function with result `ret` and
/// lifetime bounds `bounds`) are *call-scoped* `&mut`s: `&'a mut U` where
/// `'a` is in no other part of the signature, not in `U`, another
/// parameter, the result or a bound. Then nothing the function can name
/// outlives `'a`, nor is outlived by it, so no reference it makes from the
/// parameter can be kept anywhere when it returns: what's behind it then
/// is what the caller reads next (see docs/30-postconditions.md). Not if
/// the result has an `impl` type, which may hold every lifetime.
fn call_scoped(params: &[Type], ret: &Type, bounds: &[crate::types::Bound]) -> Vec<bool> {
    fn has_opaque(t: &Type) -> bool {
        matches!(t, Type::Opaque(..)) || t.parts().any(has_opaque)
    }
    let opaque = has_opaque(ret);
    let in_bounds = |r: &crate::types::Region| {
        bounds.iter().any(|b| match b {
            crate::types::Bound::Region(a, b) => a == r || b == r,
            crate::types::Bound::Param(_, a) | crate::types::Bound::Proj(_, a) => a == r,
        })
    };
    (0..params.len())
        .map(|i| {
            let Type::Ref(r @ crate::types::Region::Named(_), true, inner) = &params[i] else {
                return false;
            };
            let elsewhere = params
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, t)| t)
                .chain([ret, &**inner])
                .any(|t| t.regions().contains(&r));
            !opaque && !elsewhere && !in_bounds(r)
        })
        .collect()
}

/// Which of the parameters `params` that aren't call-scoped are
/// *lend-framed* `&mut`s, of a function of scheme `s` with result `ret`
/// (`own` as in `free::free_params`), and the type parameters `W` its
/// result can write values of (see docs/59-lends.md). Parameter `i` is one
/// if it's `&'a mut U`, with `'a` in no other parameter, not in `U` nor in
/// a bound, the result has no `impl` type, and every way the result can
/// write is to values of a type parameter, each free (`free::is_free`):
/// its parts that hold `'a` are `&'a mut X` and `&'a mut [X]`, or `&'a V`
/// with no `Unsafe` in `V` but in its type parameters, through enums,
/// tuples, structs, boxes, arrays and slices. Then by parametricity
/// (lemma L1) what's written through the result, until its borrow ends,
/// is values of `W` in places of those types, which a measure free in
/// `W` can't see (lemma L2): so what the function ensures of such a
/// measure at return is still true when the borrow ends, which is when the
/// caller next reads the place (lemma L3).
pub(super) fn lend_framed(
    adts: &crate::types::Adts,
    s: &crate::types::Scheme,
    own: Option<(&Type, &str)>,
    scoped: &[bool],
) -> Framed {
    let Type::Fn(sig) = &s.ty else {
        return Vec::new();
    };
    let (params, ret) = (&sig.params, &*sig.ret);
    fn has_opaque(t: &Type) -> bool {
        matches!(t, Type::Opaque(..)) || t.parts().any(has_opaque)
    }
    let in_bounds = |r: &crate::types::Region| {
        s.bounds.iter().any(|b| match b {
            crate::types::Bound::Region(a, b) => a == r || b == r,
            crate::types::Bound::Param(_, a) | crate::types::Bound::Proj(_, a) => a == r,
        })
    };
    (0..params.len())
        .map(|i| {
            if scoped.get(i).copied().unwrap_or(false) || has_opaque(ret) {
                return None;
            }
            let Type::Ref(r @ crate::types::Region::Named(_), true, inner) = &params[i] else {
                return None;
            };
            let elsewhere = (params.iter().enumerate())
                .any(|(j, t)| j != i && t.regions().contains(&r))
                || inner.regions().contains(&r);
            if elsewhere || in_bounds(r) {
                return None;
            }
            let mut w = Vec::new();
            if !writes(adts, ret, r, &mut w, &mut Vec::new()) {
                return None;
            }
            w.sort();
            w.dedup();
            w.iter().all(|x| free::is_free(s, x, own)).then_some(w)
        })
        .collect()
}

/// Does what a value of type `t` can write through its references of
/// region `r` go only to values of type parameters, which it adds to `w`?
/// (See `lend_framed`.)
fn writes(
    adts: &crate::types::Adts,
    t: &Type,
    r: &crate::types::Region,
    w: &mut Vec<String>,
    seen: &mut Vec<Type>,
) -> bool {
    if !t.regions().contains(&r) {
        return true;
    }
    let param = |t: &Type| match t {
        Type::Param(x, _) => Some(x.clone()),
        Type::Slice(inner) => match &**inner {
            Type::Param(x, _) => Some(x.clone()),
            _ => None,
        },
        _ => None,
    };
    match t {
        Type::Ref(q, true, inner) if q == r => match param(inner) {
            Some(x) => {
                w.push(x);
                true
            }
            None => false,
        },
        Type::Ref(q, false, inner) if q == r => {
            // Nothing's written through a `&` but what's in an `Unsafe`,
            // or might be, in a type parameter.
            if !frozen_but_params(adts, inner) {
                return false;
            }
            params_in(inner, w);
            writes(adts, inner, r, w, seen)
        }
        Type::Ref(_, _, inner) | Type::Box(inner) | Type::Array(inner, _) | Type::Slice(inner) => {
            writes(adts, inner, r, w, seen)
        }
        Type::Tuple(ts) => ts.iter().all(|t| writes(adts, t, r, w, seen)),
        Type::Adt(symbol, ..) if symbol == crate::check::UNSAFE => false,
        Type::Adt(symbol, _, args) if crate::intrinsic::is_intrinsic_type(symbol) => {
            args.iter().all(|t| writes(adts, t, r, w, seen))
        }
        Type::Adt(..) => {
            if seen.contains(t) {
                return true;
            }
            seen.push(t.clone());
            all_fields(adts, t)
                .iter()
                .all(|f| writes(adts, f, r, w, seen))
        }
        // A function value can't hold a reference.
        Type::Fn(_) => true,
        _ => false,
    }
}

/// Is `t` frozen (`types::frozen`) but for its type parameters?
fn frozen_but_params(adts: &crate::types::Adts, t: &Type) -> bool {
    fn go(adts: &crate::types::Adts, t: &Type, seen: &mut Vec<Type>) -> bool {
        match t {
            Type::Adt(symbol, ..) if symbol == crate::check::UNSAFE => false,
            Type::Adt(symbol, _, args) if crate::intrinsic::is_intrinsic_type(symbol) => {
                args.iter().all(|a| go(adts, a, seen))
            }
            Type::Adt(..) => {
                if seen.contains(t) {
                    return true;
                }
                seen.push(t.clone());
                all_fields(adts, t).iter().all(|f| go(adts, f, seen))
            }
            Type::Box(inner) | Type::Array(inner, _) | Type::Slice(inner) => go(adts, inner, seen),
            Type::Tuple(ts) => ts.iter().all(|t| go(adts, t, seen)),
            _ => true,
        }
    }
    go(adts, t, &mut Vec::new())
}

/// The type parameters in `t`, into `out`.
fn params_in(t: &Type, out: &mut Vec<String>) {
    if let Type::Param(x, _) = t {
        out.push(x.clone());
    }
    for p in t.parts() {
        params_in(p, out);
    }
}

/// The variants pattern `p` matches, if it only says which variant a
/// value is: `V`, `V(_, ..)`, `V { .. }`, `_` (every variant: `[]` with
/// `true`), or several of those with `|`. The `bool` says it matches every
/// variant.
fn variant_test(p: &Pattern) -> Option<(Vec<String>, bool)> {
    match &p.kind {
        PatternKind::Wild => Some((Vec::new(), true)),
        PatternKind::Variant(_, _, v, subs)
            if subs.iter().all(|s| matches!(s.kind, PatternKind::Wild)) =>
        {
            Some((vec![v.name.clone()], false))
        }
        PatternKind::Or(alts) => {
            let mut names = Vec::new();
            let mut all = false;
            for a in alts {
                let (n, every) = variant_test(a)?;
                names.extend(n);
                all |= every;
            }
            Some((names, all))
        }
        _ => None,
    }
}

/// How far what's in a predicate written out of an alias must be seen: as
/// far as the alias is (R5 of docs/25-type-aliases.md).
fn alias_reach(o: &super::written::Origin) -> Option<(String, Level)> {
    let what = |how: &str| format!("`{}` is {how}, but its refinement names", o.alias);
    match o.vis {
        Vis::Private => None,
        Vis::Pub(_) => Some((what("public"), Level::Package)),
        Vis::Export(_) => Some((what("exported"), Level::Export)),
    }
}

/// What `name` names in a predicate, if it's one of the const parameters
/// `consts`: a `usize`, the same all through the function or value.
fn const_named(consts: &[Ident], name: &str) -> Option<Named> {
    let p = consts.iter().rev().find(|p| p.name == name)?;
    let ty = Type::Int(crate::types::IntTy::Usize);
    Some(Named::Value(ty, PredName::Const(name.to_string()), p.span))
}

/// Calls `f` with `e` and everything in it.
fn each_expr(e: &Expr, f: &mut impl FnMut(&Expr)) {
    f(e);
    for c in e.children() {
        each_expr(c, f);
    }
}

/// The value of `e`, if it's a constant, of literals: what a predicate
/// can divide by.
fn constant(e: &Expr) -> Option<i128> {
    match &e.kind {
        ExprKind::Int(n, _) => Some(i128::from(*n)),
        ExprKind::Unary(UnOp::Neg, x) => constant(x)?.checked_neg(),
        ExprKind::Binary(op, l, r) => {
            let (a, b) = (constant(l)?, constant(r)?);
            match op {
                BinOp::Add => a.checked_add(b),
                BinOp::Sub => a.checked_sub(b),
                BinOp::Mul => a.checked_mul(b),
                BinOp::Div => a.checked_div(b),
                BinOp::Rem => a.checked_rem(b),
                _ => None,
            }
        }
        _ => None,
    }
}

/// How many unknowns `e`, an integer term, multiplies together: 0 for a
/// constant, 1 for a sum of names, 2 for a product of two. The logic has
/// products of two (`logic::times`).
fn degree(e: &Expr) -> usize {
    match &e.kind {
        ExprKind::Int(..) => 0,
        ExprKind::Unary(UnOp::Neg, x) => degree(x),
        ExprKind::Binary(BinOp::Add | BinOp::Sub, l, r) => degree(l).max(degree(r)),
        ExprKind::Binary(BinOp::Mul, l, r) => degree(l) + degree(r),
        // A quotient by a constant is an unknown of its own, unless what's
        // divided is a constant.
        ExprKind::Binary(BinOp::Div | BinOp::Rem, l, _) => degree(l).min(1),
        _ => 1,
    }
}

/// What an expression is, to the logic: an `i64`, as a linear sum, a
/// `bool`, as a formula, a struct, as its fields' values, a reference or a
/// box, as what it points to, or anything else, of which nothing is known.
/// But for those, each value is an *object*, which says which value it
/// is, so what a measure gives of it (`Walk::measured`).
/// What the logic knows a const's value is, from its initializer: when
/// the value it worked out for the body is a constant, as it is of a
/// literal, or arithmetic on those and on other consts it knows (lemma C5
/// of docs/36-consts.md). Of a call, it's what the function's type says,
/// so nothing, without a refinement. The interpreter, which works out the
/// value while compiling, checks it's this (`crate::consts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Known {
    Int(i128),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
enum Val {
    Int(Lin),
    Bool(Formula),
    Struct(Obj, Vec<Val>),
    /// An array, known by its length, which its type says: what its
    /// values are isn't followed, but what's said of them is, of its object
    /// (`Walk::trees`).
    Array(Obj, Lin),
    Ref(Obj, Pointee),
    Obj(Obj),
    Other,
}

/// Which value a value that isn't an integer or a `bool` is, by number.
/// Each stands for one value, which never changes: a place that may have
/// changed is another object.
type Obj = u32;

/// What a reference or a box points to: a place, which is read when it's
/// needed, a value, or what isn't known.
#[derive(Debug, Clone, PartialEq)]
enum Pointee {
    Place(Place),
    Val(Box<Val>),
    Unknown,
}

/// What's known of a value of an enum (`Walk::parts`): which of its
/// variants it is, by number, and the values of every variant's fields, of
/// which only those of the variant it is mean anything. Each is a function
/// of the value, as a measure is, so an object's are the same however it's
/// got at (lemma E1 of docs/53-refined-type-arguments.md).
#[derive(Debug, Clone)]
struct Parts {
    /// The enum's symbol.
    symbol: String,
    tag: Lin,
    fields: Vec<Vec<Val>>,
}

/// Where a value is, a *place*: a local variable, and the fields and `*`s
/// from it to there. `self.start`, in a method taking `&mut self`, is the
/// field `start` of `*self`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Place {
    root: NodeId,
    path: Vec<Proj>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Proj {
    /// A struct's field, by index.
    Field(usize),
    /// What a reference or a box points to.
    Deref,
}

impl Place {
    fn local(id: NodeId) -> Place {
        Place {
            root: id,
            path: Vec::new(),
        }
    }

    fn then(&self, p: Proj) -> Place {
        let mut q = self.clone();
        q.path.push(p);
        q
    }

    /// Is it `outer`, or inside it?
    fn within(&self, outer: &Place) -> bool {
        self.root == outer.root && self.path.starts_with(&outer.path)
    }

    /// The field it is of the place it's in, if it's one.
    fn field_of(&self) -> Option<(Place, usize)> {
        match self.path.last()? {
            Proj::Field(i) => {
                let outer = Place {
                    root: self.root,
                    path: self.path[..self.path.len() - 1].to_vec(),
                };
                Some((outer, *i))
            }
            Proj::Deref => None,
        }
    }
}

/// The state to run each branch from.
struct Snap {
    facts: usize,
    cur: HashMap<Place, Var>,
    objs: HashMap<Place, Obj>,
    reachable: bool,
    declared: usize,
    invalidated: Vec<Invalidation>,
}

/// How a path to where paths meet ended: what it learned, what each place
/// stood for at its end, and whether it gets there.
struct Outcome {
    facts: Vec<Formula>,
    cur: HashMap<Place, Var>,
    objs: HashMap<Place, Obj>,
    reachable: bool,
    invalidated: Vec<Invalidation>,
}

/// The pass over one body (see `Checker::refine`).
struct Walk<'c> {
    cx: &'c Checker,
    /// The proofs that failed, which checking goes on past (`failed`).
    errors: Vec<Error>,
    /// What each variable of the logic stands for, by number.
    kinds: Vec<Kind>,
    /// What's known here, on this path.
    facts: Vec<Formula>,
    /// Whether this point can be reached at all.
    reachable: bool,
    /// Which variable of the logic each `i64` or `bool` place stands for
    /// here. One that isn't in it is a value nothing's known of yet, but
    /// its refinements.
    cur: HashMap<Place, Var>,
    /// Which object the value at each place that isn't an integer or a
    /// `bool` is here, as `cur` does for those.
    objs: HashMap<Place, Obj>,
    /// How many objects there are.
    next_obj: Obj,
    /// What each measure gives of each object, once it's been asked: by
    /// the object and the measure's symbol. An object is one value, so a
    /// measure of it is one value too.
    measured: HashMap<(Obj, String), Val>,
    /// What's known of each object that's an enum's value: its variant
    /// and fields (see `Parts`).
    parts: HashMap<Obj, Parts>,
    /// What's said of the values at positions of each object: refinements
    /// of type arguments (`targs`).
    trees: HashMap<Obj, targs::Tree>,
    /// What's declared of each local's value's positions, which each
    /// write must keep (`Walk::place_decl`).
    decl: HashMap<NodeId, targs::Tree>,
    /// What the call that last lent each place `&mut` left at its
    /// positions, for when it's read next (`Walk::place_read`).
    lent_trees: HashMap<Place, targs::Tree>,
    /// What each closure literal can count on of each of its parameters, by
    /// the closure's node, as the call it's given to says
    /// (`targs::closure_trees`).
    closure_params: HashMap<NodeId, Vec<targs::Tree>>,
    /// What each closure literal walked here was walked with of its
    /// parameters, by the closure's type's symbol: what it can count on of
    /// them, and what it keeps of what's written through them. A call that
    /// gives one, for a type parameter the callee calls with a `&mut` to
    /// something refined, needs it to be exactly that (lemma W7 of
    /// docs/63-what-is-declared.md).
    closure_decls: HashMap<String, Vec<targs::Tree>>,
    /// The closures a call was refused for so (`targs::closures_given`):
    /// once is enough to say so.
    closures_refused: HashSet<String>,
    /// While working out what's known where branches meet: what a
    /// reference points to isn't read (`Walk::join_values`).
    no_place_reads: bool,
    /// For each argument of the call being checked, how many more
    /// references or boxes a coercion looked through, and its own type
    /// (`Walk::call_trees`).
    coerced_args: Vec<Option<(usize, Type)>>,
    /// The module of the function walked, where its measures are their
    /// bodies.
    module: usize,
    /// The refinement of each refined local, with its name.
    invariants: HashMap<NodeId, (Refinement, String)>,
    /// What each local bound to a `&` of a place points to: `*r` is that
    /// place, which can't change while `r` can be used.
    alias: HashMap<NodeId, Place>,
    /// The locals in scope, in the order they were bound.
    declared: Vec<NodeId>,
    /// The operators proved not to panic, by node.
    proved: HashSet<NodeId>,
    /// The ids of the function's parameters.
    params: Vec<NodeId>,
    /// Their values at the call (see `Walk::settled`).
    entry: Vec<Val>,
    /// The refinement of the function's result, with its name.
    ret: Option<(Refinement, String)>,
    /// What the function ensures, with its name.
    ensures: Option<(Vec<Refinement>, String)>,
    /// Reading a struct's fields doesn't assume its refinements: they're
    /// being proved.
    proving_fields: bool,
    /// Places lent straight to a call-scoped `&mut` whose own refinement
    /// (a refined local's, or what a refined reference points to) is
    /// proved again after the call (`Walk::post`): until it is, a value
    /// read there doesn't have it.
    reproving: Vec<Place>,
    /// How many closures deep this is.
    closures: usize,
    /// The variable for the greatest `isize`, once there's one: how big
    /// it is depends on the machine. The greatest `usize` is twice it, plus
    /// one, and the least `isize` minus it, minus one.
    size: Option<Var>,
    /// The least and greatest values of each type parameter `T: Int`, once
    /// there are some: it can be any integer type, so they're variables,
    /// or, of a `T: Signed`, the least is -1 minus the greatest, and of a
    /// `T: Unsigned`, 0.
    bounds: HashMap<String, (Lin, Lin)>,
    /// The value of each const parameter of the function, once there's one:
    /// a `usize` the caller picks, the same all through.
    consts: HashMap<String, Val>,
    /// What's true everywhere in the function, of the variables of
    /// `size`, `bounds` and `quotients`, which no path forgets, as those
    /// are made once, on whichever path first needs them; and that a
    /// character's value is a scalar value. Only what holds in every state
    /// may be one: what defines a new variable (lemma Q2 of
    /// docs/72-division.md), or what holds of a value whatever the path.
    laws: Vec<Formula>,
    /// The variable for the quotient of each sum by each constant greater
    /// than 1, whose law (`truncated`) is in `laws` (`Walk::divide`).
    quotients: HashMap<(Lin, i128), Var>,
    /// Explanations saved at the point each operator's proof failed.
    failures: HashMap<NodeId, ProofFailure>,
    /// Source events that changed a place's value along this path.
    invalidated: Vec<Invalidation>,
    /// On MIR, the value of each local that's no variable's, a
    /// temporary's or the result's, here (`mir::Slot`).
    slots: HashMap<crate::mir::Local, mir::Slot>,
    /// On MIR, the scrutinees whose values the patterns being bound take
    /// apart, as they were matched (`Walk::locate`). Needed where the
    /// scrutinee is at no place the walk can name, as `parent[v]` (behind
    /// the reference `index` gives) in `let Some(p) = parent[v] else ..`:
    /// read again, it would say nothing of what's declared there
    /// (`refine_let_else_index.t`; docs/70-bounds-in-impls.md).
    pinned: Vec<(crate::mir::Place, Val)>,
    /// On MIR, the closures made into locals for a call to give to a
    /// function, walked at the call (`Walk::mir_call`): by local, the
    /// lambda's node and the closure's type.
    deferred: HashMap<crate::mir::Local, (NodeId, Type)>,
    /// On MIR, the value of the const being walked, if its body is known
    /// to give a constant.
    known: Option<Known>,
    /// The variables gone out of scope on this path: what's known of them
    /// is forgotten where paths meet.
    dead: HashSet<NodeId>,
}

/// A call, as `Walk::call_at` needs it: its result's type, the function
/// it calls, if it's one (resolved: `Walk::callee_at`), and of each of its
/// arguments (a method's receiver first), where it is, its type, its
/// text, and the references a coercion looked through.
struct CallAt {
    ty: Type,
    resolved: Option<(String, Vec<Type>)>,
    spans: Vec<Span>,
    tys: Vec<Type>,
    texts: Vec<String>,
    coerced: Vec<Option<(usize, Type)>>,
    span: Span,
}

/// A call's postconditions, to learn once what it's lent is forgotten.
struct Post {
    /// The callee's name, for messages.
    name: String,
    sig: Option<RefinedSig>,
    /// The arguments' values at the call, what's behind references
    /// included.
    old: Vec<Val>,
    /// The place each argument lends to a call-scoped `&mut` parameter.
    scoped: Vec<Option<Place>>,
    targs: Vec<(String, Type)>,
    result: Val,
    /// What it leaves at the positions of each place it's lent `&mut`.
    lent: Vec<(Place, targs::Tree)>,
}

/// How many references deep what a function's parameters point to is kept
/// as it is at the call.
const ENTRY_DEPTH: usize = 4;

/// How a predicate's names other than its binder get their values: those
/// of the function being walked, a call's arguments, or the fields of a
/// struct.
#[derive(Clone, Copy)]
enum Env<'a> {
    Here,
    /// At a call, with the arguments' values, and what the callee's type
    /// parameters are there, for its const parameters. One that isn't
    /// among them is the function's own.
    Call(&'a [Val], &'a [(String, Type)]),
    /// Of a struct's value, with its fields' values, and its type
    /// parameters' arguments.
    Fields(&'a [Val], &'a [(String, Type)]),
    /// A postcondition's: the arguments' values at the call, for `old(..)`,
    /// and at return, or `None` for the function being walked, whose
    /// parameters are read where it returns; and the type arguments.
    Post(&'a [Val], Option<&'a [Val]>, &'a [(String, Type)]),
    /// A refinement of a type argument's, with the value of each name in
    /// it by node, as it was where it was said (`targs::Closed`).
    Closed(&'a HashMap<NodeId, Val>),
}

impl Walk<'_> {
    fn fresh(&mut self, kind: Kind) -> Var {
        self.kinds.push(kind);
        (self.kinds.len() - 1) as Var
    }

    /// `a / k` or `a % k` (`op`) in the logic, for a constant `k` other
    /// than 0, which is linear (docs/72-division.md): the quotient by
    /// `|k|` is a variable, one for each sum `a` and `|k|`, which a law
    /// (`truncated`) makes the integers' quotient, rounding towards zero, as
    /// `/` does: a function of `a`, so it's the same wherever it's said,
    /// in a body or a predicate, on any path. `a / k` is it, or its
    /// negation if `k < 0`, and `a % k` is `a` minus `|k|` times it, as `a
    /// == (a / k) * k + a % k`. `None` if a number gets too big.
    fn divide(&mut self, op: BinOp, a: &Lin, k: i128) -> Option<Lin> {
        let m = k.checked_abs().filter(|&m| m != 0)?;
        let q = if let Some(n) = a.as_constant() {
            Lin::constant(n.checked_div(m)?)
        } else if m == 1 {
            a.clone()
        } else if let Some(&q) = self.quotients.get(&(a.clone(), m)) {
            Lin::var(q)
        } else {
            let v = self.fresh(Kind::Integer);
            let sign = logic::range_of(a, &self.kinds).map(|(lo, hi)| (lo >= 0, hi <= 0));
            let law = truncated(a, m, &Lin::var(v), sign)?;
            self.laws.push(law);
            self.quotients.insert((a.clone(), m), v);
            Lin::var(v)
        };
        match op {
            BinOp::Div if k > 0 => Some(q),
            BinOp::Div => q.scale(-1),
            _ => a.sub(&q.scale(m)?),
        }
    }

    /// `a op b`, in a predicate or a measure's body, if it's in the logic
    /// (`binary`), where `/` and `%` are by a constant (`divide`).
    fn logic_op(&mut self, op: BinOp, a: Val, b: Val) -> Result<Val, Failure> {
        match (op, a, b) {
            (BinOp::Div | BinOp::Rem, Val::Int(a), Val::Int(b)) => {
                let k = b.as_constant().ok_or(Failure::Unsupported)?;
                let v = self.divide(op, &a, k).ok_or(Failure::ArithmeticLimit)?;
                Ok(Val::Int(v))
            }
            (op, a, b) => binary(op, a, b, &mut self.kinds),
        }
    }

    /// Variable `v` of the logic, as a value.
    fn var(&self, v: Var) -> Val {
        match self.kinds[v as usize] {
            Kind::Int(..) | Kind::Bound(..) | Kind::Product(..) | Kind::Integer => {
                Val::Int(Lin::var(v))
            }
            Kind::Bool => Val::Bool(Formula::holds(v)),
        }
    }

    /// The least and greatest values of `t`, if it's an integer type, or a
    /// type parameter that's `Int`, which can be any of them. What's known
    /// of one whose size isn't, as the logic says, is only what holds of
    /// all it can be: of `isize` and `usize`, what holds of 32 and 64 bits,
    /// and of a type parameter, what holds of every integer type, that its
    /// least is at most 0 and its greatest at least 127.
    fn range(&mut self, t: &Type) -> Option<(Lin, Lin)> {
        let size = |w: &mut Self| match w.size {
            Some(v) => Lin::var(v),
            None => {
                let (_, lo) = IntTy::I32.range_64();
                let (_, hi) = IntTy::I64.range_64();
                let v = w.fresh(Kind::Bound(lo, hi));
                w.size = Some(v);
                Lin::var(v)
            }
        };
        match t {
            Type::Int(IntTy::Isize) => {
                let max = size(self);
                Some((max.scale(-1)?.plus(-1)?, max))
            }
            Type::Int(IntTy::Usize) => Some((Lin::constant(0), size(self).scale(2)?.plus(1)?)),
            Type::Int(t) => {
                let (lo, hi) = t.range()?;
                Some((Lin::constant(lo), Lin::constant(hi)))
            }
            // But for the surrogates, in between: see `scalar`.
            Type::Char => Some((Lin::constant(0), Lin::constant(0x10FFFF))),
            Type::Param(p, _) if self.is_int(t) => {
                if let Some(b) = self.bounds.get(p) {
                    return Some(b.clone());
                }
                let (min, _) = IntTy::I64.range_64();
                let (_, max) = IntTy::U64.range_64();
                let (every_lo, every_hi) = super::ints::param_range(self.cx, t);
                let b = if self.implements(t, UNSIGNED) {
                    let hi = self.fresh(Kind::Bound(every_hi, max));
                    (Lin::constant(0), Lin::var(hi))
                } else if self.implements(t, SIGNED) {
                    let (_, i64_max) = IntTy::I64.range_64();
                    let hi = Lin::var(self.fresh(Kind::Bound(every_hi, i64_max)));
                    (hi.scale(-1)?.plus(-1)?, hi)
                } else {
                    let (lo, hi) = (
                        Lin::var(self.fresh(Kind::Bound(min, every_lo))),
                        Lin::var(self.fresh(Kind::Bound(every_hi, max))),
                    );
                    // Two's complement: the greatest is at least -1 minus
                    // the least, as it's equal in a signed type, and the
                    // least is 0 in an unsigned one.
                    let law = Formula::le(&Lin::constant(-1), &hi.add(&lo)?)?;
                    self.laws.push(law);
                    (lo, hi)
                };
                self.bounds.insert(p.clone(), b.clone());
                Some(b)
            }
            _ => None,
        }
    }

    /// That `v`, a character's value, is a Unicode scalar value: in its
    /// range, and not a surrogate. Its range is a variable's kind, but the
    /// gap in it isn't, as a kind is an interval: it's said where a
    /// character's value turns into an integer, by `as`, the only place
    /// it's seen as a number.
    fn scalar(v: &Lin) -> Option<Formula> {
        Some(Formula::and([
            Formula::le(&Lin::constant(0), v)?,
            Formula::le(v, &Lin::constant(0x10FFFF))?,
            Formula::or([
                Formula::le(v, &Lin::constant(0xD7FF))?,
                Formula::le(&Lin::constant(0xE000), v)?,
            ]),
        ]))
    }

    /// Is `t` an integer type, a type parameter that's `Int`, or `char`:
    /// is its value a number, in the logic?
    fn is_scalar(&self, t: &Type) -> bool {
        matches!(t, Type::Char) || self.is_int(t)
    }

    /// Is `t` an integer type, or a type parameter that's `Int`?
    fn is_int(&self, t: &Type) -> bool {
        match t {
            Type::Int(_) => true,
            Type::Param(_, _) => self.implements(t, INT),
            _ => false,
        }
    }

    /// Does `t` implement built-in trait `name`, where the function is?
    fn implements(&self, t: &Type, name: &str) -> bool {
        let cx = self.cx;
        (cx.traits
            .implements(t, name, &cx.adts.proj.env, &cx.adts.proj.local))
        .is_ok()
    }

    /// The kind of variable for a value of type `t`, if it's an integer or
    /// a `bool`.
    fn kind(&mut self, t: &Type) -> Option<Kind> {
        match t {
            Type::Bool => Some(Kind::Bool),
            t => self.range(t).map(|(lo, hi)| Kind::Int(lo, hi)),
        }
    }

    /// A value of type `t` of which nothing is known: but if it's a
    /// struct, its fields have its refinements.
    fn unknown(&mut self, t: &Type) -> Val {
        match t {
            Type::Int(_) | Type::Param(_, _) | Type::Char if self.is_scalar(t) => {
                match self.kind(t) {
                    Some(k) => Val::Int(Lin::var(self.fresh(k))),
                    None => Val::Other,
                }
            }
            Type::Bool => Val::Bool(Formula::holds(self.fresh(Kind::Bool))),
            // A tuple is a struct, its fields `0`, `1`, .. (lemma T5 of
            // docs/58-tuples.md).
            Type::Adt(..) | Type::Tuple(_) => match struct_fields(&self.cx.adts, t) {
                Some(fields) => {
                    let vals: Vec<Val> = fields
                        .iter()
                        .map(|(_, ft)| {
                            let ft = self.cx.solved(ft);
                            self.unknown(&ft)
                        })
                        .collect();
                    self.assume_fields(t, &vals);
                    Val::Struct(self.new_obj(), vals)
                }
                None => {
                    let o = self.new_obj();
                    self.enum_parts(o, t, None);
                    Val::Obj(o)
                }
            },
            // What its type says of an array, its length, holds behind any
            // reference to it: nothing can change it.
            Type::Array(_, n) => match self.length(n) {
                Val::Int(l) => Val::Array(self.new_obj(), l),
                _ => Val::Obj(self.new_obj()),
            },
            Type::Ref(_, _, inner) | Type::Box(inner) if matches!(**inner, Type::Array(..)) => {
                let a = self.unknown(inner);
                Val::Ref(self.new_obj(), Pointee::Val(Box::new(a)))
            }
            Type::Ref(..) | Type::Box(_) => Val::Ref(self.new_obj(), Pointee::Unknown),
            t if has_obj(t) => Val::Obj(self.new_obj()),
            _ => Val::Other,
        }
    }

    /// Gives object `o`, a value of type `t`, its `Parts` if `t` is an
    /// enum and it has none yet: those of `known`, a variant and its
    /// fields' values, if that's what's known, or else new ones nothing's
    /// known of. Each field of a variant it isn't, too, so that a
    /// refinement of it can be said, under the condition that it is.
    fn enum_parts(&mut self, o: Obj, t: &Type, known: Option<(usize, Vec<Val>)>) {
        if self.parts.contains_key(&o) {
            return;
        }
        let Type::Adt(symbol, ..) = t else {
            return;
        };
        let Some(crate::types::AdtKind::Enum(variants)) = self.cx.adts.get(symbol).map(|d| &d.kind)
        else {
            return;
        };
        let n = variants.len();
        let mut fields = Vec::new();
        let mut known = known;
        for v in 0..n {
            if let Some((k, vals)) = &mut known
                && *k == v
            {
                fields.push(std::mem::take(vals));
                continue;
            }
            let types = crate::types::variant_fields(&self.cx.adts, t, v);
            let vals = (types.iter())
                .map(|ft| {
                    let ft = self.cx.solved(ft);
                    self.unknown(&ft)
                })
                .collect();
            fields.push(vals);
        }
        let tag = match known {
            Some((k, _)) => Lin::constant(i128::try_from(k).expect("few variants")),
            None => {
                let last = i128::try_from(n).expect("few variants") - 1;
                Lin::var(self.fresh(Kind::Int(Lin::constant(0), Lin::constant(last.max(0)))))
            }
        };
        let symbol = symbol.clone();
        // Every value of the enum has its variants' fields' refinements,
        // proved where it was made.
        self.assume_variants(t, &tag, &fields);
        self.parts.insert(
            o,
            Parts {
                symbol,
                tag,
                fields,
            },
        );
    }

    /// The index of variant `name` of the enum `symbol`.
    fn variant_index(&self, symbol: &str, name: &str) -> Option<usize> {
        match &self.cx.adts.get(symbol)?.kind {
            crate::types::AdtKind::Enum(variants) => variants.iter().position(|(v, _)| v == name),
            crate::types::AdtKind::Struct(_) => None,
        }
    }

    fn new_obj(&mut self) -> Obj {
        self.next_obj += 1;
        self.next_obj - 1
    }

    /// The object the value at place `p` is: the one it was, if it can't
    /// have changed since, or a new one.
    /// Gives too whether it's a new one.
    fn obj_at(&mut self, p: &Place) -> (Obj, bool) {
        if let Some(&o) = self.objs.get(p) {
            return (o, false);
        }
        let o = self.new_obj();
        self.objs.insert(p.clone(), o);
        (o, true)
    }

    /// What a reference or box `v` points to, if that's known.
    fn pointee(&mut self, v: Val) -> Val {
        match v {
            Val::Ref(_, Pointee::Place(q)) => self.read_place(&q),
            Val::Ref(_, Pointee::Val(v)) => *v,
            _ => Val::Other,
        }
    }

    /// What reference or box `r`, of type `t`, of which `decl` is what's
    /// declared, points to, and what's declared of it: below the top of
    /// `decl`, and what `r` itself says (as `place_decl` does through a
    /// `*`). What it says is known of what it points to.
    fn deref_lvalue(&mut self, r: Val, decl: &targs::Tree, t: &Type) -> (Val, targs::Tree) {
        let mut out = targs::below(decl, 0);
        let said = (targs::obj_of(&r))
            .and_then(|o| self.trees.get(&o))
            .map(|tree| targs::below(tree, 0))
            .unwrap_or_default();
        out.extend(said.iter().cloned());
        // What's declared of the place it's to, if that's known (lemma F1
        // of docs/68-fail-closed.md).
        if let Val::Ref(_, Pointee::Place(p)) = &r {
            let p = p.clone();
            out.extend(self.place_decl(&p));
        }
        let inner = match t {
            Type::Ref(_, _, inner) | Type::Box(inner) => self.cx.solved(inner),
            _ => return (Val::Other, out),
        };
        let got = match self.pointee(r) {
            Val::Other => self.unknown(&inner),
            v => v,
        };
        if !said.is_empty() {
            self.assume_tree(&got, &inner, &said);
        }
        (got, out)
    }

    /// The receiver `v` is, after `derefs` `*`s, and then a `&` or `&mut`
    /// if `by` says so: a reborrow of what a reference points to is that
    /// reference, the same object.
    fn autoref(&mut self, mut v: Val, derefs: usize, by: Option<bool>) -> Val {
        let peel = match by {
            Some(_) if derefs > 0 => derefs - 1,
            _ => derefs,
        };
        for _ in 0..peel {
            v = self.pointee(v);
        }
        match by {
            Some(_) if derefs == 0 => Val::Ref(self.new_obj(), Pointee::Val(Box::new(v))),
            _ => v,
        }
    }

    /// What measure `symbol` gives of the value `recv`, its `&self`, points
    /// to. In its module, a struct's is its body, of the struct's fields;
    /// elsewhere, and of anything else, it's a variable of the logic, the
    /// same each time it's asked of the same object. A slice's or a
    /// string's is of the reference, which says how long what it points to
    /// is.
    fn measure(&mut self, symbol: &str, recv: Val) -> Val {
        let m = self.cx.measures[symbol].clone();
        if let Some(body) = &m.body
            && (m.module == self.module || !m.reads_fields)
        {
            let term = match &body.kind {
                ExprKind::Block(_, Some(tail)) => &**tail,
                _ => body,
            };
            if let Some(v) = self.measure_term(term, &recv) {
                return v;
            }
        }
        // An array's is its length.
        if m.of_array
            && let Val::Array(_, l) = self.pointee(recv.clone())
        {
            return Val::Int(l);
        }
        let of = match (&recv, m.unsized_self) {
            (Val::Ref(o, _), true) => Some(*o),
            (_, true) => None,
            (_, false) => {
                let target = self.pointee(recv.clone());
                match target {
                    Val::Struct(o, _) | Val::Ref(o, _) | Val::Obj(o) => Some(o),
                    _ => None,
                }
            }
        };
        let Some(o) = of else {
            return self.measure_value(&m.ret);
        };
        if let Some(v) = self.measured.get(&(o, symbol.to_string())) {
            return v.clone();
        }
        let v = self.measure_value(&m.ret);
        self.measured.insert((o, symbol.to_string()), v.clone());
        v
    }

    /// A value of what a measure gives, `ret`, nothing's known of. One of
    /// a type parameter's, `T: Int` in the measure's impl, is of whichever
    /// integer type that is where it's called: so an integer every
    /// integer type has room for.
    fn measure_value(&mut self, ret: &Type) -> Val {
        match ret {
            Type::Param(_, _) => {
                let (lo, _) = IntTy::I64.range_64();
                let (_, hi) = IntTy::U64.range_64();
                let k = Kind::Int(Lin::constant(lo), Lin::constant(hi));
                Val::Int(Lin::var(self.fresh(k)))
            }
            t => self.unknown(t),
        }
    }

    /// The value of term `e` of a measure's body, whose `self` is `this`,
    /// if the logic can say it.
    fn measure_term(&mut self, e: &Expr, this: &Val) -> Option<Val> {
        Some(match &e.kind {
            ExprKind::Int(n, _) => Val::Int(Lin::constant(i128::from(*n))),
            ExprKind::Bool(b) => Val::Bool(if *b { Formula::True } else { Formula::False }),
            ExprKind::Var(..) => this.clone(),
            ExprKind::Field(x, _) => {
                let (derefs, i) = *self.cx.typed.fields.get(&e.id)?;
                let mut v = self.measure_term(x, this)?;
                for _ in 0..derefs {
                    v = self.pointee(v);
                }
                match v {
                    Val::Struct(_, mut vals) if i < vals.len() => vals.swap_remove(i),
                    _ => return None,
                }
            }
            ExprKind::MethodCall(x, ..) => {
                let (Res::Fn(symbol, _), derefs, by) = self.cx.typed.methods.get(&e.id)?.clone()
                else {
                    return None;
                };
                let v = self.measure_term(x, this)?;
                let recv = self.autoref(v, derefs, by);
                self.measure(&symbol, recv)
            }
            ExprKind::Unary(UnOp::Neg, x) => match self.measure_term(x, this)? {
                Val::Int(a) => Val::Int(a.scale(-1)?),
                _ => return None,
            },
            ExprKind::Unary(UnOp::Not, x) => match self.measure_term(x, this)? {
                Val::Bool(f) => Val::Bool(f.not()),
                _ => return None,
            },
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.measure_term(l, this)?, self.measure_term(r, this)?);
                self.logic_op(*op, a, b).ok()?
            }
            // Which variant it is: the arm whose variant the tag says, the
            // first that does (lemma L5 of docs/59-lends.md).
            ExprKind::Match(x, arms) => {
                let mut v = self.measure_term(x, this)?;
                while let Val::Ref(..) = v {
                    v = self.pointee(v);
                }
                let Val::Obj(o) = v else {
                    return None;
                };
                let parts = self.parts.get(&o)?.clone();
                let mut earlier: Vec<Formula> = Vec::new();
                let mut cases: Vec<(Formula, Val)> = Vec::new();
                for a in arms {
                    let (names, every) = variant_test(&a.pat)?;
                    let here = match every {
                        true => Formula::True,
                        false => {
                            let mut tests = Vec::new();
                            for n in &names {
                                let k = self.variant_index(&parts.symbol, n)?;
                                let k = i128::try_from(k).ok()?;
                                tests.push(Formula::eq(&parts.tag, &Lin::constant(k))?);
                            }
                            Formula::or(tests)
                        }
                    };
                    let first = Formula::and(
                        earlier
                            .iter()
                            .map(|f| f.clone().not())
                            .chain([here.clone()])
                            .collect::<Vec<_>>(),
                    );
                    earlier.push(here);
                    cases.push((first, self.measure_term(&a.body, this)?));
                }
                match cases.first().map(|(_, v)| v.clone()) {
                    Some(Val::Bool(_)) => {
                        let mut any = Vec::new();
                        for (c, v) in cases {
                            let Val::Bool(f) = v else {
                                return None;
                            };
                            any.push(Formula::and([c, f]));
                        }
                        Val::Bool(Formula::or(any))
                    }
                    Some(Val::Int(_)) => {
                        let r = Lin::var(self.fresh(Kind::Int(
                            Lin::constant(i128::from(i64::MIN)),
                            Lin::constant(i128::from(u64::MAX)),
                        )));
                        for (c, v) in cases {
                            let Val::Int(n) = v else {
                                return None;
                            };
                            let f = Formula::or([c.not(), Formula::eq(&r, &n)?]);
                            self.fact(f);
                        }
                        Val::Int(r)
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// The refinements of the fields of `t`, if it's a struct that has any.
    fn refined_fields(&self, t: &Type) -> Option<&RefinedFields> {
        match t {
            Type::Adt(symbol, ..) => self.cx.field_refined.get(symbol),
            _ => None,
        }
    }

    /// The refinements of enum `t`'s variants' fields, if it has any.
    fn refined_variants(&self, t: &Type) -> Option<&RefinedVariants> {
        match t {
            Type::Adt(symbol, ..) => self.cx.variant_refined.get(symbol),
            _ => None,
        }
    }

    /// Checks that `vals`, the fields of variant `v` of enum `t` being
    /// made, have their refinements, written at `spans`.
    fn need_variant(
        &mut self,
        t: &Type,
        v: usize,
        vals: &[Val],
        spans: &[Span],
        at: Span,
    ) -> Result<(), Error> {
        let Some(rv) = self.refined_variants(t).cloned() else {
            return Ok(());
        };
        let variant = &rv.variants[v];
        let args = self.adt_args(t);
        for (j, r) in variant.refs.iter().enumerate() {
            if let Some(r) = r {
                let what = format!("field {j} of `{}::{}`", rv.name, variant.name);
                let span = spans.get(j).copied().unwrap_or(at);
                self.need(r, &vals[j], Env::Fields(vals, &args), span, what)?;
            }
        }
        Ok(())
    }

    /// What's known of every value of enum `t`, whose tag is `tag` and
    /// whose variants' fields are `fields`: if it's a variant, its fields'
    /// refinements.
    fn assume_variants(&mut self, t: &Type, tag: &Lin, fields: &[Vec<Val>]) {
        let Some(rv) = self.refined_variants(t).cloned() else {
            return;
        };
        let args = self.adt_args(t);
        for (v, variant) in rv.variants.iter().enumerate() {
            let n = i128::try_from(v).expect("few variants");
            let Some(is) = Formula::eq(tag, &Lin::constant(n)) else {
                continue;
            };
            for (j, r) in variant.refs.iter().enumerate() {
                if let Some(r) = r
                    && let Some(f) = self.pred(r, &fields[v][j], Env::Fields(&fields[v], &args))
                {
                    self.fact(Formula::or([is.clone().not(), f]));
                }
            }
        }
    }

    /// What's known of a value of struct `t` whose fields are `vals`: its
    /// refinements, which every value of it has.
    fn assume_fields(&mut self, t: &Type, vals: &[Val]) {
        let Some(fields) = self.refined_fields(t).cloned() else {
            return;
        };
        for (j, r) in fields.refs.iter().enumerate() {
            if let Some(r) = r
                && let Some(f) = self.pred(r, &vals[j], Env::Fields(vals, &self.adt_args(t)))
            {
                self.fact(f);
            }
        }
    }

    fn ty_of_bind(&self, id: NodeId) -> Type {
        match self.cx.typed.binds.get(&id) {
            Some(b) => self.cx.solved(&b.ty),
            None => Type::Unit,
        }
    }

    /// The type of what's at place `p`.
    fn place_ty(&self, p: &Place) -> Type {
        let mut t = self.ty_of_bind(p.root);
        for proj in &p.path {
            t = match (proj, t) {
                (Proj::Deref, Type::Ref(_, _, inner) | Type::Box(inner)) => self.cx.solved(&inner),
                (Proj::Field(i), t) => match struct_fields(&self.cx.adts, &t) {
                    Some(fields) if *i < fields.len() => self.cx.solved(&fields[*i].1),
                    _ => Type::Unit,
                },
                _ => Type::Unit,
            };
        }
        t
    }

    /// What `p` points to: if `p` is a local bound to a `&` of a place,
    /// that place.
    fn deref(&self, p: Place) -> Place {
        if p.path.is_empty()
            && let Some(q) = self.alias.get(&p.root)
        {
            return q.clone();
        }
        p.then(Proj::Deref)
    }

    fn fact(&mut self, f: Formula) {
        if f != Formula::True {
            self.facts.push(f);
        }
    }

    /// Does `goal` follow from what's known? Anything does where nothing
    /// gets to.
    fn proves(&self, goal: Option<&Formula>) -> Result<(), Failure> {
        if !self.reachable {
            return Ok(());
        }
        let Some(g) = goal else {
            return Err(Failure::Unsupported);
        };
        if self.laws.is_empty() {
            return logic::valid(&self.facts, g, &self.kinds);
        }
        let all: Vec<Formula> = self.facts.iter().chain(&self.laws).cloned().collect();
        logic::valid(&all, g, &self.kinds)
    }

    /// The value at place `p`.
    fn read_place(&mut self, p: &Place) -> Val {
        let t = self.place_ty(p);
        match t {
            Type::Bool => self.leaf(p, &t),
            _ if self.is_scalar(&t) => self.leaf(p, &t),
            Type::Adt(..) | Type::Tuple(_) | Type::Ref(..) | Type::Box(_) => self.read_obj(p, &t),
            t if has_obj(&t) => self.read_obj(p, &t),
            _ => Val::Other,
        }
    }

    /// The value at place `p`, of type `t`, which has an object: a new one
    /// of a refined variable has its refinement.
    fn read_obj(&mut self, p: &Place, t: &Type) -> Val {
        let (o, new) = self.obj_at(p);
        let v = match t {
            Type::Adt(..) | Type::Tuple(_) => match self.fields_of(p, t) {
                Some(vals) => Val::Struct(o, vals),
                None => {
                    self.enum_parts(o, t, None);
                    Val::Obj(o)
                }
            },
            Type::Ref(..) | Type::Box(_) => Val::Ref(o, Pointee::Place(self.deref(p.clone()))),
            // Its length is its type's, whatever's been done to it.
            Type::Array(_, n) => match self.length(n) {
                Val::Int(l) => Val::Array(o, l),
                _ => Val::Obj(o),
            },
            _ => Val::Obj(o),
        };
        if new && p.path.is_empty() {
            self.assume_invariant(p.root, &v);
        }
        // What's declared there, and what a call it was lent to left.
        if new {
            let tree = self.place_read(p);
            self.assume_tree(&v, t, &tree);
        }
        // A new value of a field, as after a loop that lends it, is one the
        // struct's refinements hold of, with its other fields, as of every
        // value of the struct (`fields_of`, for a field that's a scalar).
        if new
            && !self.proving_fields
            && let Some((q, _)) = p.field_of()
        {
            let qt = self.place_ty(&q);
            if self.refined_fields(&qt).is_some()
                && let Some(vals) = self.fields_of(&q, &qt)
            {
                self.assume_fields(&qt, &vals);
            }
        }
        v
    }

    /// The values of the fields of the struct at `q`, of type `t`: each
    /// one's variable, and a new one for each nothing's known of, which,
    /// with the rest, has the struct's refinements.
    fn fields_of(&mut self, q: &Place, t: &Type) -> Option<Vec<Val>> {
        let fields = struct_fields(&self.cx.adts, t)?;
        let mut fresh = false;
        let mut vals = Vec::new();
        for (j, (_, ft)) in fields.iter().enumerate() {
            let ft = self.cx.solved(ft);
            let p = q.then(Proj::Field(j));
            let Some(kind) = self.kind(&ft) else {
                // A field that's a new object is a new value too.
                if has_obj(&ft) && !self.objs.contains_key(&p) {
                    fresh = true;
                }
                vals.push(self.read_place(&p));
                continue;
            };
            let v = match self.cur.get(&p) {
                Some(&v) => v,
                _ => {
                    fresh = true;
                    let v = self.fresh(kind);
                    let tree = self.place_read(&p);
                    let val = self.var(v);
                    self.assume_tree(&val, &ft, &tree);
                    self.cur.insert(p, v);
                    v
                }
            };
            vals.push(self.var(v));
        }
        if fresh && !self.proving_fields {
            self.assume_fields(t, &vals);
        }
        // And what's declared of the struct where it is, as `read_obj` says
        // of one it reads whole: a field read through a reference a
        // pattern took out of an enum is of a struct whose refinement only
        // the reference says (`place_decl`).
        if fresh && !self.proving_fields && !self.objs.contains_key(q) {
            let tree = self.place_read(q);
            if !tree.is_empty() {
                let (o, _) = self.obj_at(q);
                self.assume_tree(&Val::Struct(o, vals.clone()), t, &tree);
            }
        }
        Some(vals)
    }

    /// The `i64` or `bool`, of type `t`, at place `p`.
    fn leaf(&mut self, p: &Place, t: &Type) -> Val {
        if let Some((q, i)) = p.field_of() {
            let qt = self.place_ty(&q);
            if let Some(mut vals) = self.fields_of(&q, &qt) {
                return vals.swap_remove(i);
            }
        }
        if let Some(&v) = self.cur.get(p) {
            return self.var(v);
        }
        let v = self.unknown(t);
        if let Val::Int(_) | Val::Bool(_) = &v {
            let var = self.kinds.len() as Var - 1;
            self.cur.insert(p.clone(), var);
        }
        if p.path.is_empty() {
            self.assume_invariant(p.root, &v);
        }
        let tree = self.place_read(p);
        self.assume_tree(&v, t, &tree);
        v
    }

    /// Forgets what's known of what's at `p`, and in it: it has a new value.
    /// What it's in changes with it, up to a `*`: a reference is the same
    /// one whatever's written through it.
    fn kill(&mut self, p: &Place) {
        self.cur.retain(|q, _| !q.within(p));
        self.lent_trees.retain(|q, _| !q.within(p));
        // What a box owns is part of its value, as a field is: so the box,
        // and what it's in, change with it too.
        let mut boxed = vec![true; p.path.len() + 1];
        let mut q = Place::local(p.root);
        for (i, step) in p.path.iter().enumerate() {
            let by_box = *step != Proj::Deref || matches!(self.place_ty(&q), Type::Box(_));
            boxed[i + 1] = by_box;
            q = q.then(*step);
        }
        let around = |q: &Place| p.within(q) && (q.path.len()..p.path.len()).all(|i| boxed[i + 1]);
        self.objs.retain(|q, _| !q.within(p) && !around(q));
    }

    /// Place `p` has a new value, `val` if it's known.
    fn write(&mut self, p: &Place, val: Option<Val>, at: Option<Span>) {
        if let Some(span) = at {
            self.invalidate(p, span, "assignment");
        }
        self.kill(p);
        let t = self.place_ty(p);
        self.set(p, &t, val);
    }

    /// What's at `p`, of type `t`, which nothing's known of, is `val`, if
    /// that's known: each `i64` or `bool` in it stands for a new variable of
    /// the logic, equal to its part of `val`, which has its local's
    /// refinement, if it has one.
    fn set(&mut self, p: &Place, t: &Type, val: Option<Val>) {
        let kind = match t {
            Type::Adt(..) | Type::Tuple(_) => {
                if let (Some(Val::Struct(o, vals)), Some(fields)) =
                    (&val, struct_fields(&self.cx.adts, t))
                {
                    self.objs.insert(p.clone(), *o);
                    for (j, (v, (_, ft))) in vals.iter().zip(fields).enumerate() {
                        let ft = self.cx.solved(&ft);
                        self.set(&p.then(Proj::Field(j)), &ft, Some(v.clone()));
                    }
                }
                if let Some(Val::Obj(o)) = &val {
                    self.objs.insert(p.clone(), *o);
                }
                if let (Some(v), true) = (&val, p.path.is_empty()) {
                    self.assume_invariant(p.root, v);
                }
                return;
            }
            _ if !self.is_scalar(t) && *t != Type::Bool => {
                if let Some(Val::Ref(o, _) | Val::Obj(o)) = &val {
                    self.objs.insert(p.clone(), *o);
                }
                // What a box owns is a place in it, which has its value; and
                // what a shared reference points to is one that can't change
                // while it's borrowed.
                if let (
                    Type::Box(inner) | Type::Ref(_, false, inner),
                    Some(Val::Ref(_, Pointee::Val(w))),
                ) = (t, &val)
                {
                    let inner = self.cx.solved(inner);
                    self.set(&p.then(Proj::Deref), &inner, Some((**w).clone()));
                }
                if let (Some(v), true) = (&val, p.path.is_empty()) {
                    self.assume_invariant(p.root, v);
                }
                return;
            }
            t => match self.kind(t) {
                Some(kind) => kind,
                None => return,
            },
        };
        let v = self.fresh(kind);
        self.cur.insert(p.clone(), v);
        match val {
            Some(val) => {
                for f in equal(&self.var(v), &val) {
                    self.fact(f);
                }
            }
            // A value nothing's known of has what's declared there.
            None => {
                let tree = self.place_read(p);
                let now = self.var(v);
                self.assume_tree(&now, t, &tree);
            }
        }
        if p.path.is_empty() {
            let now = self.var(v);
            self.assume_invariant(p.root, &now);
        }
    }

    /// Local `id` is bound, to `val` if it's known.
    fn bind(&mut self, id: NodeId, val: Option<Val>) {
        self.declared.push(id);
        self.dead.remove(&id);
        self.write(&Place::local(id), val, None);
    }

    /// Local `id`, named `name`, is declared with refinement `r`: of its
    /// whole value and enums' parts, an invariant; what's carried, what's
    /// declared of the values at its positions (`decl`), of its names now.
    fn declare(&mut self, id: NodeId, r: &Refinement, name: &str) {
        if let Some(ways) = r.ways_only() {
            self.invariants.insert(id, (ways, name.to_string()));
        }
        if r.has_carried() {
            let tree = self.carried_tree(r, Env::Here);
            self.decl.insert(id, tree);
        }
    }

    fn assume_invariant(&mut self, id: NodeId, val: &Val) {
        if self.reproving.contains(&Place::local(id)) {
            return;
        }
        if let Some((r, _)) = self.invariants.get(&id).cloned()
            && let Some(f) = self.pred(&r, val, Env::Here)
        {
            self.fact(f);
        }
    }

    /// Refinement `r`'s predicate, of `val`, with its other names' values
    /// from `env`, if it can be said in the logic.
    fn pred(&mut self, r: &Refinement, val: &Val, env: Env) -> Option<Formula> {
        self.predicate(r, val, env).ok()
    }

    fn predicate(&mut self, r: &Refinement, val: &Val, env: Env) -> Result<Formula, Failure> {
        let mut all = Vec::new();
        for part in &r.parts {
            all.push(self.part(part, val, env)?);
        }
        // What its carried parts say of the values at their positions is
        // said of `val`'s objects (lemma A6).
        if r.has_carried()
            && let Some(base) = self.base_in(r, env)
        {
            let tree = self.carried_tree(r, env);
            self.assume_tree(val, &base, &tree);
        }
        Ok(Formula::and(all))
    }

    /// The type refinement `r` refines, with the type arguments `env` has.
    fn base_in(&self, r: &Refinement, env: Env) -> Option<Type> {
        let base = r.base.clone()?;
        let targs = match env {
            Env::Call(_, t) | Env::Fields(_, t) | Env::Post(_, _, t) => t,
            Env::Here | Env::Closed(_) => &[][..],
        };
        if targs.is_empty() {
            return Some(base);
        }
        let names: Vec<String> = targs.iter().map(|(n, _)| n.clone()).collect();
        let args: Vec<Type> = targs.iter().map(|(_, t)| t.clone()).collect();
        Some(
            self.cx
                .solved(&crate::types::Subst::new(&names, &args).apply(&base)),
        )
    }

    /// Refinement part `part`'s predicate, of each part of `val` it's of:
    /// of a variant's field, if `val` is that variant.
    fn part(&mut self, part: &Part, val: &Val, env: Env) -> Result<Formula, Failure> {
        // What every value at a position is (`targs`), which isn't a
        // formula of `val`'s parts: `predicate` and `need` say it.
        if part.carried {
            return Ok(Formula::True);
        }
        let mut all = Vec::new();
        for way in &part.ways {
            let mut guards = Vec::new();
            let mut v = val.clone();
            for &(variant, field) in way {
                let Val::Obj(o) = v else {
                    return Err(Failure::Unsupported);
                };
                let parts = self.parts.get(&o).ok_or(Failure::Unsupported)?;
                let n = i128::try_from(variant).expect("few variants");
                guards
                    .push(Formula::eq(&parts.tag, &Lin::constant(n)).ok_or(Failure::Unsupported)?);
                v = parts.fields[variant][field].clone();
            }
            let f = match self.term(&part.pred, &v, env)? {
                Val::Bool(f) => f,
                _ => return Err(Failure::Unsupported),
            };
            all.push(match guards.is_empty() {
                true => f,
                false => Formula::or([Formula::and(guards).not(), f]),
            });
        }
        Ok(Formula::and(all))
    }

    fn term(&mut self, e: &Expr, binder: &Val, env: Env) -> Result<Val, Failure> {
        let int = |v: Val| match v {
            Val::Int(l) => Ok(l),
            _ => Err(Failure::Unsupported),
        };
        let boolean = |v: Val| match v {
            Val::Bool(f) => Ok(f),
            _ => Err(Failure::Unsupported),
        };
        Ok(match &e.kind {
            ExprKind::Int(n, _) => Val::Int(Lin::constant(i128::from(*n))),
            ExprKind::Char(c) => Val::Int(Lin::constant(i128::from(u32::from(*c)))),
            ExprKind::Bool(b) => Val::Bool(if *b { Formula::True } else { Formula::False }),
            ExprKind::Var(..) => match (
                self.cx.pred_names.get(&e.id).ok_or(Failure::Unsupported)?,
                env,
            ) {
                (PredName::Binder, _) => binder.clone(),
                (_, Env::Closed(names)) => names.get(&e.id).ok_or(Failure::Unsupported)?.clone(),
                (PredName::Param(i), Env::Call(args, _)) => {
                    args.get(*i).ok_or(Failure::Unsupported)?.clone()
                }
                (PredName::Param(i), Env::Post(_, Some(now), _)) => {
                    now.get(*i).ok_or(Failure::Unsupported)?.clone()
                }
                (PredName::Param(i), Env::Here | Env::Post(_, None, _)) => {
                    let id = *self.params.get(*i).ok_or(Failure::Unsupported)?;
                    self.read_place(&Place::local(id))
                }
                (PredName::Local(id), _) => self.read_place(&Place::local(*id)),
                (PredName::Field(j), Env::Fields(vals, _)) => {
                    vals.get(*j).ok_or(Failure::Unsupported)?.clone()
                }
                (PredName::Param(_), Env::Fields(..)) | (PredName::Field(_), _) => {
                    return Err(Failure::Unsupported);
                }
                (
                    PredName::Const(name),
                    Env::Call(_, args) | Env::Fields(_, args) | Env::Post(_, _, args),
                ) => match args.iter().find(|(p, _)| p == name) {
                    Some((_, t)) => self.length(&t.clone()),
                    None => self.length(&Type::Param(name.clone(), false)),
                },
                (PredName::Const(name), Env::Here) => {
                    self.length(&Type::Param(name.clone(), false))
                }
                (PredName::Item(symbol), _) => {
                    let symbol = symbol.clone();
                    self.const_item(&symbol)
                }
            },
            ExprKind::Unary(UnOp::Neg, x) => Val::Int(
                int(self.term(x, binder, env)?)?
                    .scale(-1)
                    .ok_or(Failure::ArithmeticLimit)?,
            ),
            ExprKind::Unary(UnOp::Not, x) => Val::Bool(boolean(self.term(x, binder, env)?)?.not()),
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.term(l, binder, env)?, self.term(r, binder, env)?);
                self.logic_op(*op, a, b)?
            }
            // `old(x)`: `x` at the call; of a closed refinement, what its
            // names were then.
            ExprKind::Call(_, args) if args.len() == 1 => match env {
                Env::Post(old, _, targs) => self.term(&args[0], binder, Env::Call(old, targs))?,
                Env::Closed(_) => self.term(&args[0], binder, env)?,
                _ => return Err(Failure::Unsupported),
            },
            // A field of what's named, through the `*`s the checker found:
            // of a struct, as every value of one is, in the logic, its
            // fields' values.
            ExprKind::Field(x, _) => {
                let (derefs, i) = *self.cx.pred_fields.get(&e.id).ok_or(Failure::Unsupported)?;
                let mut v = self.term(x, binder, env)?;
                for _ in 0..derefs {
                    v = self.pointee(v);
                }
                match v {
                    Val::Struct(_, mut vals) if i < vals.len() => vals.swap_remove(i),
                    _ => return Err(Failure::Unsupported),
                }
            }
            // A measure of what's named, through the `*`s the checker
            // found: the `&self` it's given is the last reference.
            ExprKind::MethodCall(x, ..) => {
                let (symbol, derefs) = self
                    .cx
                    .pred_measures
                    .get(&e.id)
                    .ok_or(Failure::Unsupported)?
                    .clone();
                let v = self.term(x, binder, env)?;
                let recv = self.autoref(v, derefs, Some(false));
                self.measure(&symbol, recv)
            }
            _ => return Err(Failure::Unsupported),
        })
    }

    /// Fails with `what` at `span` unless refinement `r` holds of `val`,
    /// with names from `env`: each of its parts, which the error names.
    fn need(
        &mut self,
        r: &Refinement,
        val: &Val,
        env: Env,
        span: Span,
        what: String,
    ) -> Result<(), Error> {
        if r.has_carried()
            && let Some(base) = self.base_in(r, env)
        {
            let decl = self.written_tree(val, &base);
            self.need_carried(r, val, &base, env, Some(&decl), span, &what)?;
            // Proved of the values at each position where nothing can
            // write, which never change: so it's known of them from here on,
            // as an empty list's elements, which have every refinement, do
            // once the list's declared as having one. Not where something
            // can (behind a `&mut`): there it would say what may be written.
            let mut proved = self.carried_tree(r, env);
            proved.retain(|(pos, _)| {
                !matches!(
                    self.type_at(&base, pos),
                    Some((_, crate::borrowck::regions::Variance::Inv)) | None
                )
            });
            self.assume_tree(val, &base, &proved);
        }
        for part in &r.parts {
            if part.carried {
                continue;
            }
            let goal = self.part(part, val, env);
            let outcome = if !self.reachable {
                Ok(())
            } else {
                match &goal {
                    Ok(g) => self.proves(Some(g)),
                    Err(reason) => Err(*reason),
                }
            };
            let Err(reason) = outcome else {
                continue;
            };
            let failure = self.proof_failure(reason, goal.as_ref().ok(), Some(val), &part.text());
            self.failed(failure.attach(Error::ty(
                span,
                format!("can't prove `{}` for {what}", part.text()),
            )));
            // Assumed from here on, as if it had been proved.
            for part in &r.parts {
                if !part.carried
                    && let Ok(g) = self.part(part, val, env)
                {
                    self.fact(g);
                }
            }
            return Ok(());
        }
        Ok(())
    }

    /// `e`, that a refinement couldn't be proved, is reported, but checking
    /// goes on as if it had been: it's assumed from here on, as if proved,
    /// so no later failure is a consequence of this one, and each is worth
    /// saying (D1 of docs/62-t-chess-findings.md).
    pub(super) fn failed(&mut self, e: Error) {
        self.errors.push(e);
    }

    /// Fails at `span` unless the fields `vals` of a value of struct `t`
    /// have its refinements: all of them, or with `changed`, those about
    /// that field.
    fn need_fields(
        &mut self,
        t: &Type,
        vals: &[Val],
        changed: Option<usize>,
        span: Span,
    ) -> Result<(), Error> {
        let Some(fields) = self.refined_fields(t).cloned() else {
            return Ok(());
        };
        let which: Vec<usize> = match changed {
            Some(i) => fields.about(i).collect(),
            None => (0..fields.refs.len()).collect(),
        };
        for j in which {
            let Some(r) = &fields.refs[j] else {
                continue;
            };
            let mut what = format!("field `{}` of `{}`", fields.names[j], fields.name);
            if let Some(i) = changed
                && i != j
            {
                what += &format!(", which names `{}`", fields.names[i]);
            }
            let args = self.adt_args(t);
            self.need(r, &vals[j], Env::Fields(vals, &args), span, what)?;
        }
        Ok(())
    }

    /// `v`, the function's result, at `span`, must have its refinement,
    /// and what the function leaves what `ensures` says. Both are about the
    /// parameters as they are here, and with `old(..)`, at the call.
    fn result(&mut self, v: Val, span: Span) -> Result<(), Error> {
        if self.closures > 0 {
            return Ok(());
        }
        let entry = self.entry.clone();
        // What may be written through it must be what its type declares.
        if let Some(id) = self.cx.current
            && let Type::Fn(sig) = &self.cx.fns[id.0].scheme.ty
        {
            let ret = self.cx.solved(&sig.ret);
            let declared = match &self.ret {
                Some((r, _)) if r.has_carried() => {
                    self.carried_tree(&r.clone(), Env::Post(&entry, None, &[]))
                }
                _ => Vec::new(),
            };
            let name = self.cx.fns[id.0].name.name.clone();
            let what = format!("the result of `{name}`");
            self.keep_inv(&v, &ret, &declared, &[], span, &what)?;
            self.opaque_result(&v, &ret, span, &name);
        }
        if let Some((r, name)) = self.ret.clone() {
            let what = match self.cx.current.is_some_and(|id| self.cx.fns[id.0].konst) {
                true => format!("the value of const `{name}`"),
                false => format!("the result of `{name}`"),
            };
            self.need(&r, &v, Env::Post(&entry, None, &[]), span, what)?;
        }
        if let Some((ensures, name)) = self.ensures.clone() {
            for r in &ensures {
                let what = format!("what `{name}` ensures");
                self.need(r, &Val::Other, Env::Post(&entry, None, &[]), span, what)?;
            }
        }
        Ok(())
    }

    /// `v`, with what each reference in it points to read now, `depth`
    /// references deep, rather than when it's needed.
    fn settled(&mut self, v: Val, depth: usize) -> Val {
        match v {
            Val::Ref(o, Pointee::Place(q)) if depth > 0 => {
                let to = self.read_place(&q);
                let to = self.settled(to, depth - 1);
                Val::Ref(o, Pointee::Val(Box::new(to)))
            }
            Val::Struct(o, vals) if depth > 0 => {
                let vals = vals
                    .into_iter()
                    .map(|v| self.settled(v, depth - 1))
                    .collect();
                Val::Struct(o, vals)
            }
            v => v,
        }
    }

    /// The state to run each branch from.
    fn snapshot(&self) -> Snap {
        Snap {
            facts: self.facts.len(),
            cur: self.cur.clone(),
            objs: self.objs.clone(),
            reachable: self.reachable,
            declared: self.declared.len(),
            invalidated: self.invalidated.clone(),
        }
    }

    /// Goes back to `start`.
    fn restore(&mut self, start: &Snap) {
        self.facts.truncate(start.facts);
        self.cur = start.cur.clone();
        self.objs = start.objs.clone();
        self.reachable = start.reachable;
        self.declared.truncate(start.declared);
        self.invalidated = start.invalidated.clone();
    }

    /// Where paths `outcomes`, all from `start`, meet: what's known is
    /// that one of them was taken, and each place is whichever value the
    /// path taken gave it. A place a path gave a new value nothing's
    /// known of, or never read, is nothing known in that one. Of each of
    /// `values`, a type and the value each path gives it, in order, gives
    /// what it is after.
    fn join_with(
        &mut self,
        start: &Snap,
        outcomes: Vec<Outcome>,
        values: Vec<(Type, Vec<Val>)>,
    ) -> Vec<Val> {
        self.restore(start);
        let taken: Vec<bool> = outcomes.iter().map(|o| o.reachable).collect();
        let reached: Vec<Outcome> = outcomes.into_iter().filter(|o| o.reachable).collect();
        let values: Vec<(Type, Vec<Val>)> = (values.into_iter())
            .map(|(t, vals)| {
                let vals = vals.into_iter().zip(&taken).filter(|(_, k)| **k);
                (t, vals.map(|(v, _)| v).collect())
            })
            .collect();
        if reached.is_empty() {
            self.reachable = false;
            return values.iter().map(|(t, _)| self.unknown(t)).collect();
        }
        let in_scope: HashSet<NodeId> = self.declared.iter().copied().collect();
        if let [_] = &reached[..] {
            let o = reached.into_iter().next().expect("one");
            self.facts.extend(o.facts);
            self.cur = o.cur;
            self.cur.retain(|p, _| in_scope.contains(&p.root));
            self.objs = o.objs;
            self.objs.retain(|p, _| in_scope.contains(&p.root));
            self.invalidated = o.invalidated;
            return values.into_iter().map(|(_, mut v)| v.remove(0)).collect();
        }
        // A place is the same object after, if every branch left it so;
        // else it's one nothing's known of.
        let mut objs = reached[0].objs.clone();
        objs.retain(|p, o| {
            in_scope.contains(&p.root) && reached.iter().all(|r| r.objs.get(p) == Some(o))
        });
        let mut places: Vec<&Place> = (reached.iter())
            .flat_map(|o| o.cur.keys())
            .filter(|p| in_scope.contains(&p.root))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        places.sort();
        let mut cur = HashMap::new();
        let mut new: Vec<(Var, Vec<Option<Var>>)> = Vec::new();
        for p in places {
            let vars: Vec<Option<Var>> = reached.iter().map(|o| o.cur.get(p).copied()).collect();
            match vars[0] {
                Some(v) if vars.iter().all(|w| *w == Some(v)) => {
                    cur.insert(p.clone(), v);
                }
                _ => {
                    let had = vars.iter().flatten().next().expect("in one");
                    let n = self.fresh(self.kinds[*had as usize].clone());
                    cur.insert(p.clone(), n);
                    new.push((n, vars));
                }
            }
        }
        // What's said of positions of each branch's objects is said, under
        // the condition the branch was taken, of the one after (`targs`):
        // of a reference, what may be written through it is whichever
        // place's that branch's points to.
        let guards: Vec<Formula> = reached
            .iter()
            .map(|o| Formula::and(o.facts.clone()))
            .collect();
        let mut tree_joins: Vec<(Obj, Type, Vec<Option<Obj>>)> = Vec::new();
        // A place each branch left a different object at is a new one, of
        // which each measure every branch knows of its object is, in each
        // case, what it was of that branch's: a measure is a function of
        // the value.
        let mut joined: Vec<(Val, Vec<Val>)> = Vec::new();
        let mut differ: Vec<&Place> = (reached[0].objs.keys())
            .filter(|p| in_scope.contains(&p.root) && !objs.contains_key(*p))
            .filter(|p| reached.iter().all(|r| r.objs.contains_key(*p)))
            .collect();
        differ.sort();
        for p in differ {
            let each: Vec<Obj> = reached.iter().map(|r| r.objs[p]).collect();
            let mut symbols: Vec<String> = (self.measured.keys())
                .filter(|(o, _)| *o == each[0])
                .map(|(_, m)| m.clone())
                .filter(|m| {
                    each.iter()
                        .all(|o| self.measured.contains_key(&(*o, m.clone())))
                })
                .collect();
            symbols.sort();
            let o = self.new_obj();
            for m in symbols {
                let vals: Vec<Val> = (each.iter())
                    .map(|b| self.measured[&(*b, m.clone())].clone())
                    .collect();
                if !vals.iter().all(|v| matches!(v, Val::Int(_) | Val::Bool(_))) {
                    continue;
                }
                let ret = self.cx.measures[&m].ret.clone();
                let v = self.measure_value(&ret);
                self.measured.insert((o, m), v.clone());
                joined.push((v, vals));
            }
            // An enum's parts are what they were in each branch's.
            if each.iter().all(|b| self.parts.contains_key(b)) {
                let t = self.place_ty(p);
                self.enum_parts(o, &t, None);
                let vals = each.iter().map(|b| Val::Obj(*b)).collect();
                joined.push((Val::Obj(o), vals));
            }
            let t = self.place_ty(p);
            tree_joins.push((o, t, each.iter().map(|b| Some(*b)).collect()));
            objs.insert(p.clone(), o);
        }
        let results: Vec<Val> = values.iter().map(|(t, _)| self.unknown(t)).collect();
        for (o, t, each) in tree_joins {
            self.join_trees(o, &t, &each, &guards);
        }
        for (result, (t, vals)) in results.iter().zip(&values) {
            self.join_values(result, t, vals, &guards);
        }
        let mut cases = Vec::new();
        for (i, o) in reached.into_iter().enumerate() {
            for event in o.invalidated {
                if !self.invalidated.contains(&event) {
                    self.invalidated.push(event);
                }
            }
            let mut parts = o.facts;
            for (result, (_, vals)) in results.iter().zip(&values) {
                parts.extend(self.same(result, &vals[i]));
            }
            for (n, vars) in &new {
                if let Some(v) = vars[i] {
                    parts.extend(equal(&self.var(*n), &self.var(v)));
                }
            }
            for (v, vals) in &joined {
                parts.extend(self.same(v, &vals[i]));
            }
            cases.push(Formula::and(parts));
        }
        self.fact(Formula::or(cases));
        self.cur = cur;
        self.objs = objs;
        results
    }

    /// That `a` and `b` are equal, as far as the logic can say: `equal`,
    /// and of enums' values, their parts (see `Parts`), which are
    /// functions of the value.
    fn same(&mut self, a: &Val, b: &Val) -> Vec<Formula> {
        let mut out = match (a, b) {
            (Val::Obj(x), Val::Obj(y)) => {
                let (Some(p), Some(q)) = (self.parts.get(x).cloned(), self.parts.get(y).cloned())
                else {
                    return self.same_measures(*x, *y);
                };
                let mut out: Vec<Formula> = Formula::eq(&p.tag, &q.tag).into_iter().collect();
                for (f, g) in p.fields.iter().zip(&q.fields) {
                    for (a, b) in f.iter().zip(g) {
                        out.extend(self.same(a, b));
                    }
                }
                out
            }
            (Val::Struct(_, a), Val::Struct(_, b)) => {
                let mut out = Vec::new();
                for (a, b) in a.iter().zip(b) {
                    out.extend(self.same(a, b));
                }
                out
            }
            _ => equal(a, b),
        };
        if let (
            Val::Obj(x) | Val::Struct(x, _) | Val::Ref(x, _),
            Val::Obj(y) | Val::Struct(y, _) | Val::Ref(y, _),
        ) = (a, b)
        {
            out.extend(self.same_measures(*x, *y));
        }
        out
    }

    /// That objects `x` and `y`, which are equal, have the same measures:
    /// each `y` has, which `x` then has, as a measure is a function of the
    /// value (a slice's, of the reference).
    fn same_measures(&mut self, x: Obj, y: Obj) -> Vec<Formula> {
        if x == y {
            return Vec::new();
        }
        let mut symbols: Vec<String> = (self.measured.keys())
            .filter(|(o, _)| *o == y)
            .map(|(_, m)| m.clone())
            .collect();
        symbols.sort();
        let mut out = Vec::new();
        for m in symbols {
            let w = self.measured[&(y, m.clone())].clone();
            let v = match self.measured.get(&(x, m.clone())) {
                Some(v) => v.clone(),
                None => {
                    let ret = self.cx.measures[&m].ret.clone();
                    let v = self.measure_value(&ret);
                    self.measured.insert((x, m), v.clone());
                    v
                }
            };
            out.extend(equal(&v, &w));
        }
        out
    }

    /// The head of a loop at `span` that changes `changed`, with invariant
    /// `inv`, from the paths into it from outside: the invariant holds as
    /// the loop starts; then each place the loop may change is forgotten,
    /// but for the measures its frame keeps (`frames`, from `frames_of`),
    /// and the invariant is assumed of them.
    fn loop_start(
        &mut self,
        changed: &[Invalidation],
        frames: &[(Place, Vec<String>)],
        inv: Option<&Expr>,
        span: Span,
    ) -> Result<(), Error> {
        let inv = inv.map(|p| Refinement::whole(p.clone()));
        if let Some(r) = &inv {
            let span = r.parts[0].pred.span;
            let what = "the loop's invariant, as the loop starts".to_string();
            self.need(r, &Val::Other, Env::Here, span, what)?;
        }
        self.forget(changed, frames, span);
        if let Some(r) = &inv
            && let Some(f) = self.pred(r, &Val::Other, Env::Here)
        {
            self.fact(f);
        }
        Ok(())
    }

    /// Each place the loop may change has a value nothing's known of but
    /// its refinements, and the measures `frames` says the loop keeps of
    /// it: those are what they were. Keep why its entry facts cannot be
    /// assumed.
    fn forget(
        &mut self,
        changes: &[Invalidation],
        frames: &[(Place, Vec<String>)],
        loop_span: Span,
    ) {
        let kept = self.measures_now(frames);
        for change in changes {
            self.invalidate_loop(change, loop_span);
            self.kill(change.place());
        }
        self.measures_kept(kept);
    }

    /// The value of each measure `frames` names of its place, now: before
    /// what changes the place, which keeps them (`measures_kept`).
    fn measures_now(&mut self, frames: &[(Place, Vec<String>)]) -> Vec<(Place, String, Val)> {
        let mut kept = Vec::new();
        for (p, measures) in frames {
            for m in measures {
                let recv = Val::Ref(self.new_obj(), Pointee::Place(p.clone()));
                kept.push((p.clone(), m.clone(), self.measure(m, recv)));
            }
        }
        kept
    }

    /// Each measure in `kept` of its place is what it was
    /// (`measures_now`): what changed the place since kept it.
    fn measures_kept(&mut self, kept: Vec<(Place, String, Val)>) {
        for (p, m, before) in kept {
            let recv = Val::Ref(self.new_obj(), Pointee::Place(p));
            let now = self.measure(&m, recv);
            for f in equal(&now, &before) {
                self.fact(f);
            }
        }
    }

    /// The measures a loop keeps of each place it changes, `changed`:
    /// its *frame*. Of the loop's `changes`, each a place changed with
    /// the measures of places in it the change keeps (`Keeps`): nothing,
    /// for an assignment; for a place lent straight to a call-scoped
    /// `&mut`, the measures the call keeps of it (`keeps_at`); for one a
    /// closure captures, those its body keeps of each place in it
    /// (`closure_frames`). A measure of place `p` is kept if every change
    /// in the loop to `p`, to a place in it, or to one it's in, keeps it
    /// of `p`. By induction on how many times round the loop has been, at
    /// the head, and anywhere in the body, the measure is what it was
    /// before the loop: each change since was one keeping it, which
    /// nothing else could see or write `p` during (it's lent, and the
    /// borrow checker lets nothing else have it while it is), and a
    /// measure is of the value alone (lemma F1 of docs/52-loop-frames.md).
    /// The same of a closure's body, by induction on how many times it's
    /// run (lemma F2 of docs/69-loans.md).
    fn frames_of(
        &mut self,
        changes: &[(Place, Keeps)],
        changed: &[Invalidation],
    ) -> Vec<(Place, Vec<String>)> {
        let mut places: Vec<&Place> = changed.iter().map(|c| c.place()).collect();
        places.sort();
        places.dedup();
        let mut frames = Vec::new();
        let none = Vec::new();
        for p in places {
            let mut kept: Option<Vec<String>> = None;
            for (q, keeps) in changes {
                if !q.within(p) && !p.within(q) {
                    continue;
                }
                let keeps = (keeps.iter().find(|(f, _)| f == p)).map_or(&none, |(_, ms)| ms);
                kept = Some(match kept {
                    None => keeps.clone(),
                    Some(k) => k.into_iter().filter(|m| keeps.contains(m)).collect(),
                });
            }
            if let Some(k) = kept.filter(|k| !k.is_empty()) {
                frames.push((p.clone(), k));
            }
        }
        frames
    }

    /// The measures of what a call of `symbol` (resolved) at type
    /// arguments `targs`, with arguments of types `tys` (`None` for one
    /// whose type isn't known), lends its argument `i` to a call-scoped
    /// `&mut` its postconditions say it leaves as they were, for every
    /// value of its other arguments: the postconditions are assumed of new
    /// objects for what's behind the `&mut` before and after, and new
    /// values for the other arguments, which nothing's known of, and each
    /// measure they ask of what's there after must be proved equal to the
    /// one before. Those are what any call it makes at any time keeps:
    /// what the callee proved, and the variables here are new, so what's
    /// known here says nothing of them.
    fn keeps_at(
        &mut self,
        symbol: &str,
        targs: Vec<Type>,
        tys: &[Option<Type>],
        i: usize,
    ) -> Option<Vec<String>> {
        let sig = self.cx.refined.get(symbol)?.clone();
        if !sig.scoped.get(i).copied().unwrap_or(false) || sig.ensures.is_empty() {
            return None;
        }
        let targs: Vec<(String, Type)> = (sig.type_params.iter().cloned()).zip(targs).collect();
        let (before, after) = (self.new_obj(), self.new_obj());
        let behind =
            |w: &mut Self, o: Obj| Val::Ref(w.new_obj(), Pointee::Val(Box::new(Val::Obj(o))));
        let mut old = Vec::new();
        let mut now = Vec::new();
        for (j, t) in tys.iter().enumerate() {
            if j == i {
                old.push(behind(self, before));
                now.push(behind(self, after));
            } else {
                let v = match t {
                    None => Val::Other,
                    Some(t) => self.unknown(t),
                };
                old.push(v.clone());
                now.push(v);
            }
        }
        let start = self.facts.len();
        let reachable = std::mem::replace(&mut self.reachable, true);
        for r in &sig.ensures {
            if let Some(f) = self.pred(r, &Val::Other, Env::Post(&old, Some(&now), &targs)) {
                self.fact(f);
            }
        }
        let asked: Vec<(String, Val)> = (self.measured.iter())
            .filter(|((o, _), _)| *o == after)
            .map(|((_, m), v)| (m.clone(), v.clone()))
            .collect();
        let mut kept = Vec::new();
        for (m, v) in asked {
            if self.cx.measures[&m].unsized_self {
                continue;
            }
            let recv = behind(self, before);
            let was = self.measure(&m, recv);
            let same = equal(&v, &was);
            if !same.is_empty() && same.iter().all(|f| self.proves(Some(f)).is_ok()) {
                kept.push(m);
            }
        }
        self.facts.truncate(start);
        self.reachable = reachable;
        kept.sort();
        Some(kept)
    }

    /// `l op r`, an operator at node `site` on integers of type `ty`:
    /// proves it can't panic if it can, and gives its value, which it has
    /// if it didn't panic.
    fn arith(&mut self, site: NodeId, op: BinOp, l: Val, r: Val, ty: &Type) -> Val {
        if op.bitwise() {
            return self.bits(site, op, l, r, ty);
        }
        let (Val::Int(a), Val::Int(b), Some((min, max))) = (l, r, self.range(ty)) else {
            if self.reachable {
                self.operator_proof(
                    site,
                    None,
                    Failure::Unsupported,
                    &format!("`{op}` must be safe for `{ty}`"),
                );
            }
            return self.unknown(ty);
        };
        let in_range = |l: &Lin| Some(Formula::and([Formula::le(&min, l)?, Formula::le(l, &max)?]));
        // Why the value isn't in the logic, if it isn't: a product of
        // three variables isn't, nor a number too big for it.
        let mut missing = Failure::ArithmeticLimit;
        let (value, goal) = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                let value = match op {
                    BinOp::Add => a.add(&b),
                    BinOp::Sub => a.sub(&b),
                    _ => logic::times(&a, &b, &mut self.kinds)
                        .map_err(|f| missing = f)
                        .ok(),
                };
                let goal = value.as_ref().and_then(in_range);
                (value, goal)
            }
            // Not by zero, nor the least value by -1. Whatever else is
            // divided fits: the quotient is no further from 0 than what's
            // divided, but for the least value of a signed type by -1, which
            // in each is one past its greatest. So this suffices of every
            // integer type, and so of a type parameter's, though the logic
            // knows of none of them what its greatest value is, but that
            // it's at least -1 minus its least: it holds in every one.
            _ => {
                let goal = (|| {
                    let zero = Formula::eq(&b, &Lin::constant(0))?;
                    let overflow = Formula::and([
                        Formula::eq(&a, &min)?,
                        Formula::eq(&b, &Lin::constant(-1))?,
                    ]);
                    Some(Formula::and([zero.not(), overflow.not()]))
                })();
                (None, goal)
            }
        };
        let condition = if matches!(op, BinOp::Div | BinOp::Rem) {
            format!("the divisor must be nonzero and `{op}` must not overflow `{ty}`")
        } else {
            format!("the result of `{op}` must fit in `{ty}`")
        };
        self.operator_proof(site, goal, missing, &condition);
        match value {
            Some(v) => Val::Int(v),
            // By a constant, what a predicate's `/` or `%` is (`divide`).
            None if let Some(v) = b.as_constant().and_then(|k| self.divide(op, &a, k)) => {
                Val::Int(v)
            }
            None => {
                let result = self.unknown(ty);
                if let Val::Int(q) = &result
                    && op == BinOp::Rem
                    && let Some(f) = remainder(&a, &b, q)
                {
                    self.fact(f);
                } else if let Val::Int(q) = &result
                    && op == BinOp::Div
                    && let Some(f) = self.quotient(&a, &b, q)
                {
                    self.fact(f);
                }
                result
            }
        }
    }

    /// What `q = a / b` says of `q · b`, for a divisor that isn't a
    /// constant (`divide` has those), when the signs of `a` and `b` are
    /// known here: it's within one `b` of `a`, on the side of zero, as
    /// division rounds towards zero. `a = q·b + r`, where `r` has the sign
    /// of `a` and is smaller than `b`: for `a >= 0` and `b > 0`, `q·b <= a
    /// <= q·b + b - 1`. A product: what the logic knows of it is in
    /// `logic::lemmas`.
    fn quotient(&mut self, a: &Lin, b: &Lin, q: &Lin) -> Option<Formula> {
        let zero = Lin::constant(0);
        let a_sign = if self.proves(Formula::le(&zero, a).as_ref()).is_ok() {
            1
        } else if self.proves(Formula::le(a, &zero).as_ref()).is_ok() {
            -1
        } else {
            return None;
        };
        let b_sign = if self.proves(Formula::lt(&zero, b).as_ref()).is_ok() {
            1
        } else if self.proves(Formula::lt(b, &zero).as_ref()).is_ok() {
            -1
        } else {
            return None;
        };
        let qb = logic::times(q, b, &mut self.kinds).ok()?;
        // |r| <= |b| - 1, as a sum: b - 1 when b > 0, -b - 1 when b < 0.
        let most = b.scale(b_sign)?.plus(-1)?;
        // q has the sign of a·b, or is 0.
        let q_sign = if a_sign == b_sign {
            Formula::le(&zero, q)?
        } else {
            Formula::le(q, &zero)?
        };
        Some(if a_sign > 0 {
            // q·b <= a <= q·b + |b| - 1
            Formula::and([
                q_sign,
                Formula::le(&qb, a)?,
                Formula::le(a, &qb.add(&most)?)?,
            ])
        } else {
            // q·b - |b| + 1 <= a <= q·b
            Formula::and([
                q_sign,
                Formula::le(&qb.sub(&most)?, a)?,
                Formula::le(a, &qb)?,
            ])
        })
    }

    /// `l op r` for a bitwise operator or a shift at node `site`, on
    /// integers of type `ty`. Only `<<` can panic: by a constant it's
    /// multiplying, so linear, and proved not to as `*` is; by anything
    /// else it isn't proved. The others aren't linear, but for `>>` by a
    /// constant, so their result is a new value, of which the logic knows
    /// what `bitwise` says.
    fn bits(&mut self, site: NodeId, op: BinOp, l: Val, r: Val, ty: &Type) -> Val {
        let (Val::Int(a), Val::Int(b)) = (l, r) else {
            return self.unknown(ty);
        };
        if op == BinOp::Shl {
            let shifted = b
                .as_constant()
                .filter(|k| (0..=MAX_SHIFT).contains(k))
                .and_then(|k| a.scale(1 << k));
            let Some(v) = shifted else {
                if self.reachable {
                    let reason = if b
                        .as_constant()
                        .is_some_and(|k| (0..=MAX_SHIFT).contains(&k))
                    {
                        Failure::ArithmeticLimit
                    } else {
                        Failure::Unsupported
                    };
                    self.operator_proof(
                        site,
                        None,
                        reason,
                        "the shifted value must fit in its integer type",
                    );
                }
                return self.unknown(ty);
            };
            let goal = self.fits(&v, ty);
            self.operator_proof(
                site,
                goal,
                Failure::ArithmeticLimit,
                &format!("the shifted value must fit in `{ty}`"),
            );
            return Val::Int(v);
        }
        let result = self.unknown(ty);
        if let Val::Int(q) = &result
            && let Some(f) = bitwise(op, &a, &b, q)
        {
            self.fact(f);
        }
        result
    }

    /// That `v` is a value of `ty`, if it's known to be, as a goal: for
    /// `-x` and `x as T`, whose values are what's in ℤ.
    fn fits(&mut self, v: &Lin, ty: &Type) -> Option<Formula> {
        let (min, max) = self.range(ty)?;
        Some(Formula::and([Formula::le(&min, v)?, Formula::le(v, &max)?]))
    }

    /// The function a call of `symbol` at type arguments `targs` calls:
    /// an impl's method, if it's a trait's and the impl is known
    /// (`select`), with that impl's type arguments, then the method's.
    fn callee_at(&self, symbol: &str, targs: Vec<Type>) -> (String, Vec<Type>) {
        let fallback = || (symbol.to_string(), targs.clone());
        let cx = self.cx;
        let Some((tr, m)) = (cx.traits.traits.iter())
            .find_map(|(tr, d)| Some((tr, d.methods.iter().find(|m| m.symbol == symbol)?)))
        else {
            return fallback();
        };
        let Some(self_ty) = targs.first() else {
            return fallback();
        };
        // Its type parameters, and projections and `impl` types, are rigid
        // to `select`, so the impl it finds for a type that's one of those
        // inside one that isn't is the one for every type they can be: any
        // other would overlap it, which the overlap check, and the orphan
        // rule across packages, rule out (lemma A7 of
        // docs/60-refined-type-arguments.md). A type not yet worked out
        // could be anything.
        fn unknown(t: &Type) -> bool {
            matches!(t, Type::Var(_) | Type::Inferred(..)) || t.parts().any(unknown)
        }
        let head_open = matches!(
            self_ty,
            Type::Param(_, _) | Type::Proj(..) | Type::Opaque(..)
        );
        if head_open || unknown(self_ty) {
            return fallback();
        }
        let Some((i, args)) = cx.traits.select(self_ty, Some(tr), &[]) else {
            return fallback();
        };
        let Some((_, method)) = cx.traits.impls[i]
            .methods
            .iter()
            .find(|(n, _)| *n == m.name)
        else {
            // The trait's default body, which says what the trait says.
            return fallback();
        };
        let mut all = args;
        all.extend(targs.iter().skip(1).cloned());
        (method.clone(), all)
    }

    /// The value of length `n`: a `Nat`'s number, or a const parameter's
    /// variable, one for the whole function, as the caller picks it once;
    /// or a const item's value, if it's known, and else a variable of its
    /// own the same way.
    fn length(&mut self, n: &Type) -> Val {
        let usize = Type::Int(crate::types::IntTy::Usize);
        match n {
            Type::Nat(k) => Val::Int(Lin::constant(i128::from(*k))),
            Type::Const(symbol) => self.const_item(symbol),
            Type::Param(name, _) => {
                if let Some(v) = self.consts.get(name) {
                    return v.clone();
                }
                let v = self.unknown(&usize);
                self.consts.insert(name.clone(), v.clone());
                v
            }
            _ => self.unknown(&usize),
        }
    }

    /// The value of const `symbol`, wherever it's named, in a body, a
    /// length or a predicate: what's known of it (`Known`), or else one
    /// variable for the whole function, of the const's type, of which what
    /// its type's refinement says holds.
    fn const_item(&mut self, symbol: &str) -> Val {
        match self.cx.const_values.get(symbol) {
            Some(Known::Int(k)) => return Val::Int(Lin::constant(*k)),
            Some(Known::Bool(true)) => return Val::Bool(Formula::True),
            Some(Known::Bool(false)) => return Val::Bool(Formula::False),
            None => {}
        }
        // Not a name a const parameter can have.
        let key = format!("const {symbol}");
        if let Some(v) = self.consts.get(&key) {
            return v.clone();
        }
        let ty = match self
            .cx
            .fns
            .iter()
            .find(|f| f.symbol == symbol)
            .map(|f| &f.scheme.ty)
        {
            Some(Type::Fn(sig)) => (*sig.ret).clone(),
            _ => return Val::Other,
        };
        let v = self.unknown(&ty);
        // It holds everywhere in the function, not only on the path where
        // the const is first named, which a branch would forget.
        if let Some(r) = self.cx.refined.get(symbol).and_then(|s| s.ret.clone())
            && let Some(f) = self.pred(&r, &v, Env::Post(&[], None, &[]))
        {
            self.laws.push(f);
        }
        self.consts.insert(key, v.clone());
        v
    }

    /// What struct type `t`'s type parameters are, by name.
    fn adt_args(&self, t: &Type) -> Vec<(String, Type)> {
        match t {
            Type::Adt(symbol, _, args) => match self.cx.adts.get(symbol) {
                Some(d) => d.params.iter().cloned().zip(args.iter().cloned()).collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Fails if `e`, which names function `symbol` but doesn't call it,
    /// names one with refined parameters.
    fn as_value(&self, symbol: &str, span: Span) -> Result<(), Error> {
        let Some(sig) = self.cx.refined.get(symbol) else {
            return Ok(());
        };
        if let Some(i) = sig.params.iter().position(Option::is_some) {
            return Err(Error::ty(
                span,
                format!(
                    "`{}` can't be used as a value: its parameter `{}` has a refinement, which \
                     a call through a value wouldn't check",
                    sig.name, sig.names[i]
                ),
            ));
        }
        Ok(())
    }

    /// A call at `site`, with arguments whose values are `vals` (the
    /// receiver first, for a method) at `places`: checks its refined
    /// parameters, and gives its result, and what it has yet to give once
    /// what it's lent is forgotten (`post_now`).
    fn call_at(
        &mut self,
        site: &CallAt,
        vals: Vec<Val>,
        places: Vec<Option<Place>>,
    ) -> Result<(Val, Post), Error> {
        let ty = site.ty.clone();
        let resolved = site.resolved.clone();
        let symbol = resolved.as_ref().map(|(s, _)| s.as_str());
        let sig = symbol.and_then(|s| self.cx.refined.get(s)).cloned();
        let targs: Vec<(String, Type)> = match (&sig, &resolved) {
            (Some(sig), Some((_, targs))) => {
                sig.type_params.iter().cloned().zip(targs.clone()).collect()
            }
            _ => Vec::new(),
        };
        if let Some(sig) = &sig {
            for (i, r) in sig.params.iter().enumerate() {
                if let (Some(r), Some(span)) = (r, site.spans.get(i))
                    && let Some(r) = r.ways_only()
                {
                    let what = format!("argument `{}` of `{}`", sig.names[i], sig.name);
                    self.need(&r, &vals[i], Env::Call(&vals, &targs), *span, what)?;
                }
            }
        }
        // A call through a value: nothing keeps what's at an invariant
        // position of what it's given, so nothing may be declared there.
        if resolved.is_none() && self.reachable {
            for (j, t) in site.tys.iter().enumerate() {
                let (Some(v), p) = (vals.get(j), places.get(j).cloned().flatten()) else {
                    continue;
                };
                let written = match &p {
                    Some(p) => targs::under(&self.place_decl(p), 0),
                    None => self.written_tree(v, t),
                };
                if self.declared_inv(t, &written) {
                    return Err(Error::ty(
                        site.spans[j],
                        format!(
                            "can't give `{}` to a call through a value: what's behind it has \
                             refinements of type arguments, which the call could write past",
                            site.texts.get(j).map_or("it", |t| t.as_str())
                        ),
                    ));
                }
            }
        }
        // Refinements of type arguments: what the call needs of them, and
        // says of its result and of what it's lent (`targs`).
        let trees = match &resolved {
            Some((s, t)) if self.reachable => self.call_trees(
                s,
                t,
                sig.as_ref(),
                &vals,
                &places,
                &site.spans,
                &site.coerced,
                site.span,
            )?,
            _ => targs::CallTrees::default(),
        };
        if ty == Type::Never {
            self.reachable = false;
        }
        // A measure is what it measures.
        if let Some(s) = symbol
            && self.cx.measures.contains_key(s)
            && let Some(recv) = vals.first()
        {
            let post = Post {
                name: String::new(),
                sig: None,
                old: Vec::new(),
                scoped: Vec::new(),
                targs: Vec::new(),
                result: Val::Other,
                lent: Vec::new(),
            };
            return Ok((self.measure(s, recv.clone()), post));
        }
        let mut result = self.unknown(&ty);
        if let Some(s) = symbol
            && self.reachable
        {
            result = self.free_theorems(s, &vals, result);
        }
        self.assume_tree(&result, &ty, &trees.result);
        // What the two intrinsics that say how big an integer type is give.
        if let (Some(f @ ("core::num::least" | "core::num::greatest")), Val::Int(v)) =
            (symbol, &result)
            && let Some((lo, hi)) = self.range(&ty)
            && let Some(f) = Formula::eq(v, if f.ends_with("least") { &lo } else { &hi })
        {
            self.fact(f);
        }
        // What it ensures, and its result's refinement, are about what it
        // leaves behind what it's lent, which is learned once that's
        // forgotten, where it returns (`Walk::post`); and about the
        // arguments as they are now, which are kept.
        let scoped = match symbol {
            Some(s) => self.cx.scoped_params(s),
            None => Vec::new(),
        };
        let scoped: Vec<Option<Place>> = places
            .into_iter()
            .enumerate()
            .map(|(i, p)| p.filter(|_| scoped.get(i) == Some(&true)))
            .collect();
        // The struct a field lent is in is whole until now, so it has its
        // refinements: what's known of its other fields, which the call
        // can't change, proves them again after.
        for p in scoped.iter().flatten() {
            for (q, _) in self.constrained_around(p) {
                let t = self.place_ty(&q);
                self.fields_of(&q, &t);
            }
        }
        let old = match sig.is_some() {
            true => vals
                .into_iter()
                .map(|v| self.settled(v, ENTRY_DEPTH))
                .collect(),
            false => Vec::new(),
        };
        let name = match symbol {
            Some(s) => s.rsplit("::").next().unwrap_or(s).to_string(),
            None => "the function".to_string(),
        };
        let post = Post {
            name,
            sig,
            old,
            scoped,
            targs,
            result: result.clone(),
            lent: trees.lent,
        };
        Ok((result, post))
    }

    /// `result`, what a call of function `symbol` with arguments `vals`
    /// gives, with what its free type parameters say of it (lemma P1 of
    /// docs/55-free-theorems.md): each value of one in it is one of those
    /// in the arguments, of the variants they're of. So each part of
    /// `result` a way gets to, if it's there, is one of the parts the
    /// arguments' ways get to that are there: the one, if there's one,
    /// which it then is, as an object is one value; and none if there's
    /// none, so it isn't there. If a part of an argument can't be followed,
    /// nothing is said of the parts of the result it could be.
    fn free_theorems(&mut self, symbol: &str, vals: &[Val], mut result: Val) -> Val {
        let free = self.cx.free_of(symbol);
        for p in &free {
            let candidates: Option<Vec<(Vec<Formula>, Val)>> = (p.params.iter())
                .map(|(i, way)| self.follow(vals.get(*i)?, way))
                .collect();
            let Some(candidates) = candidates else {
                continue;
            };
            for way in &p.result {
                self.free_part(&mut result, way, &candidates);
            }
        }
        result
    }

    /// The part of `v` that `way` gets to, with the conditions that it's
    /// there: that each enum on the way is that variant. `None` if it isn't
    /// known what `v` is made of.
    fn follow(&self, v: &Val, way: &[(String, usize)]) -> Option<(Vec<Formula>, Val)> {
        let mut guards = Vec::new();
        let mut v = v.clone();
        for (variant, field) in way {
            let Val::Obj(o) = v else {
                return None;
            };
            let parts = self.parts.get(&o)?;
            let k = self.variant_index(&parts.symbol, variant)?;
            let n = i128::try_from(k).expect("few variants");
            guards.push(Formula::eq(&parts.tag, &Lin::constant(n))?);
            v = parts.fields[k].get(*field)?.clone();
        }
        Some((guards, v))
    }

    /// What a free theorem says of the part of `result` that `way` gets to,
    /// whose value is one of `candidates`, each with the conditions that
    /// it's there (see `free_theorems`).
    fn free_part(
        &mut self,
        result: &mut Val,
        way: &[(String, usize)],
        candidates: &[(Vec<Formula>, Val)],
    ) {
        let Some((last, rest)) = way.split_last() else {
            // The whole result: the call returns, so it's one of them.
            match candidates {
                [] => self.reachable = false,
                [(guards, v)] => {
                    for g in guards {
                        self.fact(g.clone());
                    }
                    *result = v.clone();
                }
                _ => {
                    let f = self.one_of(result, candidates);
                    self.fact(f);
                }
            }
            return;
        };
        // The enum the part is a field of, and the conditions it's there.
        let Some((mut guards, Val::Obj(o))) = self.follow(result, rest) else {
            return;
        };
        let Some(parts) = self.parts.get(&o) else {
            return;
        };
        let (variant, field) = last;
        let Some(k) = self.variant_index(&parts.symbol, variant) else {
            return;
        };
        let n = i128::try_from(k).expect("few variants");
        let Some(tag) = Formula::eq(&parts.tag, &Lin::constant(n)) else {
            return;
        };
        guards.push(tag);
        let Some(part) = parts.fields[k].get(*field).cloned() else {
            return;
        };
        let there = Formula::and(guards);
        let f = match candidates {
            // It can't be there.
            [] => there.not(),
            // It's that value, whenever it's there: an object is one value,
            // what isn't there is no value, and so can be any.
            [(g, v)] => {
                if let Some(p) = self.parts.get_mut(&o) {
                    p.fields[k][*field] = v.clone();
                }
                Formula::or([there.not(), Formula::and(g.clone())])
            }
            _ => Formula::or([there.not(), self.one_of(&part, candidates)]),
        };
        self.fact(f);
    }

    /// That `v` is one of `candidates`, there: the disjunction of each's
    /// conditions with its equality to `v`.
    fn one_of(&mut self, v: &Val, candidates: &[(Vec<Formula>, Val)]) -> Formula {
        let mut cases = Vec::new();
        for (g, c) in candidates {
            let mut all = g.clone();
            all.extend(self.same(v, c));
            cases.push(Formula::and(all));
        }
        Formula::or(cases)
    }

    /// What a call at `span` gives once what it's lent is forgotten,
    /// `post`: its postconditions, of the values behind what it lent to
    /// call-scoped `&mut`s, which are what it left there, as nothing it
    /// made from them outlives it; and the refinements of the struct of
    /// each field a refinement is about it lent, proved again.
    fn post_now(&mut self, post: Post, span: Span) -> Result<(), Error> {
        for (p, tree) in &post.lent {
            self.lent_trees.insert(p.clone(), tree.clone());
        }
        // Each place lent whose own refinement is proved again below: a
        // refined local's, or what's declared of it where it is (what a
        // refined `&mut` points to). Until then, reading it, as for what
        // the call ensures, doesn't assume it.
        let mut reprove: Vec<(Place, Option<Refinement>, Vec<targs::Disj>)> = Vec::new();
        for p in post.scoped.iter().flatten() {
            let local = (p.path.is_empty())
                .then(|| self.invariants.get(&p.root).map(|(r, _)| r.clone()))
                .flatten();
            // Without the field refinements of a struct it's in, which
            // would read the struct's fields, assuming them before
            // `prove_fields` proves them: what's declared of the place
            // itself comes through a `*` only.
            let own: Vec<targs::Disj> = (self.place_decl_own(p).into_iter())
                .filter(|(pos, d)| pos.is_empty() && !d.is_any())
                .map(|(_, d)| d)
                .collect();
            if local.is_some() || !own.is_empty() {
                self.reproving.push(p.clone());
                reprove.push((p.clone(), local, own));
            }
        }
        if let Some(sig) = &post.sig {
            // What's behind a call-scoped `&mut` is what the call left
            // there: at the place lent, or, if that isn't known, a value
            // nothing's known of, never the one before. The reference is
            // the one it was, the same object: so what it says of what it
            // points to, a slice's length, is too.
            let now: Vec<Val> = (post.old.iter().zip(&post.scoped).enumerate())
                .map(|(i, (v, p))| {
                    let o = match v {
                        Val::Ref(o, _) => *o,
                        _ => self.new_obj(),
                    };
                    match p {
                        Some(p) => Val::Ref(o, Pointee::Place(p.clone())),
                        None if sig.scoped.get(i) == Some(&true) => Val::Ref(o, Pointee::Unknown),
                        None => v.clone(),
                    }
                })
                .collect();
            let env = Env::Post(&post.old, Some(&now), &post.targs);
            // A field lent here is a new value, which the struct's
            // refinements are proved of below, not assumed: so reading it
            // for what the call ensures assumes nothing of it but that.
            let lent_field =
                (post.scoped.iter().flatten()).any(|p| !self.constrained_around(p).is_empty());
            let was = self.proving_fields;
            self.proving_fields |= lent_field;
            let mut facts = Vec::new();
            if let Some(r) = &sig.ret
                && let Some(f) = self.pred(r, &post.result, env)
            {
                facts.push(f);
            }
            for r in &sig.ensures {
                if let Some(f) = self.pred(r, &Val::Other, env) {
                    facts.push(f);
                }
            }
            self.proving_fields = was;
            for f in facts {
                self.fact(f);
            }
        }
        for p in post.scoped.iter().flatten() {
            for (q, i) in self.constrained_around(p) {
                self.prove_fields(&q, i, span)?;
            }
        }
        for (p, local, own) in reprove {
            let t = self.place_ty(&p);
            let v = self.read_place(&p);
            let name = self.place_name(&p).unwrap_or_default();
            let name = match name.strip_prefix('(').and_then(|n| n.strip_suffix(')')) {
                Some(inner) => inner.to_string(),
                None => name,
            };
            let what = format!("`{name}` after the call to `{}`", post.name);
            if let Some(r) = &local {
                self.need(r, &v, Env::Here, span, what.clone())?;
            }
            let tree: targs::Tree = own.into_iter().map(|d| (Vec::new(), d)).collect();
            self.need_tree(&v, &t, &tree, None, span, &what)?;
            self.reproving.retain(|q| *q != p);
            // Proved: so it's known of what's there now.
            if p.path.is_empty() {
                self.assume_invariant(p.root, &v);
            }
            self.assume_tree(&v, &t, &tree);
        }
        Ok(())
    }

    /// After a write to `p`, a part of a refined local, at `span`: the
    /// local's refinement, an invariant, must still hold of its whole value
    /// (step 8; the write to a part used to go unchecked, which tuples'
    /// `t.0 = ..` made easy to write, docs/58-tuples.md).
    fn keep_invariant(&mut self, p: &Place, span: Span) -> Result<(), Error> {
        if p.path.is_empty() || p.path.contains(&Proj::Deref) && !self.boxes_only(p) {
            return Ok(());
        }
        let Some((r, name)) = self.invariants.get(&p.root).cloned() else {
            return Ok(());
        };
        // Its fields as they are now: reading the local whole would be a
        // new value of it, which has its refinement, assumed.
        let root = Place::local(p.root);
        let t = self.place_ty(&root);
        let whole = match self.fields_of(&root, &t) {
            Some(vals) => Val::Struct(self.new_obj(), vals),
            None => match self.read_place(&root) {
                Val::Ref(_, Pointee::Place(q)) => {
                    let o = self.new_obj();
                    Val::Ref(o, Pointee::Val(Box::new(self.read_place(&q))))
                }
                _ => Val::Other,
            },
        };
        self.need(&r, &whole, Env::Here, span, format!("`{name}`"))
    }

    /// Before `new`, of type `ty`, is written to place `p`, written
    /// `place`: it must have what's declared there (lemma A6), of what's
    /// in a local through fields and boxes, the local's, and through a
    /// reference, what the reference says.
    fn keep_declared(
        &mut self,
        p: &Place,
        place: &str,
        new: &Val,
        ty: &Type,
        span: Span,
    ) -> Result<(), Error> {
        let d = self.place_decl(p);
        let what = match (self.boxes_only(p), self.cx.typed.binds.get(&p.root)) {
            (true, Some(b)) => format!("`{}`", b.name),
            _ => format!("what's written through `{place}`"),
        };
        let written = self.written_tree(new, ty);
        self.need_tree(new, ty, &d, Some(&written), span, &what)
    }

    /// Is every `*` on the way from `p`'s local to it of a box?
    fn boxes_only(&mut self, p: &Place) -> bool {
        let mut q = Place::local(p.root);
        for step in &p.path {
            if *step == Proj::Deref && !matches!(self.place_ty(&q), Type::Box(_)) {
                return false;
            }
            q = q.then(*step);
        }
        true
    }

    /// The fields that place `p` is, or is in, that a refinement is
    /// about, innermost first, each as its struct's place and its index:
    /// through fields and boxes, not references (see `Walk::constrained_mir`).
    fn constrained_around(&self, p: &Place) -> Vec<(Place, usize)> {
        let mut out = Vec::new();
        for k in (1..=p.path.len()).rev() {
            let q = Place {
                root: p.root,
                path: p.path[..k - 1].to_vec(),
            };
            match p.path[k - 1] {
                Proj::Field(i) => {
                    let t = self.place_ty(&q);
                    if self
                        .refined_fields(&t)
                        .and_then(|f| f.constrained(i))
                        .is_some()
                    {
                        out.push((q, i));
                    }
                }
                Proj::Deref => {
                    if !matches!(self.place_ty(&q), Type::Box(_)) {
                        break;
                    }
                }
            }
        }
        out
    }

    /// Before a write to place `p`: what's known of the structs around it
    /// whose refinements are about the fields it's in (but the one it's a
    /// field of, which an assignment proves itself), while they're whole.
    fn around_before(&mut self, p: &Place) {
        for (q, i) in self.constrained_around(p) {
            if q.then(Proj::Field(i)) != *p {
                let t = self.place_ty(&q);
                self.fields_of(&q, &t);
            }
        }
    }

    /// After a write to place `p`: those structs' refinements, proved again.
    fn around_after(&mut self, p: &Place, span: Span) -> Result<(), Error> {
        for (q, i) in self.constrained_around(p) {
            if q.then(Proj::Field(i)) != *p {
                self.prove_fields(&q, i, span)?;
            }
        }
        Ok(())
    }

    /// The refinements of the struct at `q` about its field `i` hold, as
    /// it is now, after what's in that field changed: proved of its
    /// fields' values, without assuming them, and then known.
    fn prove_fields(&mut self, q: &Place, i: usize, span: Span) -> Result<(), Error> {
        let t = self.place_ty(q);
        self.proving_fields = true;
        let vals = self.fields_of(q, &t);
        self.proving_fields = false;
        if let Some(vals) = vals {
            self.need_fields(&t, &vals, Some(i), span)?;
            self.assume_fields(&t, &vals);
        }
        Ok(())
    }
}

/// `a op b` in the logic, if it's in it: why not if it isn't. A product
/// may add a variable to `kinds` (`logic::times`).
fn binary(op: BinOp, a: Val, b: Val, kinds: &mut Vec<Kind>) -> Result<Val, Failure> {
    let limit = Failure::ArithmeticLimit;
    Ok(match (op, a, b) {
        (BinOp::Add, Val::Int(a), Val::Int(b)) => Val::Int(a.add(&b).ok_or(limit)?),
        (BinOp::Sub, Val::Int(a), Val::Int(b)) => Val::Int(a.sub(&b).ok_or(limit)?),
        (BinOp::Mul, Val::Int(a), Val::Int(b)) => Val::Int(logic::times(&a, &b, kinds)?),
        (op, Val::Int(a), Val::Int(b)) => Val::Bool(compare(op, &a, &b).ok_or(limit)?),
        (BinOp::Eq, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b)),
        (BinOp::Ne, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b).not()),
        (BinOp::And, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::and([a, b])),
        (BinOp::Or, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::or([a, b])),
        _ => return Err(Failure::Unsupported),
    })
}

/// Does a value of type `t` have an object (see `Val`): is it anything
/// but an integer, a `bool`, `()` or a function?
fn has_obj(t: &Type) -> bool {
    !matches!(
        t,
        Type::Int(_) | Type::Bool | Type::Char | Type::Unit | Type::Never | Type::Fn(_)
    )
}

/// That `a` and `b` are equal, as far as the logic can say.
fn equal(a: &Val, b: &Val) -> Vec<Formula> {
    match (a, b) {
        (Val::Int(a), Val::Int(b)) => Formula::eq(a, b).into_iter().collect(),
        (Val::Bool(a), Val::Bool(b)) => vec![Formula::iff(a.clone(), b.clone())],
        (Val::Struct(_, a), Val::Struct(_, b)) => {
            a.iter().zip(b).flat_map(|(a, b)| equal(a, b)).collect()
        }
        _ => Vec::new(),
    }
}

/// `a op b`, for a comparison `op`.
fn compare(op: BinOp, a: &Lin, b: &Lin) -> Option<Formula> {
    match op {
        BinOp::Lt => Formula::lt(a, b),
        BinOp::Le => Formula::le(a, b),
        BinOp::Gt => Formula::lt(b, a),
        BinOp::Ge => Formula::le(b, a),
        BinOp::Eq => Formula::eq(a, b),
        BinOp::Ne => Some(Formula::eq(a, b)?.not()),
        _ => None,
    }
}

/// The greatest constant `k` the logic takes `x << k` and `x >> k` to be
/// linear by, as `x` times or divided by 2^k: the integers of every type
/// are less than 2^64 from 0, so further is as good as infinitely far.
const MAX_SHIFT: i128 = 64;

/// What the logic knows of `q`, the result of `a op b` for a bitwise
/// operator or `>>`. Each part is a lemma of two's complement arithmetic,
/// in which a non-negative number has finitely many 1 bits and a negative
/// one infinitely many, to its left:
///
/// - `a & b` has only bits both have, so it's between 0 and `a` if `a >= 0`,
///   and the same of `b`; and it's negative if both are. If `b` is all
///   ones, `2^m - 1`, a number from 0 to `b` keeps all of its bits.
/// - `a | b` has the bits either has: so it's at least each and at most
///   their sum if both are non-negative (it's `a + b - (a & b)`), and
///   negative if either is.
/// - `a ^ b` is non-negative when they have the same sign, and at most their
///   sum if both are non-negative (it's `a + b - 2·(a & b)`).
/// - `a >> b` is `a` divided by 2^b, rounding down: between 0 and `a`, or
///   `a` and -1. By a constant `k`, `2^k·q <= a <= 2^k·q + 2^k - 1`.
///
/// `None` if a number gets too big.
fn bitwise(op: BinOp, a: &Lin, b: &Lin, q: &Lin) -> Option<Formula> {
    let zero = Lin::constant(0);
    let natural = |x: &Lin| Formula::le(&zero, x);
    let implies = |p: Formula, c: Formula| Formula::or([p.not(), c]);
    let between = |lo: &Lin, x: &Lin, hi: &Lin| {
        Some(Formula::and([Formula::le(lo, x)?, Formula::le(x, hi)?]))
    };
    Some(match op {
        BinOp::BitAnd => {
            let mut parts = vec![
                implies(natural(a)?, between(&zero, q, a)?),
                implies(natural(b)?, between(&zero, q, b)?),
                implies(
                    Formula::and([natural(a)?.not(), natural(b)?.not()]),
                    natural(q)?.not(),
                ),
            ];
            for (x, mask) in [(a, b), (b, a)] {
                if let Some(k) = mask.as_constant()
                    && k >= 0
                    && (k + 1).count_ones() == 1
                {
                    parts.push(implies(between(&zero, x, mask)?, Formula::eq(q, x)?));
                }
            }
            Formula::and(parts)
        }
        BinOp::BitOr => Formula::and([
            implies(
                Formula::and([natural(a)?, natural(b)?]),
                Formula::and([
                    Formula::le(a, q)?,
                    Formula::le(b, q)?,
                    Formula::le(q, &a.add(b)?)?,
                ]),
            ),
            implies(
                Formula::or([natural(a)?.not(), natural(b)?.not()]),
                natural(q)?.not(),
            ),
        ]),
        BinOp::BitXor => Formula::and([
            implies(
                Formula::and([natural(a)?, natural(b)?]),
                Formula::le(q, &a.add(b)?)?,
            ),
            Formula::iff(natural(q)?, Formula::iff(natural(a)?, natural(b)?)),
        ]),
        BinOp::Shr => {
            let minus_one = Lin::constant(-1);
            let mut parts = vec![
                implies(natural(a)?, between(&zero, q, a)?),
                implies(natural(a)?.not(), between(a, q, &minus_one)?),
            ];
            match b.as_constant() {
                Some(k @ 0..MAX_SHIFT) => {
                    let p = 1i128 << k;
                    let pq = q.scale(p)?;
                    parts.push(between(&pq, a, &pq.plus(p - 1)?)?);
                }
                Some(MAX_SHIFT..) => {
                    parts.push(implies(natural(a)?, Formula::eq(q, &zero)?));
                    parts.push(implies(natural(a)?.not(), Formula::eq(q, &minus_one)?));
                }
                _ => {}
            }
            Formula::and(parts)
        }
        _ => return None,
    })
}

/// The law that makes `q` the quotient of `a` by a constant `m >= 1`,
/// rounding towards zero, which the logic can say, as it's linear: `m·q <=
/// a <= m·q + m - 1` if `a >= 0`, and `m·q - m + 1 <= a <= m·q` if `a <= 0`.
/// Exactly one integer is that (lemma Q1 of docs/72-division.md): the
/// greatest with `m·q <= a`, or the least with `a <= m·q`, which agree at
/// 0. If `a`'s variables' ranges say it's never negative, or never
/// positive (`sign`), only that half: the other then says nothing, and
/// would be a case for the procedure to split. `None` if a number gets
/// too big.
fn truncated(a: &Lin, m: i128, q: &Lin, sign: Option<(bool, bool)>) -> Option<Formula> {
    let zero = Lin::constant(0);
    let mq = q.scale(m)?;
    let when_pos = Formula::and([Formula::le(&mq, a)?, Formula::le(a, &mq.plus(m - 1)?)?]);
    let when_neg = Formula::and([Formula::le(&mq.plus(1 - m)?, a)?, Formula::le(a, &mq)?]);
    Some(match sign {
        Some((true, _)) => when_pos,
        Some((_, true)) => when_neg,
        _ => Formula::and([
            Formula::or([Formula::le(&zero, a)?.not(), when_pos]),
            Formula::or([Formula::le(a, &zero)?.not(), when_neg]),
        ]),
    })
}

/// What the logic knows of `q = a % b`, by any `b`, which is linear in
/// all three (lemma L4 of step 28): rounding towards zero, the remainder
/// is less than `b` from 0, no further from 0 than `a`, and 0 or of `a`'s
/// sign. These hold of the integers, and so of every integer type, as `%`
/// gives the integers' remainder whenever it doesn't fail: it's less than
/// `b` from 0, so it fits whatever `b` does, and the one case that fails,
/// the least value by -1, is proved not to happen before this is known
/// (and its remainder, 0, would satisfy it anyway). Nothing for `b = 0`,
/// which fails. `None` if a number gets too big.
fn remainder(a: &Lin, b: &Lin, q: &Lin) -> Option<Formula> {
    let zero = Lin::constant(0);
    let le = Formula::le;
    let (a_pos, a_neg) = (le(&zero, a)?, le(a, &zero)?);
    // Less than `|b|` from 0, at `lo..=hi`, one way or the other of it.
    let cases = [
        (Formula::lt(&zero, b)?, b.scale(-1)?.plus(1)?, b.plus(-1)?),
        (Formula::lt(b, &zero)?, b.plus(1)?, b.scale(-1)?.plus(-1)?),
    ];
    let mut parts = Vec::new();
    for (sign, lo, hi) in cases {
        let when = |p: Formula, c: [Formula; 3]| {
            Formula::or([Formula::and([sign.clone(), p]).not(), Formula::and(c)])
        };
        parts.push(when(a_pos.clone(), [le(&zero, q)?, le(q, &hi)?, le(q, a)?]));
        parts.push(when(a_neg.clone(), [le(&lo, q)?, le(q, &zero)?, le(a, q)?]));
    }
    Some(Formula::and(parts))
}
