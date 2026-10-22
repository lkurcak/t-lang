//! Integer literals: that each fits in its type (see docs/11-integers.md).
//!
//! A literal's type is worked out from how it's used, so whether it fits
//! can only be checked once that's known, at the end of the function. In
//! generic code, of a type parameter `T: Int`, it must fit in every integer
//! type there is, so be from 0 to 127, as `T` may be any of them. Of
//! `usize` and `isize`, it must fit on every machine, so in 32 bits.

use super::Checker;
use crate::error::Error;
use crate::types::{IntTy, Type};

/// The values every `usize` and `isize` can hold, whatever the machine: a
/// 32-bit one's. What depends on it is a compiler's to say, not the
/// checker's, which proves only what holds on every machine (see
/// docs/11-integers.md).
pub(crate) fn portable_range(t: IntTy) -> (i128, i128) {
    match t {
        IntTy::Isize => IntTy::I32.range_64(),
        IntTy::Usize => IntTy::U32.range_64(),
        t => t.range_64(),
    }
}

/// What every integer type can hold: 0 to 127.
pub(crate) const EVERY_INT: (i128, i128) = (0, 127);

impl Checker {
    /// Checks that every literal of the function fits in its type, those
    /// whose types are known: the rest can't be inferred, which is said
    /// later.
    pub(super) fn literals(&mut self) -> Result<(), Error> {
        for l in std::mem::take(&mut self.literals) {
            self.info.literals.insert(l.node, self.solved(&l.ty));
            let (lo, hi) = match self.solved(&l.ty) {
                Type::Int(t) => portable_range(t),
                Type::Var(_) => continue,
                _ => EVERY_INT,
            };
            if (lo..=hi).contains(&l.value) {
                continue;
            }
            let message = match self.solved(&l.ty) {
                Type::Int(t @ (IntTy::Isize | IntTy::Usize))
                    if {
                        let (lo, hi) = t.range_64();
                        (lo..=hi).contains(&l.value)
                    } =>
                {
                    format!(
                        "`{}` may not fit in {}, which may be 32 bits wide, whose values are then \
                         {lo} to {hi}",
                        l.value,
                        t.a()
                    )
                }
                Type::Int(t) => format!(
                    "`{}` doesn't fit in {}, whose values are {lo} to {hi}",
                    l.value,
                    t.a()
                ),
                t => format!(
                    "`{}` may not fit in `{t}`: a literal of a type parameter must fit in every \
                     integer type, so be between 0 and 127",
                    l.value
                ),
            };
            return Err(Error::ty(l.span, message));
        }
        Ok(())
    }
}
