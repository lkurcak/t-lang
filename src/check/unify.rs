//! Type variables and unification: how the checker works out the types a
//! program doesn't write down (see docs/03-inference.md).

use super::Checker;
use super::scope::{declared_in, source_name};
use crate::error::{Error, Span};
use crate::types::Type;

/// A type variable: a type the checker hasn't worked out yet.
pub(super) struct Var {
    /// What it turned out to be, once `unify` finds out. That can be another
    /// variable, or a type with variables in it, like `fn(_) -> int`.
    solution: Option<Type>,
    /// Where it came from, for when it's never solved: the span, what it's
    /// the type of ("the type of `x`") and how to fix that.
    span: Span,
    what: String,
    hint: String,
}

/// Why two types can't be made equal.
pub(super) enum Clash {
    /// Different shapes, like `int` and `bool`.
    Mismatch,
    /// Variable `.0` would have to equal type `.1`, which contains it.
    Infinite(u32, Type),
}

/// Does `t` have a type variable in it?
pub(super) fn has_vars(t: &Type) -> bool {
    matches!(t, Type::Var(_)) || t.parts().any(has_vars)
}

/// If `a` and `b` mention two different types of the same name, which look
/// the same in a message, the rest of the message, which says so.
fn same_name(a: &Type, b: &Type) -> String {
    let mut symbols: Vec<&str> = Vec::new();
    for t in [a, b] {
        super::each_adt(t, &mut |s, _| {
            if !symbols.contains(&s) {
                symbols.push(s);
            }
        });
    }
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
            span,
            what,
            hint,
        });
        Type::Var(self.vars.len() as u32 - 1)
    }

    /// `t`, looking through variables that are solved, as far as the
    /// outermost type: `fn(_) -> _`, not necessarily `fn(int) -> bool`.
    pub(super) fn shallow(&self, t: &Type) -> Type {
        let mut t = t.clone();
        while let Type::Var(v) = t {
            match &self.vars[v as usize].solution {
                Some(s) => t = s.clone(),
                None => break,
            }
        }
        t
    }

    /// `t` with every solved variable replaced by its solution, all the way
    /// down. Any `Type::Var` left in it is unknown so far.
    pub(super) fn solved(&self, t: &Type) -> Type {
        self.shallow(t).map(|p| self.solved(p))
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
    ///   occurs in that thing: `T = fn(T) -> int` has no finite solution.
    /// - Two types of the same shape unify if their parts do: two function
    ///   types if their parameters and results do, `Option<A>` and
    ///   `Option<B>` if `A` and `B` do.
    /// - Anything else must be the very same type. A type parameter is only
    ///   ever the same as itself.
    ///
    /// Solving `T` as `int` here holds for the rest of the function, so the
    /// order types are learned in doesn't matter, up to a point (see
    /// docs/03-inference.md).
    pub(super) fn unify(&mut self, a: &Type, b: &Type) -> Result<(), Clash> {
        match (self.shallow(a), self.shallow(b)) {
            (Type::Var(v), Type::Var(w)) if v == w => Ok(()),
            (Type::Var(v), t) | (t, Type::Var(v)) => {
                if self.occurs(v, &t) {
                    return Err(Clash::Infinite(v, t));
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
                let (a, b) = (self.solved(a), self.solved(b));
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
        }
    }

    /// At the end of a function: every variable made since `vars` must be
    /// solved (including those for the type arguments of generic things it
    /// uses), every comparison must be on a type that allows it, and no
    /// closure that captures may leave the frame that made it.
    ///
    /// Leaving a variable unsolved would be harmless to the interpreter, which
    /// never looks at types. But a compiler has to know how big a value is
    /// and how to pass it, so like Rust, T asks for an annotation.
    ///
    /// A type a `!` was wanted as that nothing decided is `!` (see `fits`).
    pub(super) fn solve_all(&mut self, vars: usize) -> Result<(), Error> {
        for t in std::mem::take(&mut self.diverging) {
            if let Type::Var(_) = self.shallow(&t) {
                self.unify(&t, &Type::Never)
                    .unwrap_or_else(|_| unreachable!("a variable fits anything"));
            }
        }
        self.check_escapes()?;
        self.check_captures()?;
        for (t, span, op) in std::mem::take(&mut self.comparisons) {
            let t = self.solved(&t);
            // If it's still not fully known, that's reported below.
            if !has_vars(&t) {
                self.comparable(&t, span, op)?;
            }
        }
        for v in vars..self.vars.len() {
            let t = self.solved(&Type::Var(v as u32));
            if has_vars(&t) {
                let Var {
                    span, what, hint, ..
                } = &self.vars[v];
                let message = match t {
                    Type::Var(_) => format!("cannot infer {what}; {hint}"),
                    t => format!("cannot fully infer {what}, only that it's `{t}`; {hint}"),
                };
                return Err(Error::ty(*span, message));
            }
        }
        Ok(())
    }
}
