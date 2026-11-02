//! Const items, `const N: usize = 4;`, as far as checking goes (see
//! docs/36-consts.md). A const is a function of no parameters to the
//! checker, marked `konst`, whose body is its initializer. What's its own:
//!
//! - Its type must be *plain data* (`Checker::plain`): what can be worked
//!   out while compiling and then made again at each use, so nothing on
//!   the heap, nothing mutable, no function. Lemma C4.
//! - Its initializer may panic and do nothing else (`const_effects`):
//!   no `alloc`, no `unsafe`, and no I/O, which needs the `Os`. Whatever
//!   it does, then, the interpreter can do while compiling, which is when
//!   it runs (`crate::consts`): a panic there is an error, so a use has no
//!   effects at all. Lemma C1.
//! - A use of its name is its value, of its type, not a function
//!   (`Checker::lookup`), and isn't a pattern (`Checker::const_pattern`).

use super::Checker;
use super::scope::{Binding, FnId};
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{IntTy, Type, all_fields};
use std::collections::HashSet;

impl Checker {
    /// What the name `name`, a path or not, written at node `id`, refers
    /// to, as `lookup` finds it.
    pub(super) fn const_binding(&self, name: &str, id: NodeId) -> Option<Binding> {
        match name.contains("::") {
            true => match self.modules.paths.get(&id) {
                Some(super::modules::Item::Fn(i)) => Some(Binding::Fn(FnId(*i))),
                _ => None,
            },
            false => self.binding(name),
        }
    }

    /// Checks that const `f`, of type `ty`, is plain data.
    pub(super) fn const_type(&self, f: &FnDecl, ty: &Type) -> Result<(), Error> {
        if self.plain(ty, &mut HashSet::new()) {
            return Ok(());
        }
        Err(Error::ty(
            f.ret.span,
            format!(
                "a const can't be of type `{}`: a const is plain data, of integers, `bool`, \
                 `char`, `()`, `&str` and `&[u8]`, and arrays, structs and enums of those",
                f.ret
            ),
        ))
    }

    /// Is `t` plain data: a value with no address in it but a literal's,
    /// which the interpreter can give back as a tree and a use can make
    /// again? A `&'static` in a const can only point into a literal, as
    /// there are no statics and nothing else lives that long (lemma C4).
    /// `seen` are the structs and enums already being looked at, which a
    /// recursive one meets again.
    fn plain(&self, t: &Type, seen: &mut HashSet<Type>) -> bool {
        match t {
            Type::Int(_) | Type::Bool | Type::Char | Type::Unit => true,
            Type::Ref(_, false, to) => {
                matches!(**to, Type::Str) || **to == Type::Slice(Box::new(Type::Int(IntTy::U8)))
            }
            Type::Array(elem, _) => self.plain(elem, seen),
            Type::Adt(symbol, ..) => {
                let intrinsic =
                    (self.types.iter()).any(|d| d.symbol == *symbol && d.intrinsic.is_some());
                if intrinsic {
                    return false;
                }
                if !seen.insert(t.clone()) {
                    return true;
                }
                all_fields(&self.adts, t)
                    .iter()
                    .all(|f| self.plain(f, seen))
            }
            _ => false,
        }
    }

    /// Checks what const `f`'s initializer does, `effects` at `span` as
    /// `what` says: it may panic, and that's all (lemma C1).
    pub(super) fn const_effect(
        &self,
        f: &FnDecl,
        effect: &Type,
        what: &str,
        span: Span,
    ) -> Result<(), Error> {
        if *effect == Type::Panic {
            return Ok(());
        }
        Err(Error::ty(
            span,
            format!(
                "a const's initializer can't have the effect `{}`: {what} {}, and `{}` is \
                 worked out while the program is compiled, where only panicking is possible",
                crate::types::effects_text(&Type::effects([effect.clone()])),
                super::effects::does(&Type::effects([effect.clone()])),
                f.name.name,
            ),
        ))
    }

    /// The error for an `unsafe` block, at `span`, in const `f`'s
    /// initializer: what it promises is promised to no one while compiling.
    pub(super) fn const_unsafe(&self, f: &FnDecl, span: Span) -> Error {
        Error::ty(
            span,
            format!(
                "a const's initializer can't use `unsafe`: `{}` is worked out while the program \
                 is compiled, where nothing keeps what `unsafe` promises",
                f.name.name
            ),
        )
    }

    /// The const that `name`, a path or not, written at node `id` in a
    /// predicate, names, if it names one: its symbol, its type, and where
    /// it's declared.
    pub(super) fn pred_const(&self, name: &str, id: NodeId) -> Option<(String, Type, Span)> {
        let i = match name.contains("::") {
            true => match self.modules.paths.get(&id) {
                Some(super::modules::Item::Fn(i)) => *i,
                _ => return None,
            },
            false => self.modules.fn_named(self.module, name)?,
        };
        if !self.modules.consts.contains(&i) {
            return None;
        }
        let f = self.fns.get(i)?;
        let Type::Fn(sig) = &f.scheme.ty else {
            return None;
        };
        Some((f.symbol.clone(), (*sig.ret).clone(), f.name.span))
    }

    /// Checks that each const written as a length is a `usize`, as a
    /// length is: which can only be known once their types are, after the
    /// types it may be written in were resolved.
    pub(super) fn const_lengths(&self) -> Result<(), Error> {
        let mut lengths: Vec<_> = self.modules.lengths.values().collect();
        lengths.sort_by_key(|(_, at)| at.start);
        for &&(i, at) in &lengths {
            let f = &self.fns[i];
            let Type::Fn(sig) = &f.scheme.ty else {
                continue;
            };
            if *sig.ret != Type::Int(IntTy::Usize) {
                return Err(Error::ty(
                    at,
                    format!(
                        "`{}` is a const of type `{}`, but a length is a `usize`",
                        f.name.name, sig.ret
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The error for a pattern binding `name` at `span` where a const of
    /// that name is in scope: whether it binds or compares would depend on
    /// what's in scope, so it's neither yet, and the name is kept for
    /// comparing (see docs/36-consts.md).
    pub(super) fn const_pattern(&self, name: &str, span: Span) -> Error {
        Error::ty(
            span,
            format!(
                "`{name}` is a const: a pattern can't name one yet; to compare with it, bind \
                 another name and use `if x == {name}`"
            ),
        )
    }
}
