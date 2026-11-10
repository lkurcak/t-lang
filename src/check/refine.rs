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
//! own over each body, once its types are known (`Checker::refine`), which
//! keeps track of what's known at each point (*facts*) and proves from them
//! what each refinement needs, with the procedure in `refine/logic.rs`:
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
//! `Walk::frames`);
//! after a branch that does, where it's whichever value the branch taken
//! gave it; and after a call it's lent to `&mut` (`Walk::borrows`), when
//! nothing can keep the reference after the call. A place that may change
//! some other way, through a `&mut` that may outlive where it's made, or a
//! closure that captures it, gives a new value each time it's read: nothing
//! is known of it but its refinements, which every assignment, inside a
//! closure too, must keep (`Walk::unstable_in`). A `&` can't change what it
//! points to, as nothing in T can be changed through one, so `*r` for a
//! `let r = &p;` is `p` (`Walk::alias`). A refined variable, a field a
//! refinement is about, and what a refined reference points to can't be
//! borrowed `&mut` but straight to a call-scoped parameter (`call_scoped`),
//! after which their refinements, and the struct's about the field, are
//! proved again, of what the call left, before anything assumes them
//! (`Walk::post`, `Walk::reproving`); a place in a refined variable
//! (through fields and boxes) can't be at all. So the refinements of a
//! place hold of its value whenever it can be read: every way of making or
//! changing one checks them, a call lent one runs while nothing can read it
//! (and nothing unwinds), and anything else, generic code included, only
//! moves whole values.
//!
//! Postconditions (`ensures`, a result's refinement) are proved by the
//! callee at each return, of what's behind its call-scoped `&mut`s then,
//! and learned by the caller of the places it lent, once they're forgotten
//! (`Walk::post`): nothing made from such a parameter outlives the call, so
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
//! impl (`Walk::callee`): monomorphisation picks the same one.
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

    /// Why field `i` can't be borrowed `&mut`, if it can't.
    fn constrained(&self, i: usize) -> Option<String> {
        let j = self.about(i).next()?;
        Some(self.why(i, j))
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
        let written = f.params.iter().map(|p| &p.ty).chain([&f.ret]);
        if written.clone().all(|t| t.refinements().is_empty()) && f.ensures.is_empty() {
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
                    let outer = std::mem::replace(&mut self.env, env);
                    let checked =
                        self.sig_refinements(&f, &params, &ret, (allowed, results), &consts);
                    self.env = outer;
                    checked?;
                }
                Pending::Fields { s, symbol } => {
                    self.pred_owner = None;
                    self.module = s.module;
                    self.fields_now(&s, &symbol)?;
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
        let refined = |t: &&TypeExpr| !t.refinements().is_empty();
        let param = f.params.iter().map(|p| &p.ty).find(refined);
        let result = Some(&f.ret).filter(refined);
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

    /// Checks the refinements of the fields of struct `s`, whose symbol
    /// is `symbol`, and keeps them. A field's names the fields before it,
    /// as visible as it is.
    pub(super) fn field_refinements(&mut self, s: &StructDecl, symbol: &str) -> Result<(), Error> {
        if s.fields.iter().all(|f| f.ty.refinements().is_empty()) {
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
                    let (_, binder, _) = f.ty.refinements().into_iter().next().expect("refined");
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
        if ann.refinements().is_empty() {
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
        let written = t.refinements();
        if written.is_empty() {
            return Ok(None);
        }
        let base = self.solved(base);
        let mut parts = Vec::new();
        for (written_path, binder, pred) in written {
            let (of, ways, path, carried) = self.refined_arg(t, &base, &written_path)?;
            let kind = super::NoteKind::Var(binder.name.clone());
            self.note(binder.span, kind, Some(of.clone()), Some(binder.span));
            let scope = |name: &str| match name == binder.name {
                true => Some(Named::Value(of.clone(), PredName::Binder, binder.span)),
                false => scope(name),
            };
            let got = self.pred_type(pred, &scope)?;
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
            ExprKind::Var(name, targs) if targs.is_empty() => match scope(name) {
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
            },
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
                BinOp::Div
                | BinOp::Rem
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => Err(Error::ty(
                    e.span,
                    format!(
                        "{} can't use `{op}` yet: only `+`, `-` and `*` by a constant",
                        self.pred_a()
                    ),
                )),
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
                    "{} can only use literals, names, fields, measures, `+`, `-`, `*` by a \
                     constant, comparisons, `&&`, `||` and `!`",
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
                    .implements(&t, INT, &self.env, &self.adts.proj.local)
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
            ExprKind::Var(name, targs) if targs.is_empty() => match scope(name) {
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
            },
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
                .implements(&ret, INT, &self.env, &self.adts.proj.local)
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
                "a measure's body can only use `self`'s fields, measures, literals, `+`, `-`, `*` \
                 by a constant, comparisons, `&&`, `||` and `!`",
            )
        };
        let ty = self.typed.exprs.get(&e.id).map(|t| self.solved(t));
        let scalar = match &ty {
            Some(Type::Int(_) | Type::Bool) => true,
            // One of some integer type, `T: Int`.
            Some(t @ Type::Param(_, _)) => {
                (self
                    .traits
                    .implements(t, INT, &self.env, &self.adts.proj.local))
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
            ExprKind::Binary(op, l, r) => {
                if matches!(
                    op,
                    BinOp::Div
                        | BinOp::Rem
                        | BinOp::BitAnd
                        | BinOp::BitOr
                        | BinOp::BitXor
                        | BinOp::Shl
                        | BinOp::Shr
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
    /// of its operators can't panic, taking those out of its effects.
    pub(super) fn refine(&mut self, f: &FnDecl) -> Result<(), Error> {
        let mut failed = Vec::new();
        let result = self.refine_walk(f, &mut failed);
        self.proof_errors.append(&mut failed);
        result
    }

    /// `refine`, leaving in `failed` the proofs that failed, which don't
    /// stop it (`Walk::failed`).
    fn refine_walk(&mut self, f: &FnDecl, failed: &mut Vec<Error>) -> Result<(), Error> {
        let id = self.current.expect("in a function");
        let symbol = self.fns[id.0].symbol.clone();
        let sig = self.refined.get(&symbol).cloned();
        if self.measures.contains_key(&symbol) {
            self.measure_body(&symbol, &f.body)?;
        }
        let mut w = Walk {
            cx: self,
            kinds: Vec::new(),
            facts: Vec::new(),
            laws: Vec::new(),
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
            unstable: HashSet::new(),
            lent: HashMap::new(),
            alias: HashMap::new(),
            declared: Vec::new(),
            proved: HashSet::new(),
            params: f.params.iter().map(|p| p.name.id).collect(),
            entry: Vec::new(),
            ret: None,
            ensures: None,
            posts: HashMap::new(),
            scoped_args: HashSet::new(),
            proving_fields: false,
            reproving: Vec::new(),
            closures: 0,
            loops: Vec::new(),
            size: None,
            bounds: HashMap::new(),
            consts: HashMap::new(),
            failures: HashMap::new(),
            invalidated: Vec::new(),
            errors: Vec::new(),
        };
        w.unstable_in(&f.body, false);
        let mut ret = None;
        for (i, p) in f.params.iter().enumerate() {
            if let Some(r) = sig.as_ref().and_then(|s| s.params[i].clone()) {
                w.declare(p.name.id, &r, &p.name.name);
            }
            w.bind(p.name.id, None);
        }
        // What the parameters are at the call, which a result's refinement
        // is about: what's behind a `&mut` too, which may change after.
        for p in &f.params {
            let v = w.read_place(&Place::local(p.name.id));
            let v = w.settled(v, ENTRY_DEPTH);
            w.entry.push(v);
        }
        if let Some(sig) = &sig
            && let Some(r) = &sig.ret
        {
            ret = Some((r.clone(), sig.name.clone()));
        }
        w.ret = ret;
        if let Some(sig) = &sig
            && !sig.ensures.is_empty()
        {
            w.ensures = Some((sig.ensures.clone(), sig.name.clone()));
        }
        let v = w.expr(&f.body);
        failed.append(&mut w.errors);
        let v = v?;
        let mut known = None;
        if f.is_const() && w.reachable {
            known = match &v {
                Val::Int(l) => l.as_constant().map(Known::Int),
                Val::Bool(Formula::True) => Some(Known::Bool(true)),
                Val::Bool(Formula::False) => Some(Known::Bool(false)),
                _ => None,
            };
        }
        let done = w.result(v, super::expr::result_span(&f.body));
        failed.append(&mut w.errors);
        done?;
        let proved = std::mem::take(&mut w.proved);
        let failures = std::mem::take(&mut w.failures);
        if let Some(k) = known {
            self.const_values.insert(symbol.clone(), k);
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

/// Calls `f` with each binding in `pat`.
fn each_bind(pat: &Pattern, f: &mut impl FnMut(&Pattern)) {
    match &pat.kind {
        PatternKind::Bind(..) => f(pat),
        PatternKind::Variant(_, _, _, pats)
        | PatternKind::Path(_, pats, _)
        | PatternKind::Or(pats)
        | PatternKind::Array(pats)
        | PatternKind::Tuple(pats) => pats.iter().for_each(|p| each_bind(p, f)),
        PatternKind::Struct(_, _, fields, _) => fields.iter().for_each(|fp| each_bind(&fp.pat, f)),
        PatternKind::Wild
        | PatternKind::Int(..)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Str(_)
        | PatternKind::Unit => {}
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

/// How a path through a branch ended: what it learned, what each place
/// stood for at its end, whether it gets there, and its value.
struct Outcome {
    facts: Vec<Formula>,
    cur: HashMap<Place, Var>,
    objs: HashMap<Place, Obj>,
    reachable: bool,
    val: Val,
    invalidated: Vec<Invalidation>,
}

/// A loop around what's being walked: its label, its invariant, if it
/// has one, which each `continue` for it must prove, and the ways out of
/// it so far: each `break` for it, as it ended, from `head` (the facts
/// there), as a branch is from where it starts.
struct Loop {
    label: Option<String>,
    invariant: Option<Expr>,
    head: usize,
    exits: Vec<Outcome>,
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
    /// The places that may change other than by assignment, with all
    /// that's in them.
    unstable: HashSet<Place>,
    /// The places each expression lends `&mut`, by its node: a call, a
    /// `&mut`, or a `&mut` used whole. What's known of them is forgotten
    /// once it's evaluated.
    lent: HashMap<NodeId, Vec<Place>>,
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
    /// What each call, by node, has yet to give once what it's lent is
    /// forgotten (`Walk::returned`): its postconditions.
    posts: HashMap<NodeId, Post>,
    /// The `&mut`s, by node, made straight for a call-scoped parameter,
    /// which can be of a field a refinement is about.
    scoped_args: HashSet<NodeId>,
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
    loops: Vec<Loop>,
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
    /// `size` and `bounds`, which no path forgets, as those are made once,
    /// on whichever path first needs them.
    laws: Vec<Formula>,
    /// Explanations saved at the point each operator's proof failed.
    failures: HashMap<NodeId, ProofFailure>,
    /// Source events that changed a place's value along this path.
    invalidated: Vec<Invalidation>,
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

    /// Variable `v` of the logic, as a value.
    fn var(&self, v: Var) -> Val {
        match self.kinds[v as usize] {
            Kind::Int(..) | Kind::Bound(..) | Kind::Product(..) => Val::Int(Lin::var(v)),
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
        (cx.traits.implements(t, name, &cx.env, &cx.adts.proj.local)).is_ok()
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
        let stable = !self.is_unstable(p);
        if stable && let Some(&o) = self.objs.get(p) {
            return (o, false);
        }
        let o = self.new_obj();
        if stable {
            self.objs.insert(p.clone(), o);
        }
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

    /// A `&` or `&mut` of `x`: a reference that says what's declared of
    /// the value it points to, which each write through it must keep
    /// (lemma W1 of docs/63-what-is-declared.md). To a place a local is
    /// the root of, what `place_decl` says; to what a reference points to,
    /// that reference, the same object, saying what it says; to any other
    /// place (what a reference or a box a call gave points to, a field of
    /// that), what `lvalue` says. None of these says nothing for want of a
    /// case: a value that's no place is a temporary, an anonymous `let`,
    /// which declares what a `let` of it would.
    fn borrow(&mut self, x: &Expr) -> Result<Val, Error> {
        if let Some(p) = self.place_of(x) {
            if p.path.last() == Some(&Proj::Deref) {
                let outer = Place {
                    root: p.root,
                    path: p.path[..p.path.len() - 1].to_vec(),
                };
                if let Type::Ref(..) = self.place_ty(&outer) {
                    return Ok(self.read_place(&outer));
                }
            }
            // What may be written through it is what's declared of the
            // place, which it says.
            let o = self.new_obj();
            let tree = targs::under(&self.place_decl(&p), 0);
            if !tree.is_empty() {
                self.trees.insert(o, tree);
            }
            return Ok(Val::Ref(o, Pointee::Place(p)));
        }
        if let ExprKind::Unary(UnOp::Deref, y) = &x.kind
            && let Type::Ref(..) = self.ty_of(y)
        {
            return self.expr(y);
        }
        let (v, decl) = self.lvalue(x)?;
        let o = self.new_obj();
        let tree = targs::under(&decl, 0);
        if !tree.is_empty() {
            self.trees.insert(o, tree);
        }
        Ok(Val::Ref(o, Pointee::Val(Box::new(v))))
    }

    /// The value at `x`, and what's declared of it there: what each write
    /// there must keep (lemma W1 of docs/63-what-is-declared.md). By cases
    /// on what `x` is, each case saying it: a place a local is the root of,
    /// `place_decl`; what a reference or a box points to, what's declared
    /// below the top of what it's in, and what the reference says
    /// (`deref_lvalue`); a field, what's declared of what it's in at the
    /// field's positions (`field_decl`); and a value that isn't a place, a
    /// temporary nothing else can name, what a `let` of it would declare:
    /// what its value's type says (`widen`).
    fn lvalue(&mut self, x: &Expr) -> Result<(Val, targs::Tree), Error> {
        if let Some(p) = self.place_of(x) {
            let v = self.read_place(&p);
            let decl = self.place_decl(&p);
            return Ok((v, decl));
        }
        match &x.kind {
            ExprKind::Unary(UnOp::Deref, y) => {
                let (r, decl) = self.lvalue(y)?;
                let t = self.ty_of(y);
                Ok(self.deref_lvalue(r, &decl, &t))
            }
            ExprKind::Field(base, _) => {
                let (derefs, i) = self.cx.typed.fields[&x.id];
                let (v, decl, t) = self.lvalue_derefs(base, derefs)?;
                let vals = match v {
                    Val::Struct(_, vals) => vals,
                    _ => match self.unknown(&t) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                };
                let decl = self.field_decl(&decl, &t, i, Some(&vals));
                let fv = match vals.get(i) {
                    Some(v) => v.clone(),
                    None => {
                        let ft = self.ty_of(x);
                        self.unknown(&ft)
                    }
                };
                Ok((fv, decl))
            }
            // A value, not a place: a temporary. Each kind of expression
            // is here by name, so that a new one can't be taken for one
            // without saying so (a variable that's no local's is a
            // function, a const or a static's reference).
            ExprKind::Var(..)
            | ExprKind::Unary(..)
            | ExprKind::Int(..)
            | ExprKind::Bool(..)
            | ExprKind::Char(..)
            | ExprKind::Str(..)
            | ExprKind::ByteStr(..)
            | ExprKind::Format(..)
            | ExprKind::Unit
            | ExprKind::As(..)
            | ExprKind::Binary(..)
            | ExprKind::Call(..)
            | ExprKind::MethodCall(..)
            | ExprKind::If(..)
            | ExprKind::IfLet(..)
            | ExprKind::Array(..)
            | ExprKind::Tuple(..)
            | ExprKind::Repeat(..)
            | ExprKind::Block(..)
            | ExprKind::Unsafe(..)
            | ExprKind::StructLit(..)
            | ExprKind::Variant(..)
            | ExprKind::Match(..)
            | ExprKind::Lambda(..)
            | ExprKind::Assign(..)
            | ExprKind::While(..)
            | ExprKind::WhileLet(..)
            | ExprKind::Loop(..)
            | ExprKind::For(..)
            | ExprKind::Range(..)
            | ExprKind::Index(..)
            | ExprKind::Try(..)
            | ExprKind::Return(..)
            | ExprKind::Break(..)
            | ExprKind::Continue(..) => {
                let v = self.expr(x)?;
                let t = self.ty_of(x);
                let known = self.knowledge(&v, &t);
                Ok((v, targs::widen(&known)))
            }
        }
    }

    /// `lvalue` of `base`, then through `derefs` `*`s: the value there,
    /// what's declared of it, and its type.
    fn lvalue_derefs(
        &mut self,
        base: &Expr,
        derefs: usize,
    ) -> Result<(Val, targs::Tree, Type), Error> {
        let (mut v, mut decl) = self.lvalue(base)?;
        let mut t = self.ty_of(base);
        for _ in 0..derefs {
            (v, decl) = self.deref_lvalue(v, &decl, &t);
            if let Type::Ref(_, _, inner) | Type::Box(inner) = t {
                t = self.cx.solved(&inner);
            }
        }
        Ok((v, decl, t))
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

    /// Whether `pat` matches `s`, exactly, if the logic can say: so it
    /// doesn't for the arms after one that has it.
    fn pattern_formula(&mut self, pat: &Pattern, s: &Val) -> Option<Formula> {
        match (&pat.kind, s) {
            (PatternKind::Wild | PatternKind::Bind(..), _) => Some(Formula::True),
            (PatternKind::Int(n, _), Val::Int(l)) => Formula::eq(l, &Lin::constant(*n)),
            (PatternKind::Range(lo, hi, ..), Val::Int(l)) => Some(Formula::and([
                match lo {
                    Some(lo) => Formula::le(&Lin::constant(*lo), l)?,
                    None => Formula::True,
                },
                match hi {
                    Some(hi) => Formula::le(l, &Lin::constant(*hi))?,
                    None => Formula::True,
                },
            ])),
            (PatternKind::Bool(b), Val::Bool(f)) => {
                Some(if *b { f.clone() } else { f.clone().not() })
            }
            (PatternKind::Or(alts), _) => {
                let mut each = Vec::new();
                for p in alts {
                    each.push(self.pattern_formula(p, s)?);
                }
                Some(Formula::or(each))
            }
            // That variant, with fields each matching exactly.
            (PatternKind::Variant(_, _, member, pats), _) => {
                let s = self.peeled(pat, s);
                let (tag, fields) = self.variant_parts(&s, &member.name)?;
                let mut all = vec![tag];
                for (p, f) in pats.iter().zip(fields) {
                    all.push(self.pattern_formula(p, &f)?);
                }
                Some(Formula::and(all))
            }
            // Each field matching exactly, of the values a tuple's or a
            // struct's fields are (lemma P1 of docs/62-t-chess-findings.md).
            (PatternKind::Tuple(_) | PatternKind::Struct(..), _) => {
                let mut all = Vec::new();
                for (p, f) in self.field_patterns(pat, s)? {
                    all.push(self.pattern_formula(p, &f)?);
                }
                Some(Formula::and(all))
            }
            _ => None,
        }
    }

    /// The patterns of the fields of `pat`, a tuple or struct pattern, each
    /// with the value of the field it matches, of `s`, if that's known.
    fn field_patterns<'p>(&mut self, pat: &'p Pattern, s: &Val) -> Option<Vec<(&'p Pattern, Val)>> {
        let Val::Struct(_, vals) = self.peeled(pat, s) else {
            return None;
        };
        match &pat.kind {
            PatternKind::Tuple(pats) if pats.len() == vals.len() => {
                Some(pats.iter().zip(vals).collect())
            }
            PatternKind::Struct(_, _, fields, _) => {
                let t = self.cx.solved(self.cx.typed.struct_pats.get(&pat.id)?);
                let names = struct_fields(&self.cx.adts, &t)?;
                let mut out = Vec::new();
                for f in fields {
                    let i = names.iter().position(|(n, _)| *n == f.name.name)?;
                    out.push((&f.pat, vals.get(i)?.clone()));
                }
                Some(out)
            }
            _ => None,
        }
    }

    /// What pattern `pat` matches, of value `s`: what's behind the
    /// references match ergonomics looked through.
    fn peeled(&mut self, pat: &Pattern, s: &Val) -> Val {
        let peels = self.cx.typed.peels.get(&pat.id).copied().unwrap_or(0);
        (0..peels).fold(s.clone(), |v, _| self.pointee(v))
    }

    /// That `s`, an enum's value, is variant `name`, and its fields' values
    /// then, if what it is is known.
    fn variant_parts(&self, s: &Val, name: &str) -> Option<(Formula, Vec<Val>)> {
        let Val::Obj(o) = s else {
            return None;
        };
        let parts = self.parts.get(o)?;
        let v = self.variant_index(&parts.symbol, name)?;
        let tag = Formula::eq(&parts.tag, &Lin::constant(i128::try_from(v).ok()?))?;
        Some((tag, parts.fields[v].clone()))
    }

    /// What it says of `s` that `pat` matches it, beyond `pattern_formula`
    /// (which is exactly when it matches): a string pattern of `n` bytes
    /// matches only a `&str` whose `len()` is `n`, but not every such
    /// one. So this may be assumed where `pat` matched, and its negation
    /// never where it didn't (lemma S1 of docs/42-patterns-and-loops.md).
    fn pattern_implies(&mut self, pat: &Pattern, s: &Val) -> Option<Formula> {
        match &pat.kind {
            // It's that variant, and each field matches what it has.
            PatternKind::Variant(_, _, member, pats) => {
                let s = self.peeled(pat, s);
                let (tag, fields) = self.variant_parts(&s, &member.name)?;
                let mut all = vec![tag];
                for (p, f) in pats.iter().zip(fields) {
                    all.extend(self.pattern_formula(p, &f));
                    all.extend(self.pattern_implies(p, &f));
                }
                Some(Formula::and(all))
            }
            // Each field matches, and what that implies.
            PatternKind::Tuple(_) | PatternKind::Struct(..) => {
                let mut all = Vec::new();
                for (p, f) in self.field_patterns(pat, s)? {
                    all.extend(self.pattern_formula(p, &f));
                    all.extend(self.pattern_implies(p, &f));
                }
                Some(Formula::and(all))
            }
            PatternKind::Str(text) => {
                const LEN: &str = STR_LEN;
                if !self.cx.measures.contains_key(LEN) {
                    return None;
                }
                let peels = self.cx.typed.peels.get(&pat.id).copied().unwrap_or(0);
                let mut s = s.clone();
                for _ in 0..peels {
                    s = self.pointee(s);
                }
                let Val::Int(len) = self.measure(LEN, s) else {
                    return None;
                };
                let n = i128::try_from(text.len()).expect("a literal fits");
                Formula::eq(&len, &Lin::constant(n))
            }
            // Each alternative implies what it does; one of them matched.
            PatternKind::Or(alts) => {
                let each = alts.iter().map(|a| self.pattern_implies(a, s));
                Some(Formula::or(each.collect::<Option<Vec<_>>>()?))
            }
            _ => None,
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
                binary(*op, a, b, &mut self.kinds).ok()?
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

    fn ty_of(&self, e: &Expr) -> Type {
        match self.cx.typed.exprs.get(&e.id) {
            Some(t) => self.cx.solved(t),
            None => Type::Unit,
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

    /// The place expression `e` is, if it's one.
    fn place_of(&self, e: &Expr) -> Option<Place> {
        match &e.kind {
            ExprKind::Var(..) => match self.cx.typed.vars.get(&e.id) {
                Some(Res::Local(id)) => Some(Place::local(*id)),
                _ => None,
            },
            ExprKind::Field(x, _) => {
                let (derefs, i) = *self.cx.typed.fields.get(&e.id)?;
                let mut p = self.place_of(x)?;
                for _ in 0..derefs {
                    p = self.deref(p);
                }
                Some(p.then(Proj::Field(i)))
            }
            ExprKind::Unary(UnOp::Deref, x) => Some(self.deref(self.place_of(x)?)),
            _ => None,
        }
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

    /// May what's at `p` change other than by assignment?
    fn is_unstable(&self, p: &Place) -> bool {
        (0..=p.path.len()).any(|n| {
            self.unstable.contains(&Place {
                root: p.root,
                path: p.path[..n].to_vec(),
            })
        })
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

    /// Does pattern `pat` bind anything by `&mut`?
    fn binds_mut(&self, pat: &Pattern) -> bool {
        let mut any = false;
        each_bind(pat, &mut |p| {
            let id = self.cx.typed.sites.get(&p.id).copied().unwrap_or(p.id);
            any |= self.cx.typed.binds.get(&id).and_then(|b| b.by_ref) == Some(true);
        });
        any
    }

    /// Finds the places in `e` that may change other than by assignment.
    /// Those lent `&mut`, by `&mut` or to a call, and what a `&mut` used
    /// whole points to, as whatever it's given to can write through it,
    /// change only where they're lent and while the borrow lives, which
    /// the borrow checker says no one else can read them: so what's known
    /// of them is forgotten there (`Walk::lent`). Those borrowed `&mut` by a
    /// pattern, and the locals a closure captures that are `mut` or hold a
    /// `&mut`, may change any time, as far as anything here knows.
    /// `projected` says `e` is only looked inside of, by a field, a `*`, a
    /// `&` or an assignment.
    fn unstable_in(&mut self, e: &Expr, projected: bool) {
        if !projected
            && let Type::Ref(_, true, _) = self.ty_of(e)
            && let Some(p) = self.place_of(e)
            && self.cx.typed.coerced.get(&e.id).is_none_or(|c| c.1)
        {
            let p = p.then(Proj::Deref);
            self.lent.entry(e.id).or_default().push(p);
        }
        match &e.kind {
            ExprKind::Unary(UnOp::RefMut, x) => {
                if let Some(p) = self.place_of(x) {
                    self.lent.entry(e.id).or_default().push(p);
                }
                self.unstable_in(x, true);
            }
            ExprKind::Unary(UnOp::Ref | UnOp::Deref, x) | ExprKind::Field(x, _) => {
                self.unstable_in(x, true);
            }
            ExprKind::Assign(_, place, value) => {
                self.unstable_in(place, true);
                self.unstable_in(value, false);
            }
            ExprKind::Variant(.., args, true) if self.fn_named(e).is_some() => {
                let passed: Vec<Option<Place>> = args.iter().map(|a| self.passed(a)).collect();
                self.borrows(e, &passed);
                for (a, p) in args.iter().zip(&passed) {
                    match (&a.kind, p) {
                        (ExprKind::Unary(UnOp::RefMut, x), Some(_)) => self.unstable_in(x, true),
                        (_, Some(_)) => self.unstable_in(a, true),
                        (_, None) => self.unstable_in(a, false),
                    }
                }
            }
            ExprKind::Call(callee, args) => {
                let passed: Vec<Option<Place>> = args.iter().map(|a| self.passed(a)).collect();
                self.borrows(e, &passed);
                self.unstable_in(callee, false);
                for (a, p) in args.iter().zip(&passed) {
                    match (&a.kind, p) {
                        (ExprKind::Unary(UnOp::RefMut, x), Some(_)) => self.unstable_in(x, true),
                        (_, Some(_)) => self.unstable_in(a, true),
                        (_, None) => self.unstable_in(a, false),
                    }
                }
            }
            ExprKind::MethodCall(x, _, _, args) => {
                let receiver = match self.cx.typed.methods.get(&e.id) {
                    Some((_, derefs, by)) => self.receiver(x, *derefs, *by),
                    None => self.passed(x),
                };
                let mut passed = vec![receiver];
                passed.extend(args.iter().map(|a| self.passed(a)));
                self.borrows(e, &passed);
                self.unstable_in(x, true);
                for (a, p) in args.iter().zip(&passed[1..]) {
                    match (&a.kind, p) {
                        (ExprKind::Unary(UnOp::RefMut, x), Some(_)) => self.unstable_in(x, true),
                        (_, Some(_)) => self.unstable_in(a, true),
                        (_, None) => self.unstable_in(a, false),
                    }
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                if arms.iter().any(|a| self.binds_mut(&a.pat))
                    && let Some(p) = self.place_of(scrutinee)
                {
                    self.unstable.insert(p);
                }
                self.unstable_in(scrutinee, false);
                for a in arms {
                    self.unstable_in(&a.body, false);
                }
            }
            ExprKind::Block(stmts, tail) => {
                for s in stmts {
                    match s {
                        Stmt::Let { pat, init, .. } => {
                            if let Some(init) = init {
                                if self.binds_mut(pat)
                                    && let Some(p) = self.place_of(init)
                                {
                                    self.unstable.insert(p);
                                }
                                self.unstable_in(init, false);
                            }
                        }
                        Stmt::Expr(e) | Stmt::BlockLike(e) => self.unstable_in(e, false),
                        Stmt::Fn(_) | Stmt::Type(_) => {}
                    }
                }
                if let Some(t) = tail {
                    self.unstable_in(t, false);
                }
            }
            ExprKind::Lambda(l) => {
                for id in self.cx.typed.captures.get(&l.id).into_iter().flatten() {
                    let Some(b) = self.cx.typed.binds.get(id) else {
                        continue;
                    };
                    let ty = self.cx.solved(&b.ty);
                    if b.mutable || self.has_mut_ref(&ty, &mut HashSet::new()) {
                        self.unstable.insert(Place::local(*id));
                    }
                }
                self.unstable_in(&l.body, false);
            }
            _ => {
                for c in e.children() {
                    self.unstable_in(c, false);
                }
            }
        }
    }

    /// The place argument `a` lends `&mut`, if it does, by `&mut` or by
    /// passing on a `&mut` it is.
    fn passed(&self, a: &Expr) -> Option<Place> {
        let ty = self.ty_of(a);
        let shared = self.cx.typed.coerced.get(&a.id).is_some_and(|c| !c.1);
        match (&a.kind, &ty) {
            (ExprKind::Unary(UnOp::RefMut, x), _) => self.place_of(x),
            (_, Type::Ref(_, true, _)) if !shared => self.place_of(a).map(|p| p.then(Proj::Deref)),
            _ => None,
        }
    }

    /// The place receiver `x` lends `&mut`, if it does, got through
    /// `derefs` `*`s and then maybe a `&` or `&mut` (`by`).
    fn receiver(&self, x: &Expr, derefs: usize, by: Option<bool>) -> Option<Place> {
        let mut ty = self.ty_of(x);
        for _ in 0..derefs {
            if let Type::Ref(_, _, inner) | Type::Box(inner) = ty {
                ty = self.cx.solved(&inner);
            }
        }
        let mut place = self.place_of(x);
        for _ in 0..derefs {
            place = place.map(|p| p.then(Proj::Deref));
        }
        match by {
            Some(true) => place,
            Some(false) => None,
            None => match ty {
                Type::Ref(_, true, _) => place.map(|p| p.then(Proj::Deref)),
                _ => None,
            },
        }
    }

    /// The places call `e` lends `&mut`, as `passed` says: what's known of
    /// them is forgotten when it returns. Whatever it does with them then,
    /// or after, if it keeps a reference, nothing reads them until the
    /// borrow ends, as the borrow checker says, and then they're values
    /// nothing's known of.
    fn borrows(&mut self, e: &Expr, passed: &[Option<Place>]) {
        let lent: Vec<Place> = passed.iter().flatten().cloned().collect();
        if !lent.is_empty() {
            self.lent.entry(e.id).or_default().extend(lent);
        }
    }

    /// Can something be written through a value of type `t`: does it hold
    /// a `&mut`, or might it?
    fn has_mut_ref(&self, t: &Type, seen: &mut HashSet<Type>) -> bool {
        match t {
            Type::Int(_) | Type::Bool | Type::Unit | Type::Never | Type::Fn(_) => false,
            Type::Ref(_, m, _) => *m,
            Type::Box(inner) => self.has_mut_ref(&self.cx.solved(inner), seen),
            Type::Adt(..) | Type::Tuple(_) => {
                if !seen.insert(t.clone()) {
                    return false;
                }
                all_fields(&self.cx.adts, t)
                    .iter()
                    .any(|f| self.has_mut_ref(&self.cx.solved(f), seen))
            }
            _ => true,
        }
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
        // Only of a stable place, whose object `fields_of` then finds: one
        // that's new at each read isn't the same value twice.
        if new
            && !self.proving_fields
            && !self.is_unstable(p)
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
                if has_obj(&ft) && (self.is_unstable(&p) || !self.objs.contains_key(&p)) {
                    fresh = true;
                }
                vals.push(self.read_place(&p));
                continue;
            };
            let stable = !self.is_unstable(&p);
            let v = match self.cur.get(&p) {
                Some(&v) if stable => v,
                _ => {
                    fresh = true;
                    let v = self.fresh(kind);
                    let tree = self.place_read(&p);
                    let val = self.var(v);
                    self.assume_tree(&val, &ft, &tree);
                    if stable {
                        self.cur.insert(p, v);
                    }
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
        let stable = !self.is_unstable(p);
        if stable && let Some(&v) = self.cur.get(p) {
            return self.var(v);
        }
        let v = self.unknown(t);
        if let (true, Val::Int(_) | Val::Bool(_)) = (stable, &v) {
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
        let stable = !self.is_unstable(p);
        let kind = match t {
            Type::Adt(..) | Type::Tuple(_) => {
                if let (Some(Val::Struct(o, vals)), Some(fields)) =
                    (&val, struct_fields(&self.cx.adts, t))
                {
                    if stable {
                        self.objs.insert(p.clone(), *o);
                    }
                    for (j, (v, (_, ft))) in vals.iter().zip(fields).enumerate() {
                        let ft = self.cx.solved(&ft);
                        self.set(&p.then(Proj::Field(j)), &ft, Some(v.clone()));
                    }
                }
                if let (Some(Val::Obj(o)), true) = (&val, stable) {
                    self.objs.insert(p.clone(), *o);
                }
                if let (Some(v), true) = (&val, p.path.is_empty()) {
                    self.assume_invariant(p.root, v);
                }
                return;
            }
            _ if !self.is_scalar(t) && *t != Type::Bool => {
                if let (Some(Val::Ref(o, _) | Val::Obj(o)), true) = (&val, stable) {
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
        if !self.is_unstable(p) {
            self.cur.insert(p.clone(), v);
        }
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
                binary(*op, a, b, &mut self.kinds)?
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
            let what = format!("the result of `{}`", self.cx.fns[id.0].name.name);
            self.keep_inv(&v, &ret, &declared, &[], span, &what)?;
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

    /// Runs `go` from `start`, assuming `assume`, and gives how it ended.
    fn branch(
        &mut self,
        start: &Snap,
        assume: Option<Formula>,
        go: impl FnOnce(&mut Self) -> Result<Val, Error>,
    ) -> Result<Outcome, Error> {
        self.restore(start);
        if let Some(f) = assume {
            self.fact(f);
        }
        let val = go(self)?;
        Ok(Outcome {
            facts: self.facts.split_off(start.facts),
            cur: std::mem::take(&mut self.cur),
            objs: std::mem::take(&mut self.objs),
            reachable: self.reachable,
            val,
            invalidated: self.invalidated.clone(),
        })
    }

    /// Where branches `outcomes`, all from `start`, meet: what's known is
    /// that one of them was taken, and each place is whichever value the
    /// branch taken gave it. A place a branch gave a new value nothing's
    /// known of, or never read, is nothing known in that one. Gives the
    /// value, of type `ty`.
    fn join(&mut self, start: &Snap, outcomes: Vec<Outcome>, ty: &Type) -> Val {
        self.restore(start);
        let reached: Vec<Outcome> = outcomes.into_iter().filter(|o| o.reachable).collect();
        if reached.is_empty() {
            self.reachable = false;
            return self.unknown(ty);
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
            return o.val;
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
        let branch_vals: Vec<Val> = reached.iter().map(|o| o.val.clone()).collect();
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
        let result = self.unknown(ty);
        for (o, t, each) in tree_joins {
            self.join_trees(o, &t, &each, &guards);
        }
        self.join_values(&result, ty, &branch_vals, &guards);
        let mut cases = Vec::new();
        for (i, o) in reached.into_iter().enumerate() {
            for event in o.invalidated {
                if !self.invalidated.contains(&event) {
                    self.invalidated.push(event);
                }
            }
            let mut parts = o.facts;
            parts.extend(self.same(&result, &o.val));
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
        result
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

    /// `e` has been evaluated: what it lent `&mut` may have changed.
    fn returned(&mut self, e: &Expr) {
        for p in self.lent.get(&e.id).cloned().into_iter().flatten() {
            self.invalidate(&p, e.span, "mutable lending");
            self.kill(&p);
        }
    }

    /// The head of loop `e`, with invariant `inv`, as the walk gets to it:
    /// the invariant holds as the loop starts; then each place the loop
    /// may change is forgotten, but for what its frame keeps (`frames`),
    /// and the invariant is assumed of them.
    fn loop_head(&mut self, e: &Expr, inv: Option<&Expr>) -> Result<(), Error> {
        let mut changed = Vec::new();
        self.assigned(e, &mut changed);
        let frames = self.frames(e, &changed);
        let inv = inv.map(|p| Refinement::whole(p.clone()));
        if let Some(r) = &inv {
            let span = r.parts[0].pred.span;
            let what = "the loop's invariant, as the loop starts".to_string();
            self.need(r, &Val::Other, Env::Here, span, what)?;
        }
        self.forget(&changed, &frames, e.span);
        if let Some(r) = &inv
            && let Some(f) = self.pred(r, &Val::Other, Env::Here)
        {
            self.fact(f);
        }
        Ok(())
    }

    /// At the end of a loop's body, at `span`, if it gets there: the loop's
    /// invariant `inv`, if it has one, must hold, for the next time round.
    fn again(&mut self, inv: Option<&Expr>, span: Span) -> Result<(), Error> {
        let Some(p) = inv else {
            return Ok(());
        };
        let r = Refinement::whole(p.clone());
        let what = "the loop's invariant, at the end of its body".to_string();
        let end = Span::new(span.end.saturating_sub(1), span.end);
        self.need(&r, &Val::Other, Env::Here, end, what)
    }

    /// The loop a `break` or `continue` with `label` is for, by its index
    /// in `loops`: the one with that label, or the innermost.
    fn target(&self, label: &Option<Ident>) -> Option<usize> {
        match label {
            Some(l) => (self.loops.iter()).rposition(|lp| lp.label.as_deref() == Some(&l.name)),
            None => self.loops.len().checked_sub(1),
        }
    }

    /// The places assigned anywhere in `e`, or lent `&mut` to a call,
    /// together with the source operations that may change them.
    fn assigned(&self, e: &Expr, out: &mut Vec<Invalidation>) {
        if let ExprKind::Assign(_, place, _) = &e.kind
            && let Some(p) = self.place_of(place)
        {
            out.push(Invalidation::new(p, e.span, "assignment"));
        }
        out.extend(
            self.lent
                .get(&e.id)
                .into_iter()
                .flatten()
                .map(|p| Invalidation::new(p.clone(), e.span, "mutable lending")),
        );
        for c in e.children() {
            self.assigned(c, out);
        }
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
        let mut kept = Vec::new();
        for (p, measures) in frames {
            for m in measures {
                let recv = Val::Ref(self.new_obj(), Pointee::Place(p.clone()));
                kept.push((p.clone(), m.clone(), self.measure(m, recv)));
            }
        }
        for change in changes {
            self.invalidate_loop(change, loop_span);
            self.kill(change.place());
        }
        for (p, m, before) in kept {
            let recv = Val::Ref(self.new_obj(), Pointee::Place(p));
            let now = self.measure(&m, recv);
            for f in equal(&now, &before) {
                self.fact(f);
            }
        }
    }

    /// The measures loop `e` keeps of each place it changes, `changed`:
    /// its *frame*. A measure of place `p` is kept if every change in the
    /// loop to `p`, to a place in it, or to one it's in, is a call lending
    /// `p` itself straight to a call-scoped `&mut` whose postconditions say
    /// the measure is as it was (`Walk::keeps`). By induction on how many
    /// times round the loop has been, at the head, and anywhere in the
    /// body, the measure is what it was before the loop: each change since
    /// was such a call, which nothing else could see or write `p` during
    /// (it's lent to the call, and stable), and a measure is of the value
    /// alone (lemma F1 of docs/52-loop-frames.md).
    fn frames(&mut self, e: &Expr, changed: &[Invalidation]) -> Vec<(Place, Vec<String>)> {
        let mut changes = Vec::new();
        self.changes(e, &mut changes);
        let mut places: Vec<&Place> = changed.iter().map(|c| c.place()).collect();
        places.sort();
        places.dedup();
        let mut frames = Vec::new();
        for p in places {
            if self.is_unstable(p) {
                continue;
            }
            let mut kept: Option<Vec<String>> = None;
            for (q, keeps) in &changes {
                if !q.within(p) && !p.within(q) {
                    continue;
                }
                let keeps = match keeps {
                    Some(ms) if q == p => ms,
                    _ => &Vec::new(),
                };
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

    /// Each change in `e` to a place: an assignment, which keeps nothing,
    /// or a `&mut` lent, with the measures it keeps (`Walk::keeps`) if
    /// it's lent straight to a call.
    fn changes(&mut self, e: &Expr, out: &mut Vec<(Place, Option<Vec<String>>)>) {
        if let ExprKind::Assign(_, place, _) = &e.kind
            && let Some(p) = self.place_of(place)
        {
            out.push((p, None));
        }
        if let Some(lent) = self.lent.get(&e.id).cloned() {
            let call = match &e.kind {
                ExprKind::Call(callee, args) => self
                    .fn_named(callee)
                    .map(|s| (s.to_string(), args.iter().map(|a| self.passed(a)).collect())),
                ExprKind::Variant(.., args, true) => self
                    .fn_named(e)
                    .map(|s| (s.to_string(), args.iter().map(|a| self.passed(a)).collect())),
                ExprKind::MethodCall(x, _, _, args) => match self.cx.typed.methods.get(&e.id) {
                    Some((Res::Fn(s, _), derefs, by)) => {
                        let mut passed = vec![self.receiver(x, *derefs, *by)];
                        passed.extend(args.iter().map(|a| self.passed(a)));
                        Some((s.clone(), passed))
                    }
                    _ => None,
                },
                _ => None,
            };
            for p in lent {
                let keeps = match &call {
                    Some((symbol, passed)) => {
                        let at: Vec<usize> = (passed.iter().enumerate())
                            .filter(|(_, q)| q.as_ref() == Some(&p))
                            .map(|(i, _)| i)
                            .collect();
                        match at.as_slice() {
                            [i] => self.keeps(e, symbol, *i),
                            _ => None,
                        }
                    }
                    None => None,
                };
                out.push((p, keeps));
            }
        }
        for c in e.children() {
            self.changes(c, out);
        }
    }

    /// The measures of what call `e`, of function `symbol`, lends its
    /// argument `i` to a call-scoped `&mut` its postconditions say it
    /// leaves as they were, for every value of its other arguments: the
    /// postconditions are assumed of new objects for what's behind the
    /// `&mut` before and after, and new values for the other arguments,
    /// which nothing's known of, and each measure they ask of what's there
    /// after must be proved equal to the one before. Those are what any
    /// call it makes at any time keeps: what the callee proved, and the
    /// variables here are new, so what's known here says nothing of them.
    fn keeps(&mut self, e: &Expr, symbol: &str, i: usize) -> Option<Vec<String>> {
        let (symbol, targs) = self.callee(e, symbol);
        let sig = self.cx.refined.get(&symbol)?.clone();
        if !sig.scoped.get(i).copied().unwrap_or(false) || sig.ensures.is_empty() {
            return None;
        }
        let targs: Vec<(String, Type)> = (sig.type_params.iter().cloned()).zip(targs).collect();
        let args: Vec<&Expr> = match &e.kind {
            ExprKind::Call(_, args) | ExprKind::Variant(.., args, true) => args.iter().collect(),
            ExprKind::MethodCall(x, _, _, args) => std::iter::once(&**x).chain(args).collect(),
            _ => return None,
        };
        let (before, after) = (self.new_obj(), self.new_obj());
        let behind =
            |w: &mut Self, o: Obj| Val::Ref(w.new_obj(), Pointee::Val(Box::new(Val::Obj(o))));
        let mut old = Vec::new();
        let mut now = Vec::new();
        for (j, a) in args.iter().enumerate() {
            if j == i {
                old.push(behind(self, before));
                now.push(behind(self, after));
            } else {
                // A method's receiver may have been borrowed: its value
                // isn't of the expression's type.
                let v = match (j, &e.kind) {
                    (0, ExprKind::MethodCall(..)) => Val::Other,
                    _ => {
                        let t = self.ty_of(a);
                        self.unknown(&t)
                    }
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
            None => {
                let result = self.unknown(ty);
                if let (Val::Int(q), Some(k)) = (&result, b.as_constant())
                    && k > 0
                    && let Some(f) = divided(op, &a, k, q)
                {
                    self.fact(f);
                } else if let Val::Int(q) = &result
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
    /// constant (`divided` has those), when the signs of `a` and `b` are
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

    /// What function `callee` names, if it names one: its symbol.
    /// The type arguments of the function call `e` calls, solved.
    /// The function call `e` of `symbol` runs, if the checker knows it:
    /// for a method of a trait, the method of the impl for the type its
    /// `Self` is, if that type has no type parameters, nor anything else
    /// that could be any type, in it. Then monomorphisation finds that
    /// impl, by the same `select` of the same type, and coherence makes it
    /// the only one; so an impl's method can say more of what it gives, or
    /// ensure more, than its trait does, for the calls known to be of it
    /// (lemma R1 of docs/53-refined-type-arguments.md). Gives too the type
    /// arguments of the call, as the function's are: an impl's method's
    /// are its impl's and then its own.
    fn callee(&self, e: &Expr, symbol: &str) -> (String, Vec<Type>) {
        let targs = self.call_targs(e);
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

    fn call_targs(&self, e: &Expr) -> Vec<Type> {
        let res = match &e.kind {
            ExprKind::MethodCall(..) => self.cx.typed.methods.get(&e.id).map(|m| &m.0),
            ExprKind::Call(callee, _) => self.cx.typed.vars.get(&callee.id),
            _ => self.cx.typed.vars.get(&e.id),
        };
        match res {
            Some(Res::Fn(_, targs)) => targs.iter().map(|t| self.cx.solved(t)).collect(),
            _ => Vec::new(),
        }
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

    fn fn_named(&self, callee: &Expr) -> Option<&str> {
        match self.cx.typed.vars.get(&callee.id) {
            Some(Res::Fn(symbol, _)) => Some(symbol),
            _ => None,
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

    /// A call at `e` of function `symbol`, if it's one, with arguments
    /// `args` (the receiver first, for a method) whose values are `vals`:
    /// checks its refined parameters, and gives its result.
    fn call(
        &mut self,
        e: &Expr,
        symbol: Option<&str>,
        args: &[&Expr],
        vals: Vec<Val>,
        places: Vec<Option<Place>>,
    ) -> Result<Val, Error> {
        let ty = self.ty_of(e);
        let resolved = symbol.map(|s| self.callee(e, s));
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
                if let (Some(r), Some(arg)) = (r, args.get(i))
                    && let Some(r) = r.ways_only()
                {
                    let what = format!("argument `{}` of `{}`", sig.names[i], sig.name);
                    self.need(&r, &vals[i], Env::Call(&vals, &targs), arg.span, what)?;
                }
            }
        }
        // A call through a value: nothing keeps what's at an invariant
        // position of what it's given, so nothing may be declared there.
        if resolved.is_none() && self.reachable {
            for (j, a) in args.iter().enumerate() {
                let t = self.ty_of(a);
                let (Some(v), p) = (vals.get(j), places.get(j).cloned().flatten()) else {
                    continue;
                };
                let written = match &p {
                    Some(p) => targs::under(&self.place_decl(p), 0),
                    None => self.written_tree(v, &t),
                };
                if self.declared_inv(&t, &written) {
                    return Err(Error::ty(
                        a.span,
                        format!(
                            "can't give `{}` to a call through a value: what's behind it has \
                             refinements of type arguments, which the call could write past",
                            crate::pretty::expr_text(a)
                        ),
                    ));
                }
            }
        }
        // Refinements of type arguments: what the call needs of them, and
        // says of its result and of what it's lent (`targs`).
        let trees = match &resolved {
            Some((s, t)) if self.reachable => {
                let spans: Vec<Span> = args.iter().map(|a| a.span).collect();
                let coerced: Vec<Option<(usize, Type)>> = (args.iter())
                    .map(|a| match self.cx.typed.coerced.get(&a.id) {
                        Some((n, _)) if *n > 0 => Some((*n, self.ty_of(a))),
                        _ => None,
                    })
                    .collect();
                self.call_trees(s, t, sig.as_ref(), &vals, &places, &spans, &coerced, e.span)?
            }
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
            return Ok(self.measure(s, recv.clone()));
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
        self.posts.insert(e.id, post);
        Ok(result)
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

    /// What call `e` gives once what it's lent is forgotten (see `call`):
    /// its postconditions, of the values behind what it lent to call-scoped
    /// `&mut`s, which are what it left there, as nothing it made from them
    /// outlives it; and the refinements of the struct of each field a
    /// refinement is about it lent, proved again.
    fn post(&mut self, e: &Expr) -> Result<(), Error> {
        let Some(post) = self.posts.remove(&e.id) else {
            return Ok(());
        };
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
                self.prove_fields(&q, i, e.span)?;
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
                self.need(r, &v, Env::Here, e.span, what.clone())?;
            }
            let tree: targs::Tree = own.into_iter().map(|d| (Vec::new(), d)).collect();
            self.need_tree(&v, &t, &tree, None, e.span, &what)?;
            self.reproving.retain(|q| *q != p);
            // Proved: so it's known of what's there now.
            if p.path.is_empty() {
                self.assume_invariant(p.root, &v);
            }
            self.assume_tree(&v, &t, &tree);
        }
        Ok(())
    }

    /// Fails if `x`, borrowed `&mut` at `span`, is a refined local, or a
    /// field a refinement is about, unless it's lent straight to a
    /// call-scoped parameter (`scoped`): then the refinements are proved
    /// again when the call returns (`Walk::post`), and until then nothing
    /// can read the struct, which the borrow is of a part of.
    fn no_mut_borrow(&self, x: &Expr, span: Span, scoped: bool) -> Result<(), Error> {
        if let (ExprKind::Var(name, _), Some(Res::Local(id))) =
            (&x.kind, self.cx.typed.vars.get(&x.id))
            && self.invariants.contains_key(id)
            && !scoped
        {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{name}` as `&mut`: its type has a refinement, which a write \
                     through the reference could break"
                ),
            ));
        }
        // Nor a part of one: a write to the part is a write to it.
        if let Some(name) = self.in_refined_local(x) {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{}` as `&mut`: it's in `{name}`, whose type has a refinement, \
                     which a write through the reference could break",
                    crate::pretty::expr_text(x)
                ),
            ));
        }
        // A field lent straight to a call-scoped `&mut` is proved again
        // after the call, at its place (`Walk::post`): one with no place, of
        // a temporary, has nowhere to be, and its destructor may count on
        // it.
        let why = match (scoped, self.place_of(x)) {
            (false, _) => self.constrained_field(x, false),
            (true, None) => self.constrained_field(x, true),
            (true, Some(_)) => None,
        };
        if let Some(why) = why {
            return Err(Error::ty(
                span,
                format!(
                    "can't borrow `{}` as `&mut`: {why}, which a write through the reference \
                     could break",
                    crate::pretty::expr_text(x)
                ),
            ));
        }
        Ok(())
    }

    /// The name of the refined local `x` is a part of, through fields and
    /// boxes, if it's one: not the local itself, nor through a reference,
    /// as what's behind one isn't part of its value (and no refinement of a
    /// `mut` variable names what's behind a `&mut`).
    fn in_refined_local(&self, x: &Expr) -> Option<String> {
        let mut x = match &x.kind {
            ExprKind::Field(..) | ExprKind::Unary(UnOp::Deref, _) => x,
            _ => return None,
        };
        loop {
            match &x.kind {
                ExprKind::Field(base, _) => {
                    let (derefs, _) = *self.cx.typed.fields.get(&x.id)?;
                    let mut t = self.ty_of(base);
                    for _ in 0..derefs {
                        t = match t {
                            Type::Box(inner) => self.cx.solved(&inner),
                            _ => return None,
                        };
                    }
                    x = base;
                }
                ExprKind::Unary(UnOp::Deref, base) if matches!(self.ty_of(base), Type::Box(_)) => {
                    x = base;
                }
                ExprKind::Var(_, _) => {
                    let Some(Res::Local(id)) = self.cx.typed.vars.get(&x.id) else {
                        return None;
                    };
                    return self.invariants.get(id).map(|(_, name)| name.clone());
                }
                _ => return None,
            }
        }
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
        place: &Expr,
        new: &Val,
        ty: &Type,
        span: Span,
    ) -> Result<(), Error> {
        let d = self.place_decl(p);
        let what = match (self.boxes_only(p), self.cx.typed.binds.get(&p.root)) {
            (true, Some(b)) => format!("`{}`", b.name),
            _ => format!(
                "what's written through `{}`",
                crate::pretty::expr_text(place)
            ),
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

    /// If `x` is a field a refinement is about, why it can't be borrowed
    /// `&mut`.
    ///
    /// Or a place in such a field, through fields and boxes: a write to it
    /// changes the field. Not through a reference: what's behind one isn't
    /// part of the value, and no refinement can name what's behind a
    /// `&mut`, nor anything write through a `&`: only a cell's `replace`
    /// does, and no refinement can name what's in a cell (lemma K1 of
    /// docs/37-cell.md). `unplaced`: of a struct at no place, lent to a
    /// call (`RefinedFields::constrained_unplaced`).
    fn constrained_field(&self, x: &Expr, unplaced: bool) -> Option<String> {
        let (outer, mut x) = (x, x);
        loop {
            match &x.kind {
                ExprKind::Field(base, _) => {
                    let (derefs, i) = *self.cx.typed.fields.get(&x.id)?;
                    let mut t = self.ty_of(base);
                    let mut through_ref = false;
                    for _ in 0..derefs {
                        t = match t {
                            Type::Box(inner) => self.cx.solved(&inner),
                            Type::Ref(_, _, inner) => {
                                through_ref = true;
                                self.cx.solved(&inner)
                            }
                            _ => return None,
                        };
                    }
                    let why = self.refined_fields(&t).and_then(|f| match unplaced {
                        true => f.constrained_unplaced(i),
                        false => f.constrained(i),
                    });
                    if let Some(why) = why {
                        return Some(match std::ptr::eq(x, outer) {
                            true => why,
                            false => format!("it's in `{}`: {why}", crate::pretty::expr_text(x)),
                        });
                    }
                    if through_ref {
                        return None;
                    }
                    x = base;
                }
                ExprKind::Unary(UnOp::Deref, inner)
                    if matches!(self.ty_of(inner), Type::Box(_)) =>
                {
                    x = inner;
                }
                _ => return None,
            }
        }
    }

    /// The fields that place `p` is, or is in, that a refinement is
    /// about, innermost first, each as its struct's place and its index:
    /// through fields and boxes, not references (see `constrained_field`).
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

    /// The condition `c` as a formula, and what it is if it isn't a `bool`.
    fn cond(&mut self, c: &Expr) -> Result<Formula, Error> {
        Ok(match self.expr(c)? {
            Val::Bool(f) => f,
            _ => Formula::holds(self.fresh(Kind::Bool)),
        })
    }

    fn expr(&mut self, e: &Expr) -> Result<Val, Error> {
        let v = self.expr_here(e)?;
        self.returned(e);
        self.post(e)?;
        Ok(v)
    }

    fn expr_here(&mut self, e: &Expr) -> Result<Val, Error> {
        match &e.kind {
            ExprKind::Int(n, _) => Ok(Val::Int(Lin::constant(i128::from(*n)))),
            ExprKind::Char(c) => Ok(Val::Int(Lin::constant(i128::from(u32::from(*c))))),
            ExprKind::Bool(b) => Ok(Val::Bool(if *b { Formula::True } else { Formula::False })),
            ExprKind::Unit | ExprKind::ByteStr(_) => Ok(Val::Other),
            // A string literal: a reference no one else has, as long as
            // its text is.
            ExprKind::Str(text) => {
                let o = self.new_obj();
                let v = Val::Ref(o, Pointee::Unknown);
                if self.cx.measures.contains_key(STR_LEN)
                    && let Ok(n) = i128::try_from(text.len())
                    && let Val::Int(len) = self.measure(STR_LEN, v.clone())
                    && let Some(f) = Formula::eq(&len, &Lin::constant(n))
                {
                    self.fact(f);
                }
                Ok(v)
            }
            ExprKind::Var(..) => match self.cx.typed.vars.get(&e.id) {
                Some(Res::Local(id)) => Ok(self.read_place(&Place::local(*id))),
                Some(Res::Fn(symbol, _)) => {
                    self.as_value(symbol, e.span)?;
                    Ok(Val::Other)
                }
                Some(Res::Const(name)) => Ok(self.length(&Type::Param(name.clone(), false))),
                // A const: what's known of its value, or else what its type
                // says, as of a call of a function with no parameters.
                Some(Res::Item(symbol)) => {
                    let symbol = symbol.clone();
                    Ok(self.const_item(&symbol))
                }
                _ => Ok(Val::Other),
            },
            // What's known of its elements is what's known of the values
            // it's made of.
            ExprKind::Array(items) => {
                let mut vals = Vec::new();
                for item in items {
                    vals.push(self.expr(item)?);
                }
                let v = Val::Array(self.new_obj(), Lin::constant(items.len() as i128));
                self.elements(&v, &self.ty_of(e), &vals);
                Ok(v)
            }
            ExprKind::Repeat(x, _) => {
                let item = self.expr(x)?;
                let ty = self.ty_of(e);
                let v = self.unknown(&ty);
                self.elements(&v, &ty, &[item]);
                Ok(v)
            }
            ExprKind::Unary(UnOp::Neg, x) => {
                let v = self.expr(x)?;
                let ty = self.ty_of(e);
                let Some(neg) = (match v {
                    Val::Int(a) => a.scale(-1),
                    _ => None,
                }) else {
                    if self.reachable {
                        self.operator_proof(
                            e.id,
                            None,
                            Failure::ArithmeticLimit,
                            &format!("the negated value must fit in `{ty}`"),
                        );
                    }
                    return Ok(self.unknown(&ty));
                };
                let goal = self.fits(&neg, &ty);
                self.operator_proof(
                    e.id,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the negated value must fit in `{ty}`"),
                );
                Ok(Val::Int(neg))
            }
            // The same number, if it fits. A character's is a scalar
            // value, and one fits in a `char` if it's one.
            ExprKind::As(x, _) => {
                let v = self.expr(x)?;
                let ty = self.ty_of(e);
                let Val::Int(a) = v else {
                    return Ok(self.unknown(&ty));
                };
                if self.ty_of(x) == Type::Char
                    && let Some(law) = Self::scalar(&a)
                {
                    self.laws.push(law);
                }
                let goal = match ty {
                    Type::Char => Self::scalar(&a),
                    _ => self.fits(&a, &ty),
                };
                self.operator_proof(
                    e.id,
                    goal,
                    Failure::ArithmeticLimit,
                    &format!("the converted value must fit in `{ty}`"),
                );
                Ok(Val::Int(a))
            }
            // Of an integer, the least plus the greatest minus it: each bit
            // flipped, which is linear.
            ExprKind::Unary(UnOp::Not, x) => {
                let v = self.expr(x)?;
                let ty = self.ty_of(e);
                let flipped = match (v, self.range(&ty)) {
                    (Val::Bool(f), _) => return Ok(Val::Bool(f.not())),
                    (Val::Int(a), Some((min, max))) => min.add(&max).and_then(|m| m.sub(&a)),
                    _ => None,
                };
                Ok(match flipped {
                    Some(v) => Val::Int(v),
                    None => self.unknown(&ty),
                })
            }
            ExprKind::Unary(UnOp::Deref, x) => match self.place_of(e) {
                Some(p) => Ok(self.read_place(&p)),
                None => {
                    let v = self.expr(x)?;
                    let ty = self.ty_of(e);
                    let got = match self.pointee(v.clone()) {
                        Val::Other => self.unknown(&ty),
                        v => v,
                    };
                    // What the reference says of what it points to.
                    if let Val::Ref(o, _) = &v
                        && let Some(tree) = self.trees.get(o).cloned()
                    {
                        self.assume_tree(&got, &ty, &targs::below(&tree, 0));
                    }
                    Ok(got)
                }
            },
            // A reference to a place, or to a value. One to what a
            // reference points to is that reference again, the same
            // object: what a slice's length is, say.
            ExprKind::Unary(op, x) => {
                if *op == UnOp::RefMut {
                    let scoped = self.scoped_args.contains(&e.id);
                    self.no_mut_borrow(x, e.span, scoped)?;
                }
                self.borrow(x)
            }
            ExprKind::Binary(op @ (BinOp::And | BinOp::Or), l, r) => {
                let c = self.cond(l)?;
                let start = self.snapshot();
                let and = *op == BinOp::And;
                let right = self.branch(
                    &start,
                    Some(if and { c.clone() } else { c.clone().not() }),
                    |w| w.cond(r).map(Val::Bool),
                )?;
                let short = Val::Bool(if and { Formula::False } else { Formula::True });
                let left =
                    self.branch(&start, Some(if and { c.not() } else { c }), |_| Ok(short))?;
                Ok(self.join(&start, vec![right, left], &Type::Bool))
            }
            ExprKind::Binary(op, l, r) => {
                let a = self.expr(l)?;
                let b = self.expr(r)?;
                Ok(match (op, a, b) {
                    // An `Ord` call says nothing the logic knows: only the
                    // machine's comparison of integers does.
                    _ if self.cx.is_ord_call(e.id) => self.unknown(&Type::Bool),
                    (op, a, b) if op.arith() || op.bitwise() => {
                        let ty = self.ty_of(e);
                        self.arith(e.id, *op, a, b, &ty)
                    }
                    (_, Val::Int(a), Val::Int(b)) => match compare(*op, &a, &b) {
                        Some(f) => Val::Bool(f),
                        None => self.unknown(&Type::Bool),
                    },
                    (BinOp::Eq, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b)),
                    (BinOp::Ne, Val::Bool(a), Val::Bool(b)) => Val::Bool(Formula::iff(a, b).not()),
                    _ => self.unknown(&Type::Bool),
                })
            }
            ExprKind::Call(callee, args) => {
                let symbol = self.fn_named(callee).map(str::to_string);
                if symbol.is_none() {
                    self.expr(callee)?;
                }
                let scoped = match &symbol {
                    Some(s) => self.cx.scoped_params(&self.callee(e, s).0),
                    None => Vec::new(),
                };
                for (a, _) in args.iter().zip(&scoped).filter(|(_, s)| **s) {
                    self.scoped_args.insert(a.id);
                }
                let all: Vec<&Expr> = args.iter().collect();
                let vals = self.call_args(e, symbol.as_deref(), &all, Vec::new(), Vec::new())?;
                let places = args.iter().map(|a| self.passed(a)).collect();
                let args: Vec<&Expr> = args.iter().collect();
                self.call(e, symbol.as_deref(), &args, vals, places)
            }
            ExprKind::MethodCall(x, _, _, args) => {
                let method = self.cx.typed.methods.get(&e.id);
                let symbol = match method {
                    Some((Res::Fn(s, _), ..)) => Some(s.clone()),
                    _ => None,
                };
                let scoped = match &symbol {
                    Some(s) => self.cx.scoped_params(&self.callee(e, s).0),
                    None => Vec::new(),
                };
                if let Some((_, 0, Some(true))) = method {
                    self.no_mut_borrow(x, x.span, scoped.first() == Some(&true))?;
                }
                for (a, _) in args.iter().zip(scoped.iter().skip(1)).filter(|(_, s)| **s) {
                    self.scoped_args.insert(a.id);
                }
                let recv = match method.cloned() {
                    // A `&` or `&mut` of a receiver at no place a local is
                    // the root of says what's declared there, as `&x` does.
                    Some((_, 0, by @ Some(_))) if self.receiver(x, 0, by).is_none() => {
                        self.borrow(x)?
                    }
                    Some((_, derefs, by)) => {
                        let v = self.expr(x)?;
                        self.autoref(v, derefs, by)
                    }
                    None => self.expr(x)?,
                };
                let recv_place = match method {
                    Some((_, derefs, by)) => self.receiver(x, *derefs, *by),
                    None => None,
                };
                let rest: Vec<&Expr> = args.iter().collect();
                let vals =
                    self.call_args(e, symbol.as_deref(), &rest, vec![recv], vec![recv_place])?;
                let mut places = vec![match method {
                    Some((_, derefs, by)) => self.receiver(x, *derefs, *by),
                    None => None,
                }];
                places.extend(args.iter().map(|a| self.passed(a)));
                let all: Vec<&Expr> = std::iter::once(&**x).chain(args).collect();
                self.call(e, symbol.as_deref(), &all, vals, places)
            }
            ExprKind::Variant(_, _, _, args, parens) => {
                let symbol = self.fn_named(e).map(str::to_string);
                if let Some(s) = &symbol
                    && !parens
                {
                    self.as_value(s, e.span)?;
                }
                // A call by path, `List::push(&mut l, x)`, lends as one by
                // name does.
                if let (Some(s), true) = (&symbol, *parens) {
                    let scoped = self.cx.scoped_params(&self.callee(e, s).0);
                    for (a, _) in args.iter().zip(&scoped).filter(|(_, s)| **s) {
                        self.scoped_args.insert(a.id);
                    }
                }
                let all: Vec<&Expr> = args.iter().collect();
                let callee = symbol.as_deref().filter(|_| *parens);
                let vals = self.call_args(e, callee, &all, Vec::new(), Vec::new())?;
                let args: Vec<&Expr> = args.iter().collect();
                match &symbol {
                    Some(_) if *parens => {
                        let places = args.iter().map(|a| self.passed(a)).collect();
                        self.call(e, symbol.as_deref(), &args, vals, places)
                    }
                    // `Box::new(x)`: a box of `x` (`Checker::box_new`).
                    None if *parens
                        && let Type::Box(_) = self.ty_of(e)
                        && let [v] = &vals[..] =>
                    {
                        Ok(Val::Ref(self.new_obj(), Pointee::Val(Box::new(v.clone()))))
                    }
                    // A variant's value: which it is, and its fields.
                    None => {
                        let ty = self.ty_of(e);
                        let ExprKind::Variant(_, _, member, ..) = &e.kind else {
                            unreachable!("matched above")
                        };
                        let v = match &ty {
                            Type::Adt(symbol, ..) => self.variant_index(symbol, &member.name),
                            _ => None,
                        };
                        match v {
                            Some(v) => {
                                let spans: Vec<Span> = args.iter().map(|a| a.span).collect();
                                let spans = if spans.is_empty() {
                                    vec![e.span]
                                } else {
                                    spans
                                };
                                self.keep_fields_inv(&ty, Some(v), &vals, &[], &spans)?;
                                let o = self.new_obj();
                                self.enum_parts(o, &ty, Some((v, vals)));
                                Ok(Val::Obj(o))
                            }
                            None => Ok(self.unknown(&ty)),
                        }
                    }
                    _ => {
                        let ty = self.ty_of(e);
                        Ok(self.unknown(&ty))
                    }
                }
            }
            ExprKind::If(c, then, els) => {
                let c = self.cond(c)?;
                let start = self.snapshot();
                let a = self.branch(&start, Some(c.clone()), |w| w.expr(then))?;
                let b = self.branch(&start, Some(c.not()), |w| match els {
                    Some(els) => w.expr(els),
                    None => Ok(Val::Other),
                })?;
                let ty = self.ty_of(e);
                Ok(self.join(&start, vec![a, b], &ty))
            }
            ExprKind::Block(stmts, tail) => {
                for s in stmts {
                    self.stmt(s)?;
                }
                match tail {
                    Some(t) => self.expr(t),
                    None => Ok(Val::Other),
                }
            }
            ExprKind::Unsafe(block) => self.expr(block),
            ExprKind::Match(scrutinee, arms) => {
                let s = self.expr(scrutinee)?;
                let sty = self.ty_of(scrutinee);
                let place = self.place_of(scrutinee);
                let start = self.snapshot();
                let mut before: Vec<Formula> = Vec::new();
                let mut outcomes = Vec::new();
                for arm in arms {
                    let this = self.pattern_formula(&arm.pat, &s);
                    // What matching implies but isn't the same as: true of
                    // this arm, so assumed in it, but not false of the arms
                    // after, so not in `before`.
                    let implied = self.pattern_implies(&arm.pat, &s);
                    let assume = Formula::and(
                        (before.iter().cloned().map(Formula::not))
                            .chain(this.clone())
                            .chain(implied),
                    );
                    let o = self.branch(&start, Some(assume), |w| {
                        w.bind_pattern(&arm.pat, Some(&s), place.clone(), None, Some(&sty))?;
                        w.expr(&arm.body)
                    })?;
                    outcomes.push(o);
                    before.extend(this);
                }
                let ty = self.ty_of(e);
                Ok(self.join(&start, outcomes, &ty))
            }
            ExprKind::Lambda(l) => {
                let start = self.snapshot();
                let loops = std::mem::take(&mut self.loops);
                self.closures += 1;
                let assumed = self.closure_params.remove(&l.id).unwrap_or_default();
                if let Type::Adt(symbol, ..) = self.ty_of(e) {
                    self.closure_decls.insert(symbol, assumed.clone());
                }
                for (i, p) in l.params.iter().enumerate() {
                    if let Some(tree) = assumed.get(i).filter(|t| !t.is_empty()) {
                        self.decl.insert(p.name.id, tree.clone());
                    }
                    self.bind(p.name.id, None);
                }
                let result = self.expr(&l.body);
                self.closures -= 1;
                self.loops = loops;
                self.restore(&start);
                result?;
                Ok(Val::Other)
            }
            ExprKind::Assign(op, place, value) => {
                let v = self.expr(value)?;
                let target = self.place_of(place);
                let ty = self.ty_of(place);
                let old = |w: &mut Self| match &target {
                    Some(p) => w.read_place(p),
                    None => w.unknown(&ty),
                };
                match (&place.kind, self.cx.typed.vars.get(&place.id)) {
                    (ExprKind::Var(name, _), Some(Res::Local(id))) => {
                        let id = *id;
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = old(self);
                                self.arith(e.id, *op, old, v, &ty)
                            }
                        };
                        if let Some((r, _)) = self.invariants.get(&id).cloned() {
                            let what = format!("`{name}`");
                            self.need(&r, &new, Env::Here, value.span, what)?;
                        }
                        let d = self.decl.get(&id).cloned().unwrap_or_default();
                        let what = format!("`{name}`");
                        let written = self.written_tree(&new, &ty);
                        self.need_tree(&new, &ty, &d, Some(&written), value.span, &what)?;
                        self.write(&Place::local(id), Some(new), Some(e.span));
                    }
                    (ExprKind::Field(base, _), _) => {
                        let (derefs, i) = self.cx.typed.fields[&place.id];
                        // The struct the field is of, and its fields' values;
                        // and, of one that isn't at a place a local is the
                        // root of, what's declared of the field (`lvalue`).
                        let (t, mut vals, declared) = match &target {
                            Some(p) => {
                                let (q, _) = p.field_of().expect("a field");
                                let t = self.place_ty(&q);
                                let vals = self.fields_of(&q, &t).unwrap_or_default();
                                (t, vals, None)
                            }
                            None => {
                                let (v, decl, t) = self.lvalue_derefs(base, derefs)?;
                                let vals = match v {
                                    Val::Struct(_, vals) => vals,
                                    _ => match self.unknown(&t) {
                                        Val::Struct(_, vals) => vals,
                                        _ => Vec::new(),
                                    },
                                };
                                let declared = self.field_decl(&decl, &t, i, None);
                                (t, vals, Some(declared))
                            }
                        };
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = vals.get(i).cloned().unwrap_or(Val::Other);
                                self.arith(e.id, *op, old, v, &ty)
                            }
                        };
                        if let Some(d) = &declared {
                            let what =
                                format!("what's written to `{}`", crate::pretty::expr_text(place));
                            let written = self.written_tree(&new, &ty);
                            self.need_tree(&new, &ty, d, Some(&written), value.span, &what)?;
                        }
                        if let Some(p) = &target {
                            self.keep_declared(p, place, &new, &ty, value.span)?;
                        }
                        if i < vals.len() {
                            vals[i] = new.clone();
                            self.need_fields(&t, &vals, Some(i), value.span)?;
                        }
                        if let Some(p) = &target {
                            self.around_before(p);
                            self.write(p, Some(new), Some(e.span));
                            self.around_after(p, value.span)?;
                            self.keep_invariant(p, value.span)?;
                        }
                    }
                    _ => {
                        // Through a reference that isn't a place's: what
                        // it says of what it points to, what's written
                        // must have.
                        let mut through = None;
                        if target.is_none() {
                            match &place.kind {
                                ExprKind::Unary(UnOp::Deref, inner) => {
                                    through = Some(self.expr(inner)?);
                                }
                                _ => {
                                    self.expr(place)?;
                                }
                            }
                        }
                        let new = match op {
                            None => v,
                            Some(op) => {
                                let old = old(self);
                                self.arith(e.id, *op, old, v, &ty)
                            }
                        };
                        if let Some(Val::Ref(o, _)) = &through
                            && let Some(tree) = self.trees.get(o).cloned()
                        {
                            let d = targs::below(&tree, 0);
                            let what = format!(
                                "what's written through `{}`",
                                crate::pretty::expr_text(place)
                            );
                            let written = self.written_tree(&new, &ty);
                            self.need_tree(&new, &ty, &d, Some(&written), value.span, &what)?;
                        }
                        if let Some(p) = &target {
                            self.keep_declared(p, place, &new, &ty, value.span)?;
                        }
                        if let Some(p) = &target {
                            self.around_before(p);
                            self.write(p, Some(new), Some(e.span));
                            self.around_after(p, value.span)?;
                            self.keep_invariant(p, value.span)?;
                        }
                    }
                }
                Ok(Val::Other)
            }
            // The Hoare rule, with what a loop's frame keeps (lemma I1 of
            // docs/54-loop-invariants.md): the invariant holds as the loop
            // starts; at its head each place it may change is a value
            // nothing's known of but its refinements, its frame and the
            // invariant; the body, from there, must make it hold again for
            // the next time round, at its end and at each `continue`. Its
            // exits, the condition false at the head and each `break`, meet
            // after it as branches do (lemma I2).
            ExprKind::While(label, c, inv, body) => {
                self.loop_head(e, inv.as_deref())?;
                let c = self.cond(c)?;
                let start = self.snapshot();
                self.loops.push(Loop {
                    label: label.as_ref().map(|l| l.name.name.clone()),
                    invariant: inv.as_deref().cloned(),
                    head: start.facts,
                    exits: Vec::new(),
                });
                self.fact(c.clone());
                let body = self
                    .expr(body)
                    .and_then(|_| self.again(inv.as_deref(), body.span));
                let lp = self.loops.pop().expect("pushed");
                body?;
                let done = Outcome {
                    facts: vec![c.not()],
                    cur: start.cur.clone(),
                    objs: start.objs.clone(),
                    reachable: start.reachable,
                    val: Val::Other,
                    invalidated: start.invalidated.clone(),
                };
                let exits = std::iter::once(done).chain(lp.exits).collect();
                self.join(&start, exits, &Type::Unit);
                Ok(Val::Other)
            }
            ExprKind::Loop(label, inv, body) => {
                self.loop_head(e, inv.as_deref())?;
                let start = self.snapshot();
                self.loops.push(Loop {
                    label: label.as_ref().map(|l| l.name.name.clone()),
                    invariant: inv.as_deref().cloned(),
                    head: start.facts,
                    exits: Vec::new(),
                });
                let body = self
                    .expr(body)
                    .and_then(|_| self.again(inv.as_deref(), body.span));
                let lp = self.loops.pop().expect("pushed");
                body?;
                let ty = self.ty_of(e);
                Ok(self.join(&start, lp.exits, &ty))
            }
            ExprKind::Return(x) => {
                let v = match x {
                    Some(x) => self.expr(x)?,
                    None => Val::Other,
                };
                let span = x.as_ref().map_or(e.span, |x| x.span);
                self.result(v, span)?;
                self.reachable = false;
                Ok(Val::Other)
            }
            // A way out of the loop it's for: how things are here is one of
            // the ways they may be after it.
            ExprKind::Break(label, x) => {
                let val = match x {
                    Some(x) => self.expr(x)?,
                    None => Val::Other,
                };
                if let Some(i) = self.target(label)
                    && self.reachable
                {
                    let head = self.loops[i].head;
                    let exit = Outcome {
                        facts: self.facts[head..].to_vec(),
                        cur: self.cur.clone(),
                        objs: self.objs.clone(),
                        reachable: true,
                        val,
                        invalidated: self.invalidated.clone(),
                    };
                    self.loops[i].exits.push(exit);
                }
                self.reachable = false;
                Ok(Val::Other)
            }
            // Round again: the loop's invariant must hold here.
            ExprKind::Continue(label) => {
                if let Some(i) = self.target(label)
                    && let Some(p) = self.loops[i].invariant.clone()
                {
                    let r = Refinement::whole(p);
                    let what = "the loop's invariant, at this `continue`".to_string();
                    self.need(&r, &Val::Other, Env::Here, e.span, what)?;
                }
                self.reachable = false;
                Ok(Val::Other)
            }
            ExprKind::StructLit(_, _, inits) => {
                let ty = self.ty_of(e);
                let names = struct_fields(&self.cx.adts, &ty).unwrap_or_default();
                let mut vals = vec![Val::Other; names.len()];
                let mut spans = vec![e.span; names.len()];
                for init in inits {
                    let v = self.expr(&init.value)?;
                    if let Some(i) = names.iter().position(|(n, _)| *n == init.name.name) {
                        vals[i] = v;
                        spans[i] = init.value.span;
                    }
                }
                if let Some(fields) = self.refined_fields(&ty).cloned() {
                    for (j, r) in fields.refs.iter().enumerate() {
                        if let Some(r) = r {
                            let what = format!("field `{}` of `{}`", fields.names[j], fields.name);
                            let args = self.adt_args(&ty);
                            self.need(r, &vals[j], Env::Fields(&vals, &args), spans[j], what)?;
                        }
                    }
                }
                let field_names: Vec<String> = names.iter().map(|(n, _)| n.clone()).collect();
                self.keep_fields_inv(&ty, None, &vals, &field_names, &spans)?;
                Ok(Val::Struct(self.new_obj(), vals))
            }
            // A struct's value, its fields what the values are (lemma T5 of
            // docs/58-tuples.md). No field of one has a refinement of its
            // own to check.
            ExprKind::Tuple(items) => {
                let mut vals = Vec::new();
                for item in items {
                    vals.push(self.expr(item)?);
                }
                Ok(Val::Struct(self.new_obj(), vals))
            }
            ExprKind::Field(x, _) => {
                if let Some(p) = self.place_of(e) {
                    return Ok(self.read_place(&p));
                }
                let mut v = self.expr(x)?;
                let (derefs, i) = self.cx.typed.fields[&e.id];
                for _ in 0..derefs {
                    v = self.pointee(v);
                }
                if let Val::Struct(_, mut vals) = v
                    && i < vals.len()
                {
                    return Ok(vals.swap_remove(i));
                }
                let mut t = self.ty_of(x);
                for _ in 0..derefs {
                    if let Type::Ref(_, _, inner) | Type::Box(inner) = t {
                        t = self.cx.solved(&inner);
                    }
                }
                match self.unknown(&t) {
                    Val::Struct(_, mut vals) if i < vals.len() => Ok(vals.swap_remove(i)),
                    _ => {
                        let ty = self.ty_of(e);
                        Ok(self.unknown(&ty))
                    }
                }
            }
            ExprKind::For(_)
            | ExprKind::IfLet(..)
            | ExprKind::WhileLet(..)
            | ExprKind::Range(..)
            | ExprKind::Try(..)
            | ExprKind::Format(_)
            | ExprKind::Index(..) => {
                unreachable!("rewritten when paths are resolved")
            }
        }
    }

    /// The values of a call `e`'s arguments `args`, after `first` (a
    /// method's receiver, whose places are `first_places`), of function
    /// `symbol` if it's one: each closure literal walked after the rest,
    /// counting on what they say of its parameters
    /// (`targs::closure_trees`). Making a closure runs nothing, so it's
    /// the same as walking it where it is.
    fn call_args(
        &mut self,
        e: &Expr,
        symbol: Option<&str>,
        args: &[&Expr],
        first: Vec<Val>,
        first_places: Vec<Option<Place>>,
    ) -> Result<Vec<Val>, Error> {
        let offset = first.len();
        let mut vals = first;
        let mut closures = Vec::new();
        for (i, a) in args.iter().enumerate() {
            match (&a.kind, symbol) {
                (ExprKind::Lambda(_), Some(_)) => {
                    vals.push(Val::Other);
                    closures.push(i + offset);
                }
                _ => vals.push(self.expr(a)?),
            }
        }
        if let (Some(symbol), false) = (symbol, closures.is_empty()) {
            let (s, targs) = self.callee(e, symbol);
            let mut places = first_places;
            places.extend(args.iter().map(|a| self.passed(a)));
            let trees = self.closure_trees(&s, &targs, &vals, &places, &closures);
            for j in closures {
                let a = args[j - offset];
                if let (ExprKind::Lambda(l), Some(t)) = (&a.kind, trees.get(&j)) {
                    self.closure_params.insert(l.id, t.clone());
                }
                vals[j] = self.expr(a)?;
            }
        }
        Ok(vals)
    }

    /// Binds the variables of `pat`, which matched `s` if it's known, at
    /// `place` if it's one: a variable bound by value to an `i64`, a
    /// `bool` or a struct is equal to what it's bound to, and one bound by
    /// `&` to a place points to it. `of` is the field of a struct `pat`
    /// matches, by the struct's type, if it matches one.
    fn bind_pattern(
        &mut self,
        pat: &Pattern,
        s: Option<&Val>,
        mut place: Option<Place>,
        of: Option<(&Type, usize)>,
        ty: Option<&Type>,
    ) -> Result<(), Error> {
        let mut s = s.filter(|v| !matches!(v, Val::Other)).cloned();
        let unpeeled = s.clone();
        let peels = self.cx.typed.peels.get(&pat.id).copied().unwrap_or(0);
        let mut ty = ty.map(|t| self.cx.solved(t));
        if peels > 0 {
            // Through a shared reference a call gave, of which the walk
            // knows only what it says of what it points to: a value that
            // can't change while it's borrowed, which has that (`targs`).
            let mut through = match place {
                Some(_) => None,
                None => s.clone(),
            };
            // The derefs left for `place` to take, once it's the place a
            // reference given as a value points to (`match &t { .. }`).
            let mut left = peels;
            for k in 0..peels {
                if let (Some(Val::Ref(_, Pointee::Place(p))), Some(Type::Ref(_, _, inner))) =
                    (&through, &ty)
                {
                    place = Some(p.clone());
                    ty = Some(self.cx.solved(inner));
                    through = None;
                    left = peels - k - 1;
                    continue;
                }
                let next = match (&through, &ty) {
                    (Some(Val::Ref(_, Pointee::Val(w))), Some(Type::Ref(_, _, inner))) => {
                        Some(((**w).clone(), self.cx.solved(inner)))
                    }
                    (Some(Val::Ref(o, Pointee::Unknown)), Some(Type::Ref(_, false, inner))) => {
                        let inner = self.cx.solved(inner);
                        let w = self.unknown(&inner);
                        if let Some(tree) = self.trees.get(o).cloned() {
                            self.assume_tree(&w, &inner, &targs::below(&tree, 0));
                        }
                        Some((w, inner))
                    }
                    _ => None,
                };
                through = next.as_ref().map(|(w, _)| w.clone());
                ty = match (next, &ty) {
                    (Some((_, inner)), _) => Some(inner),
                    (None, Some(Type::Ref(_, _, inner))) => Some(self.cx.solved(inner)),
                    _ => None,
                };
            }
            s = through;
            place = place.map(|p| (0..left).fold(p, |p, _| self.deref(p)));
        }
        // A variant's field, an array's element and what an alternative of
        // an or-pattern binds have no place of their own here, nor has a
        // field of what isn't at a place, so one bound by `&mut` could be
        // written through unchecked: not where anything is declared or
        // carried of the value (lemma P2 of docs/62-t-chess-findings.md).
        let placeless = match &pat.kind {
            PatternKind::Variant(..) | PatternKind::Array(_) | PatternKind::Or(_) => true,
            PatternKind::Tuple(_) | PatternKind::Struct(..) => place.is_none(),
            _ => false,
        };
        if placeless
            && let Some(name) = self.bound_mut(pat)
            && self.said_of(place.as_ref(), unpeeled.as_ref())
        {
            return Err(Error::ty(
                pat.span,
                format!(
                    "can't bind `{name}` by `&mut`: what it's in has a refinement, which a write \
                     through the reference could break"
                ),
            ));
        }
        match &pat.kind {
            PatternKind::Path(..) => unreachable!("resolved into a variant (`check::modules`)"),
            PatternKind::Bind(name, _) => {
                let id = self.cx.typed.sites.get(&pat.id).copied().unwrap_or(pat.id);
                match self.cx.typed.binds.get(&id).and_then(|b| b.by_ref) {
                    None => {
                        let val = match (s, &place) {
                            (Some(v), _) => Some(v),
                            (None, Some(p)) => Some(self.read_place(p)),
                            (None, None) => None,
                        };
                        // Without a refinement, it declares what its
                        // value's type has (`targs::widen`).
                        if let Some(v) = &val
                            && !self.decl.contains_key(&id)
                        {
                            let t = self.ty_of_bind(id);
                            let known = self.knowledge(v, &t);
                            let tree = targs::widen(&known);
                            if !tree.is_empty() {
                                self.decl.insert(id, tree);
                            }
                        }
                        self.bind(id, val);
                    }
                    Some(m) => {
                        if m && let Some((t, i)) = of
                            && let Some(why) = self.refined_fields(t).and_then(|f| f.constrained(i))
                        {
                            return Err(Error::ty(
                                pat.span,
                                format!(
                                    "can't bind `{name}` by `&mut`: {why}, which a write through \
                                     the reference could break"
                                ),
                            ));
                        }
                        match (m, place, unpeeled) {
                            (false, Some(p), _) => {
                                self.bind(id, None);
                                self.alias.insert(id, p);
                            }
                            // As `let r = &mut p;`: what's written through it
                            // must be what's declared of `p` (lemma P2 of
                            // docs/62-t-chess-findings.md).
                            (true, Some(p), _) => {
                                let o = self.new_obj();
                                let tree = targs::under(&self.place_decl(&p), 0);
                                if !tree.is_empty() {
                                    self.trees.insert(o, tree);
                                }
                                let r = Val::Ref(o, Pointee::Place(p));
                                if !self.decl.contains_key(&id) {
                                    let t = self.ty_of_bind(id);
                                    let known = self.knowledge(&r, &t);
                                    let tree = targs::widen(&known);
                                    if !tree.is_empty() {
                                        self.decl.insert(id, tree);
                                    }
                                }
                                self.bind(id, Some(r));
                            }
                            // A `&` to a value known, a variant's field.
                            (false, None, Some(v)) => {
                                let o = self.new_obj();
                                let r = Val::Ref(o, Pointee::Val(Box::new(v)));
                                if !self.decl.contains_key(&id) {
                                    let t = self.ty_of_bind(id);
                                    let known = self.knowledge(&r, &t);
                                    let tree = targs::widen(&known);
                                    if !tree.is_empty() {
                                        self.decl.insert(id, tree);
                                    }
                                }
                                self.bind(id, Some(r));
                            }
                            _ => self.bind(id, None),
                        }
                    }
                }
            }
            PatternKind::Struct(_, _, fields, _) => {
                let t = match self.cx.typed.struct_pats.get(&pat.id) {
                    Some(t) => self.cx.solved(t),
                    None => Type::Unit,
                };
                let names = struct_fields(&self.cx.adts, &t).unwrap_or_default();
                let vals = match (s, &place) {
                    (Some(Val::Struct(_, vals)), _) => vals,
                    (_, Some(p)) => match self.read_place(p) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                    _ => match self.unknown(&t) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                };
                for f in fields {
                    let i = names.iter().position(|(n, _)| *n == f.name.name);
                    let val = i.and_then(|i| vals.get(i));
                    let at = i.and_then(|i| Some(place.as_ref()?.then(Proj::Field(i))));
                    let ft = i.and_then(|i| names.get(i)).map(|(_, ft)| ft.clone());
                    self.bind_pattern(&f.pat, val, at, i.map(|i| (&t, i)), ft.as_ref())?;
                }
            }
            // A variant's fields are what's known of them, if anything is:
            // what's behind the references match ergonomics looked through,
            // a value, which can't change while the borrow lasts.
            PatternKind::Variant(_, _, member, pats) => {
                let whole = match (unpeeled, &place) {
                    (Some(v), _) => Some(self.peeled(pat, &v)),
                    (None, Some(p)) => Some(self.read_place(p)),
                    (None, None) => None,
                };
                let fields = whole.and_then(|w| self.variant_parts(&w, &member.name));
                let ftys = match &ty {
                    Some(t @ Type::Adt(symbol, ..)) => {
                        match self.variant_index(symbol, &member.name) {
                            Some(v) => crate::types::variant_fields(&self.cx.adts, t, v),
                            None => Vec::new(),
                        }
                    }
                    _ => Vec::new(),
                };
                for (j, p) in pats.iter().enumerate() {
                    let f = fields.as_ref().map(|(_, f)| f[j].clone());
                    self.bind_pattern(p, f.as_ref(), None, None, ftys.get(j))?;
                }
            }
            // As a struct pattern mentioning every field, in order (lemma
            // T5 of docs/58-tuples.md).
            PatternKind::Tuple(pats) => {
                let t = match (&place, &ty) {
                    (Some(p), _) => self.place_ty(p),
                    (None, Some(t)) => t.clone(),
                    _ => Type::Unit,
                };
                let elems = match &t {
                    Type::Tuple(ts) => ts.iter().map(|e| self.cx.solved(e)).collect(),
                    _ => Vec::new(),
                };
                let vals = match (s, &place) {
                    (Some(Val::Struct(_, vals)), _) => vals,
                    (_, Some(p)) => match self.read_place(p) {
                        Val::Struct(_, vals) => vals,
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                for (i, q) in pats.iter().enumerate() {
                    let at = place.as_ref().map(|p| p.then(Proj::Field(i)));
                    let of = matches!(t, Type::Tuple(_)).then_some((&t, i));
                    self.bind_pattern(q, vals.get(i), at, of, elems.get(i))?;
                }
            }
            // What an array's values are isn't followed.
            PatternKind::Array(pats) => {
                let elem = match &ty {
                    Some(Type::Array(e, _)) => Some(self.cx.solved(e)),
                    _ => None,
                };
                for p in pats {
                    self.bind_pattern(p, None, None, None, elem.as_ref())?;
                }
            }
            PatternKind::Or(pats) => {
                for p in pats {
                    self.bind_pattern(p, None, None, of, ty.as_ref())?;
                }
            }
            PatternKind::Wild
            | PatternKind::Int(..)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Str(_)
            | PatternKind::Unit => {}
        }
        Ok(())
    }

    /// The name of a variable `pat` binds by `&mut`, if it binds one.
    fn bound_mut(&self, pat: &Pattern) -> Option<String> {
        match &pat.kind {
            PatternKind::Bind(name, _) => {
                let id = self.cx.typed.sites.get(&pat.id).copied().unwrap_or(pat.id);
                let by_ref = self.cx.typed.binds.get(&id).and_then(|b| b.by_ref);
                (by_ref == Some(true)).then(|| name.clone())
            }
            PatternKind::Variant(.., pats)
            | PatternKind::Tuple(pats)
            | PatternKind::Array(pats)
            | PatternKind::Or(pats)
            | PatternKind::Path(_, pats, _) => pats.iter().find_map(|p| self.bound_mut(p)),
            PatternKind::Struct(_, _, fields, _) => {
                fields.iter().find_map(|f| self.bound_mut(&f.pat))
            }
            PatternKind::Wild
            | PatternKind::Int(..)
            | PatternKind::Range(..)
            | PatternKind::Bool(_)
            | PatternKind::Str(_)
            | PatternKind::Unit => None,
        }
    }

    /// Whether anything is declared of the value at `place`, if it's one,
    /// or carried of `v`, if it's known: a refinement of the local it's in,
    /// of a struct field it's in, or of a type argument at any depth.
    fn said_of(&mut self, place: Option<&Place>, v: Option<&Val>) -> bool {
        if let Some(p) = place {
            let through_ref = p.path.iter().enumerate().any(|(k, proj)| {
                *proj == Proj::Deref && {
                    let q = Place {
                        root: p.root,
                        path: p.path[..k].to_vec(),
                    };
                    matches!(self.place_ty(&q), Type::Ref(..))
                }
            });
            if (!through_ref && self.invariants.contains_key(&p.root))
                || !self.constrained_around(p).is_empty()
                || self.place_decl_own(p).iter().any(|(_, d)| !d.is_any())
            {
                return true;
            }
        }
        v.and_then(targs::obj_of)
            .and_then(|o| self.trees.get(&o))
            .is_some_and(|t| t.iter().any(|(_, d)| !d.is_any()))
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), Error> {
        match s {
            Stmt::Let { pat, init, .. } => {
                let v = match init {
                    Some(init) => Some(self.expr(init)?),
                    None => None,
                };
                if let Some(r) = self.cx.refined_lets.get(&pat.id).cloned() {
                    let name = pat.binds().pop().unwrap_or_default();
                    let v = v.clone().unwrap_or(Val::Other);
                    let span = init.as_ref().map_or(pat.span, |i| i.span);
                    self.need(&r, &v, Env::Here, span, format!("`{name}`"))?;
                    self.declare(pat.id, &r, &name);
                }
                let place = init.as_ref().and_then(|i| self.place_of(i));
                match init {
                    Some(init) => {
                        let t = self.ty_of(init);
                        self.bind_pattern(pat, v.as_ref(), place, None, Some(&t))?
                    }
                    None => self.bind_pattern(pat, None, None, None, None)?,
                }
                // `let r = &p;`: `*r` is `p`, while `r` can be used.
                if let (Some(init), PatternKind::Bind(_, false)) = (init, &pat.kind)
                    && let ExprKind::Unary(UnOp::Ref, x) = &init.kind
                    && let Some(p) = self.place_of(x)
                {
                    let id = self.cx.typed.sites.get(&pat.id).copied().unwrap_or(pat.id);
                    self.alias.insert(id, p);
                }
                Ok(())
            }
            Stmt::Expr(e) | Stmt::BlockLike(e) => self.expr(e).map(drop),
            Stmt::Fn(_) | Stmt::Type(_) => Ok(()),
        }
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

/// What `q`, the result of `a / k` or `a % k` (`op`) for a constant `k > 0`,
/// is, which the logic can say, as it's linear: rounding towards zero, the
/// quotient is the `q` with `k·q <= a <= k·q + k - 1` if `a >= 0`, and `k·q
/// - k + 1 <= a <= k·q` if `a <= 0`; the remainder has the sign of `a`, and
/// is less than `k` from 0. `None` if a number gets too big.
fn divided(op: BinOp, a: &Lin, k: i128, q: &Lin) -> Option<Formula> {
    let zero = Lin::constant(0);
    let (pos, neg) = (Formula::le(&zero, a)?, Formula::le(a, &zero)?);
    let (when_pos, when_neg) = match op {
        BinOp::Div => {
            let kq = q.scale(k)?;
            (
                Formula::and([Formula::le(&kq, a)?, Formula::le(a, &kq.plus(k - 1)?)?]),
                Formula::and([Formula::le(&kq.plus(1 - k)?, a)?, Formula::le(a, &kq)?]),
            )
        }
        BinOp::Rem => (
            Formula::and([
                Formula::le(&zero, q)?,
                Formula::le(q, &Lin::constant(k - 1))?,
            ]),
            Formula::and([
                Formula::le(&Lin::constant(1 - k), q)?,
                Formula::le(q, &zero)?,
            ]),
        ),
        _ => return None,
    };
    Some(Formula::and([
        Formula::or([pos.not(), when_pos]),
        Formula::or([neg.not(), when_neg]),
    ]))
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
