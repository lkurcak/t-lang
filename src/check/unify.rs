//! Type variables and unification: how the checker works out the types a
//! program doesn't write down (see docs/03-inference.md).

use super::Checker;
use super::scope::{declared_in, source_name};
use crate::error::{Error, Span};
use crate::types::Type;

/// A type variable: a type the checker hasn't worked out yet.
pub(super) struct Var {
    /// What it turned out to be, once `unify` finds out. That can be another
    /// variable, or a type with variables in it, like `fn(_) -> i64`.
    pub(super) solution: Option<Type>,
    /// Is it for what a type parameter stands for, in a use of something
    /// generic? Only for what its errors say: no variable is ever a slice.
    pub(super) param: bool,
    /// Is it the type of an integer literal, or of an operator's operands?
    /// Then it can only be an integer type, or a type parameter, opaque
    /// type or projection that implements `Int`, which an obligation made
    /// with it says (`Checker::fresh_int`).
    pub(super) int: bool,
    /// Where it came from, for when it's never solved: the span, what it's
    /// the type of ("the type of `x`") and how to fix that.
    span: Span,
    what: String,
    hint: String,
}

/// Why two types can't be made equal.
pub(super) enum Clash {
    /// Different shapes, like `i64` and `bool`.
    Mismatch,
    /// Variable `.0` would have to equal type `.1`, which contains it.
    Infinite(u32, Type),
    /// Variable `.0` would have to be the slice `.1`, which is only ever
    /// behind a reference, so is never a type a variable stands for: not
    /// the type of a value, nor what a type parameter is.
    Unsized(u32, Type),
}

/// Does `t` have a type variable in it?
pub(super) fn has_vars(t: &Type) -> bool {
    matches!(t, Type::Var(_)) || t.parts().any(has_vars)
}

/// If `a` and `b` mention two different types of the same name, which look
/// the same in a message, the rest of the message, which says so.
fn same_name(a: &Type, b: &Type) -> String {
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    opaques(a, &mut xs);
    opaques(b, &mut ys);
    let text = crate::types::opaque_text;
    if xs
        .iter()
        .any(|x| ys.iter().any(|y| x != y && text(x) == text(y)))
    {
        return "; each `impl` type is a type of its own, even if two look the same".to_string();
    }
    let mut symbols: Vec<&str> = Vec::new();
    for t in [a, b] {
        super::each_adt(t, &mut |s, _| {
            if !symbols.contains(&s) {
                symbols.push(s);
            }
        });
    }
    if symbols
        .iter()
        .filter(|s| crate::types::is_closure(s))
        .count()
        > 1
    {
        return "; each closure that captures has a type of its own, even if two look the same"
            .to_string();
    }
    symbols.retain(|s| !crate::types::is_closure(s));
    for (i, x) in symbols.iter().enumerate() {
        let name = source_name(x);
        if let Some(y) = symbols[i + 1..].iter().find(|y| source_name(y) == name) {
            let (x_in, y_in) = (declared_in(x), declared_in(y));
            return match x_in == y_in {
                true => format!(
                    "; these are two different types called `{name}`, declared in different \
                     blocks {x_in}"
                ),
                false => format!(
                    "; these are two different types called `{name}`, one declared {x_in} and \
                     one {y_in}"
                ),
            };
        }
    }
    String::new()
}

/// The symbols of the opaque types in `t`.
fn opaques<'t>(t: &'t Type, out: &mut Vec<&'t str>) {
    if let Type::Opaque(s, ..) = t {
        out.push(s);
    }
    t.parts().for_each(|p| opaques(p, out));
}

/// `t`, with every variable `v` in it called `name`. For explaining an
/// infinite type, which is easier with a name for the type that repeats.
fn name_var(t: &Type, v: u32, name: &str) -> Type {
    match t {
        Type::Var(w) if *w == v => Type::Param(name.to_string()),
        t => t.map(|p| name_var(p, v, name)),
    }
}

impl Checker {
    /// A new type variable, for the type of something at `span`. `what` and
    /// `hint` explain it if it's never solved, see `solve_all`.
    pub(super) fn fresh(&mut self, span: Span, what: String, hint: String) -> Type {
        self.vars.push(Var {
            solution: None,
            param: false,
            int: false,
            span,
            what,
            hint,
        });
        Type::Var(self.vars.len() as u32 - 1)
    }

    /// A new variable for an integer type, for the type of something at
    /// `span`: a literal's, or an operator's operands'. Whatever it turns out
    /// to be must implement `Int`, which only the integer types do, and
    /// type parameters bounded by it.
    pub(super) fn fresh_int(&mut self, span: Span, what: String, hint: String) -> Type {
        let t = self.fresh(span, what, hint);
        let Type::Var(v) = t else {
            unreachable!("a variable")
        };
        self.vars[v as usize].int = true;
        let pred = crate::traits::Pred {
            ty: t.clone(),
            trait_name: "Int".into(),
        };
        // Its type isn't known, so this only remembers it.
        let _ = self.oblige(pred, span, super::traits::Why::Integer);
        t
    }

    /// Is `t` a variable for an integer type, not known yet?
    pub(super) fn int_var(&self, t: &Type) -> bool {
        matches!(self.shallow(t), Type::Var(v) if self.vars[v as usize].int)
    }

    /// `t`, with each variable for an integer type that isn't known yet
    /// shown as `{integer}`, for a message.
    pub(super) fn shown(&self, t: &Type) -> Type {
        match self.shallow(t) {
            Type::Var(v) if self.vars[v as usize].int => Type::Param("{integer}".into()),
            t => t.map(|p| self.shown(p)),
        }
    }

    /// `t`, looking through variables that are solved, as far as the
    /// outermost type: `fn(_) -> _`, not necessarily `fn(i64) -> bool`.
    ///
    /// A projection whose type is known by now is worked out, so what's
    /// outermost is what it stands for: `<_ as Tr>::Out`, once `_` is
    /// `i64`, is what `impl Tr for i64` says.
    pub(super) fn shallow(&self, t: &Type) -> Type {
        let mut t = t.clone();
        loop {
            match t {
                Type::Var(v) => match &self.vars[v as usize].solution {
                    Some(s) => t = s.clone(),
                    None => return t,
                },
                Type::Proj(..) => {
                    let n = self.adts.normalise(&t.map(|p| self.solved(p)));
                    if n == t {
                        return t;
                    }
                    t = n;
                }
                t => return t,
            }
        }
    }

    /// `t` with every solved variable replaced by its solution, all the way
    /// down, and every projection worked out that can be by now. Any
    /// `Type::Var` left in it is unknown so far.
    pub(super) fn solved(&self, t: &Type) -> Type {
        let t = self.shallow(t).map(|p| self.solved(p));
        match t {
            Type::Proj(..) => self.adts.normalise(&t),
            t => t,
        }
    }

    /// Is `t` a projection on a type not known yet, which is what it stands
    /// for?
    fn unknown_proj(&self, t: &Type) -> bool {
        match t {
            Type::Proj(s, ..) => match self.shallow(s) {
                Type::Var(_) => true,
                s => self.unknown_proj(&s),
            },
            _ => false,
        }
    }

    /// Works out the goals whose types are known by now, and checks that
    /// each is what it must be.
    pub(super) fn settle_goals(&mut self) -> Result<(), Error> {
        loop {
            let goals = std::mem::take(&mut self.goals);
            let before = goals.len();
            for g in goals {
                let p = self.shallow(&g.proj);
                if self.unknown_proj(&p) {
                    self.goals.push(g);
                    continue;
                }
                // Not worked out, though its type is known: no impl, which
                // the obligation that the type implements the trait says.
                // Unless it's rigid: on a parameter, or an opaque type,
                // which implements what it does without saying what its
                // associated types are.
                if let Type::Proj(s, ..) = &p
                    && !matches!(**s, Type::Param(_) | Type::Proj(..) | Type::Opaque(..))
                {
                    continue;
                }
                // An `Fn` bound's signature: as said, but for the effects,
                // which are at most those it says (`traits::fits`).
                if crate::traits::is_sig(&g.proj)
                    && let (Type::Fn(have), Type::Fn(want)) = (&p, self.shallow(&g.is))
                {
                    let bare =
                        |f: &crate::types::FnType| Type::Fn(f.with_effects(Type::no_effects()));
                    self.unify_or(&bare(have), &bare(&want), g.span, |a, b| match &g.who {
                        Some(who) => format!("`{who}` needs calls of it to be `{b}`, not `{a}`"),
                        None => format!("expected `{b}`, found `{a}`"),
                    })?;
                    let why = super::effects::Why::Bound {
                        who: g.who.clone().unwrap_or_default(),
                        sig: g.is.clone(),
                    };
                    self.within(*have.effects.clone(), *want.effects, g.span, why);
                    continue;
                }
                // As written, with its type as it's known now.
                let proj = g.proj.map(|t| self.solved(t));
                self.unify_or(&p, &g.is, g.span, |a, b| match &g.who {
                    // Rigid: nothing says what it is.
                    Some(who) if *a == proj => {
                        let fix = match &proj {
                            Type::Proj(s, tr, n) if matches!(**s, Type::Param(_)) => {
                                format!("; say it is with a bound: `{s}: {tr}<{n} = {b}>`")
                            }
                            _ => String::new(),
                        };
                        format!("`{who}` needs `{proj}` to be `{b}`, and nothing says it is{fix}")
                    }
                    Some(who) => {
                        format!("`{proj}` is `{a}`, but `{who}` needs it to be `{b}`")
                    }
                    None => format!("expected `{b}`, found `{a}`"),
                })?;
            }
            if self.goals.len() == before {
                return Ok(());
            }
        }
    }

    /// Requires projection `lhs` to be `rhs`, as an equality bound of
    /// `who` says, at `span`.
    pub(super) fn require_eq(
        &mut self,
        lhs: Type,
        rhs: Type,
        span: Span,
        who: &str,
    ) -> Result<(), Error> {
        self.goals.push(super::Goal {
            proj: lhs,
            is: rhs,
            span,
            who: Some(who.to_string()),
        });
        self.settle_goals()
    }

    /// Does variable `v` occur in `t`?
    fn occurs(&self, v: u32, t: &Type) -> bool {
        match self.shallow(t) {
            Type::Var(w) => v == w,
            t => t.parts().any(|p| self.occurs(v, p)),
        }
    }

    /// Makes `a` and `b` the same type, by solving variables in them, or
    /// says why that's impossible. This is *unification* (Robinson, 1965):
    ///
    /// - A variable unifies with anything, by being solved as it. Unless it
    ///   occurs in that thing: `T = fn(T) -> i64` has no finite solution.
    /// - Two types of the same shape unify if their parts do: two function
    ///   types if their parameters and results do, `Option<A>` and
    ///   `Option<B>` if `A` and `B` do.
    /// - Anything else must be the very same type. A type parameter is only
    ///   ever the same as itself.
    ///
    /// Solving `T` as `i64` here holds for the rest of the function, so the
    /// order types are learned in doesn't matter, up to a point (see
    /// docs/03-inference.md).
    ///
    /// A projection on a type not known yet can't be unified with anything
    /// now, as what it stands for depends on that type. It waits, as a
    /// goal, until the type is (`settle_goals`). Unifying it with `i64`
    /// never makes its type a type whose impl says `i64`: that would make
    /// what a program means depend on which impls there are, and there may
    /// be several, or none yet.
    pub(super) fn unify(&mut self, a: &Type, b: &Type) -> Result<(), Clash> {
        let (sa, sb) = (self.shallow(a), self.shallow(b));
        if sa == sb {
            return Ok(());
        }
        for (p, other) in [(&sa, &sb), (&sb, &sa)] {
            if self.unknown_proj(p) && !matches!(other, Type::Var(_)) {
                self.goals.push(super::Goal {
                    proj: p.clone(),
                    is: other.clone(),
                    span: self.here,
                    who: None,
                });
                return Ok(());
            }
        }
        match (sa, sb) {
            (Type::Var(v), Type::Var(w)) if v == w => Ok(()),
            (a @ Type::Effects(_), b @ Type::Effects(_)) => self.unify_effects(&a, &b),
            (Type::Var(v), t) | (t, Type::Var(v)) => {
                if self.occurs(v, &t) {
                    return Err(Clash::Infinite(v, t));
                }
                match &t {
                    Type::Slice(_) => return Err(Clash::Unsized(v, t)),
                    Type::Var(w) => {
                        let (param, int) = (self.vars[v as usize].param, self.vars[v as usize].int);
                        let w = &mut self.vars[*w as usize];
                        w.param |= param;
                        w.int |= int;
                    }
                    // Its obligation, `Int`, says whether these may be.
                    Type::Int(_) | Type::Param(_) | Type::Opaque(..) | Type::Proj(..) => {}
                    _ if self.vars[v as usize].int => return Err(Clash::Mismatch),
                    _ => {}
                }
                self.vars[v as usize].solution = Some(t);
                Ok(())
            }
            (a, b) if a.same_shape(&b) => {
                for (p, q) in a.parts().zip(b.parts()) {
                    self.unify(p, q)?;
                }
                Ok(())
            }
            _ => Err(Clash::Mismatch),
        }
    }

    /// Unifies effect sets `a` and `b`. A set that's just a variable is the
    /// other; one with variables, or projections not worked out yet, waits
    /// for them, as two `Within`s, both ways; one with neither must be the
    /// same set as the other.
    fn unify_effects(&mut self, a: &Type, b: &Type) -> Result<(), Clash> {
        let (a, b) = (self.solved(a), self.solved(b));
        if a == b {
            return Ok(());
        }
        for (x, y) in [(&a, &b), (&b, &a)] {
            if let [Type::Var(v)] = x.effect_items()
                && !y.effect_items().contains(&Type::Var(*v))
            {
                self.vars[*v as usize].solution = Some(y.clone());
                return Ok(());
            }
        }
        let open = |t: &Type| {
            t.effect_items()
                .iter()
                .any(|e| matches!(e, Type::Var(_)) || self.unknown_proj(e))
        };
        if open(&a) || open(&b) {
            let why = super::effects::Why::Same;
            self.within(a.clone(), b.clone(), self.here, why.clone());
            self.within(b, a, self.here, why);
            return Ok(());
        }
        Err(Clash::Mismatch)
    }

    /// That variable `v` can't be `t`, a slice, at `span` (`Clash::Unsized`).
    pub(super) fn unsized_var(&self, v: u32, t: &Type, span: Span) -> Error {
        let t = self.solved(t);
        match self.vars[v as usize].param {
            true => Error::ty(
                span,
                format!(
                    "a type parameter can't stand for the slice `{t}`: it's only ever behind a \
                     reference"
                ),
            ),
            false => super::generics::unsized_slice(span, t),
        }
    }

    /// Unifies `a` and `b`. If they don't fit, the error at `span` says
    /// `mismatch(a, b)`, with what's known about them so far.
    pub(super) fn unify_or(
        &mut self,
        a: &Type,
        b: &Type,
        span: Span,
        mismatch: impl FnOnce(&Type, &Type) -> String,
    ) -> Result<(), Error> {
        match self.unify(a, b) {
            Ok(()) => Ok(()),
            Err(Clash::Mismatch) => {
                let (a, b) = (self.shown(&self.solved(a)), self.shown(&self.solved(b)));
                let message = mismatch(&a, &b) + &same_name(&a, &b);
                Err(Error::ty(span, message))
            }
            Err(Clash::Infinite(v, t)) => {
                let t = name_var(&self.solved(&t), v, "T");
                Err(Error::ty(
                    span,
                    format!(
                        "infinite type: this needs a type `T` that is `{t}`, which contains \
                         `T` itself"
                    ),
                ))
            }
            Err(Clash::Unsized(v, t)) => Err(self.unsized_var(v, &t, span)),
        }
    }

    /// At the end of a function: every variable made since `vars` must be
    /// solved (including those for the type arguments of generic things it
    /// uses), every obligation must hold, and no closure that captures may
    /// leave the frame that made it.
    ///
    /// Leaving a variable unsolved would be harmless to the interpreter, which
    /// never looks at types. But a compiler has to know how big a value is
    /// and how to pass it, so like Rust, T asks for an annotation.
    ///
    /// A type a `!` was wanted as that nothing decided is `!` (see `fits`).
    pub(super) fn solve_all(&mut self, vars: usize, f: &crate::ast::FnDecl) -> Result<(), Error> {
        self.settle_goals()?;
        self.literals()?;
        for t in std::mem::take(&mut self.diverging) {
            // An integer type can't be `!`: that's left to be inferred.
            if let Type::Var(v) = self.shallow(&t)
                && !self.vars[v as usize].int
            {
                self.unify(&t, &Type::Never)
                    .unwrap_or_else(|_| unreachable!("a variable fits anything"));
            }
        }
        // An integer type nothing in the body decided is never decided:
        // nothing after this unifies. Say so now, as what comes next needs
        // to know which it is: a closure's captures, for one.
        for v in vars..self.vars.len() {
            if self.int_var(&Type::Var(v as u32)) {
                return Err(self.unsolved(v));
            }
        }
        // Which operators can't panic, before effects are settled.
        self.refine(f)?;
        // Before what's worked out of the closures, which needs their
        // types without variables, effects and all.
        self.settle_effects(vars)?;
        self.settle_closures()?;
        self.prove_all()?;
        for v in vars..self.vars.len() {
            if has_vars(&self.solved(&Type::Var(v as u32))) {
                return Err(self.unsolved(v));
            }
        }
        self.settle_unused();
        Ok(())
    }

    /// That variable `v` is never solved.
    fn unsolved(&self, v: usize) -> Error {
        let t = self.shown(&self.solved(&Type::Var(v as u32)));
        let Var {
            span, what, hint, ..
        } = &self.vars[v];
        let message = match t {
            Type::Var(_) => format!("cannot infer {what}; {hint}"),
            Type::Param(p) if p == "{integer}" => format!("cannot infer {what}; {hint}"),
            t => format!("cannot fully infer {what}, only that it's `{t}`; {hint}"),
        };
        Error::ty(*span, message)
    }

    /// An unused variable whose type isn't `Discard` must still be used
    /// up, so renaming it `_x` would only hide the warning: it says so
    /// instead, with no fix.
    fn settle_unused(&mut self) {
        for (i, t) in std::mem::take(&mut self.unused) {
            let t = self.solved(&t);
            if has_vars(&t) || self.is_discard(&t) {
                continue;
            }
            let w = &mut self.warnings[i];
            w.message = format!(
                "{}: use it up, as its type, `{t}`, isn't `Discard`",
                w.message
            );
            w.fixes.clear();
        }
    }
}
