//! Floating-point values as the interpreter runs them: IEEE 754's binary32
//! and binary64, by their bits, with Rust's `f32` and `f64` doing each
//! operation, which round as IEEE 754 says, once (see docs/101-floats.md).
//!
//! A value is its bits, so two values are the same value when their bits
//! are (`Value`'s `==`, which the oracle's models compare): a NaN is the
//! same as itself, and `0.0` isn't `-0.0`. The program's `==` is IEEE's,
//! `equal` here, which is neither.

use crate::ast::BinOp;
use crate::types::FloatTy;

/// `x`'s bits, of type `t`, as Rust's float of that width.
fn f32_of(x: u64) -> f32 {
    f32::from_bits(u32::try_from(x).expect("an `f32`'s bits"))
}

fn f64_of(x: u64) -> f64 {
    f64::from_bits(x)
}

/// The bits of `x`, an `f32`.
fn bits32(x: f32) -> u64 {
    u64::from(x.to_bits())
}

/// `l op r`, of type `t`, for an arithmetic operator: IEEE's, rounded once.
pub(super) fn arith(op: BinOp, l: u64, r: u64, t: FloatTy) -> u64 {
    match t {
        FloatTy::F32 => {
            let (a, b) = (f32_of(l), f32_of(r));
            bits32(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b,
                _ => unreachable!("`{op}` isn't arithmetic of floats"),
            })
        }
        FloatTy::F64 => {
            let (a, b) = (f64_of(l), f64_of(r));
            (match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b,
                _ => unreachable!("`{op}` isn't arithmetic of floats"),
            })
            .to_bits()
        }
    }
}

/// `l op r`, of type `t`, for a comparison: IEEE's, false of a NaN but
/// for `!=`.
pub(super) fn compare(op: BinOp, l: u64, r: u64, t: FloatTy) -> bool {
    let (a, b) = match t {
        FloatTy::F32 => (f64::from(f32_of(l)), f64::from(f32_of(r))),
        FloatTy::F64 => (f64_of(l), f64_of(r)),
    };
    match op {
        BinOp::Eq => a == b,
        BinOp::Ne => a != b,
        BinOp::Lt => a < b,
        BinOp::Le => a <= b,
        BinOp::Gt => a > b,
        BinOp::Ge => a >= b,
        _ => unreachable!("`{op}` isn't a comparison"),
    }
}

/// `-x`, of type `t`: its sign flipped, the rest of its bits as they are.
pub(super) fn neg(x: u64, t: FloatTy) -> u64 {
    match t {
        FloatTy::F32 => x ^ (1 << 31),
        FloatTy::F64 => x ^ (1 << 63),
    }
}

/// `x`, of type `t`, as `f64`: exactly, as each `f32` is an `f64`.
fn wide(x: u64, t: FloatTy) -> f64 {
    match t {
        FloatTy::F32 => f64::from(f32_of(x)),
        FloatTy::F64 => f64_of(x),
    }
}

/// `x`, of type `from`, as a `to`, if it's one, as `as` converts: the
/// same number, or a NaN for a NaN (whose bits aren't said), or an
/// infinity for the same infinity.
pub(super) fn to_float(x: u64, from: FloatTy, to: FloatTy) -> Option<u64> {
    let w = wide(x, from);
    match to {
        FloatTy::F64 => Some(w.to_bits()),
        FloatTy::F32 => {
            #[allow(clippy::cast_possible_truncation)]
            let n = w as f32;
            (w.is_nan() || f64::from(n) == w).then(|| bits32(n))
        }
    }
}

/// `x`, of type `from`, as integer `n`, if it's one: finite, with no
/// fraction. Whether that fits in the type converted to is the caller's.
pub(super) fn to_int(x: u64, from: FloatTy) -> Option<i128> {
    let w = wide(x, from);
    // Every finite `f64` past 2^127 is out of every integer type's range.
    if !w.is_finite() || w.trunc() != w || w.abs() >= 2f64.powi(127) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    Some(w as i128)
}

/// Integer `n` as a `to`, if it's one exactly.
pub(super) fn of_int(n: i128, to: FloatTy) -> Option<u64> {
    let x = nearest(n, to);
    (to_int(x, to) == Some(n)).then_some(x)
}

/// The `to` nearest integer `n`, ties to even: `Float::nearest`.
#[allow(clippy::cast_precision_loss)]
pub(super) fn nearest(n: i128, to: FloatTy) -> u64 {
    match to {
        FloatTy::F32 => bits32(n as f32),
        FloatTy::F64 => (n as f64).to_bits(),
    }
}

/// The square root of `x`, of type `t`, rounded once, as IEEE 754 says
/// (and Rust's `sqrt` is).
pub(super) fn sqrt(x: u64, t: FloatTy) -> u64 {
    match t {
        FloatTy::F32 => bits32(f32_of(x).sqrt()),
        FloatTy::F64 => f64_of(x).sqrt().to_bits(),
    }
}

/// The `to` nearest `f64` `x`, ties to even: itself, as an `f64`.
#[allow(clippy::cast_possible_truncation)]
pub(super) fn nearest_f64(x: u64, to: FloatTy) -> u64 {
    match to {
        FloatTy::F32 => bits32(f64_of(x) as f32),
        FloatTy::F64 => x,
    }
}

/// How the interpreter shows `x`, of type `t`, in a message: as Rust's
/// `{:?}`, which core's `Display` writes too.
pub(super) fn show(x: u64, t: FloatTy) -> String {
    match t {
        FloatTy::F32 => format!("{:?}", f32_of(x)),
        FloatTy::F64 => format!("{:?}", f64_of(x)),
    }
}

/// Why a float of type `from` can't be converted to `to`, `"an `i32`"`
/// or the like: what the C backend says too, which can't write a float's
/// digits.
pub(super) fn not_as(from: FloatTy, to: &str) -> String {
    format!("`as` failed: the `{}` isn't {to}", from.name())
}
