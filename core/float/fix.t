//! Numbers of many bits, in fixed point: what the math functions' slow
//! paths work out values in, to hundreds of bits, when their fast paths
//! can't tell which float is nearest (see docs/102-math.md).
//!
//! A `Fix` of `n` limbs is an integer of `32n` bits in two's complement,
//! `d[0]` its lowest limb, read as that over `2^F`, `F = 32(n - 1)`: its
//! top limb is the integer part, from `-2^31` to below `2^31`, and the
//! rest are `F` bits of fraction. Each operation truncates what's past its
//! last bit, so is off by less than one *unit*, `2^-F`, of the exact
//! result, which the functions add up into a bound of their own.

use crate::num::{wrap, wrapping_add, wrapping_mul, wrapping_sub};
use crate::option::Option;
use super::dec::Format;

/// The precisions the slow paths work at, in limbs: 288 bits of fraction,
/// then 1056 (lemma M3 of docs/102-math.md).
pub const LOW: usize = 10;
pub const HIGH: usize = 34;

/// A number, as the module's doc says, of `n` limbs, all 0 past those:
/// the slow paths use up to 34, and what they work out first, of `pow`
/// and `tan`, up to two more, and `reduce` two more than that.
#[derive(Copy, Discard)]
pub struct Fix { d: [u32; 40], n: { n: usize | n >= 2 && n <= 40 } }

/// `2^k` as an `f64`, exactly, of a `k` from -1022 to 1023; else 0 below
/// and 2^1023 above.
fn power_of_two_i(k: i64) -> f64 with {} {
    if k < -1022 {
        return 0.0;
    }
    let e = if k > 1023 { 1023 } else { k };
    f64::from_bits((e + 1023) as u64 << 52)
}

/// `2^k`, of a `k` below 32.
fn pow2(k: { k: u32 | k < 32 }) -> u32 with {} {
    let mut p: u32 = 1;
    for _ in 0..k {
        p = wrapping_mul(p, 2);
    }
    p
}

impl Fix {
    /// 0, of `n` limbs.
    pub fn zero(n: { n: usize | n >= 2 && n <= 40 }) -> Fix with {} { Fix { d: [0; 40], n } }

    /// The integer `v`.
    pub fn int(n: { n: usize | n >= 2 && n <= 40 }, v: i32) -> Fix with {} {
        let mut x = Fix::zero(n);
        x.d[n - 1] = wrap(v);
        x
    }

    /// Is it below 0? Its top bit says.
    pub fn is_negative(&self) -> bool with {} { self.d[self.n - 1] >= 0x8000_0000 }

    /// Is it 0?
    pub fn is_zero(&self) -> bool with {} {
        for i in 0..self.n {
            if self.d[i] != 0 {
                return false;
            }
        }
        true
    }

    /// `-x`: every bit flipped, plus one unit. Exact, but of `-2^31`.
    pub fn neg(&self) -> Fix with {} {
        let mut r = *self;
        let mut carry: u32 = 1;
        for i in 0..self.n {
            let t = !self.d[i] as u64 + carry as u64;
            r.d[i] = wrap(t);
            carry = (t >> 32) as u32;
        }
        r
    }

    /// Its magnitude.
    pub fn abs(&self) -> Fix with {} {
        if self.is_negative() { self.neg() } else { *self }
    }

    /// `x + b`, exact, of two of the same limbs.
    pub fn add(&self, b: &Fix) -> Fix with {} {
        let mut r = *self;
        let mut carry: u32 = 0;
        for i in 0..self.n {
            let t = self.d[i] as u64 + b.d[i] as u64 + carry as u64;
            r.d[i] = wrap(t);
            carry = (t >> 32) as u32;
        }
        r
    }

    /// `x - b`, exact.
    pub fn sub(&self, b: &Fix) -> Fix with {} { self.add(&b.neg()) }

    /// `x + e` units, of an `e` at least 0: what bounds an error.
    pub fn add_units(&self, e: u32) -> Fix with {} {
        let mut r = *self;
        let mut carry = e;
        for i in 0..self.n {
            let t = self.d[i] as u64 + carry as u64;
            r.d[i] = wrap(t);
            carry = (t >> 32) as u32;
        }
        r
    }

    /// `x - e` units.
    pub fn sub_units(&self, e: u32) -> Fix with {} {
        let mut u = Fix::zero(self.n);
        u.d[0] = e;
        self.sub(&u)
    }

    /// `x · m`, exact while it fits.
    pub fn mul_small(&self, m: u32) -> Fix with {} {
        let mut r = *self;
        let mut carry: u32 = 0;
        for i in 0..self.n {
            let t = self.d[i] as u64 * m as u64 + carry as u64;
            r.d[i] = wrap(t);
            carry = (t >> 32) as u32;
        }
        r
    }

    /// `x / m`, its magnitude rounded down: less than a unit off.
    pub fn div_small(&self, m: { m: u32 | m > 0 }) -> Fix with {} {
        let a = self.abs();
        let mut r = a;
        let mut rem: u32 = 0;
        for i in (0..self.n).rev() {
            let cur = (rem as u64 << 32) | a.d[i] as u64;
            r.d[i] = wrap(cur / m as u64);
            rem = wrap(cur % m as u64);
        }
        if self.is_negative() { r.neg() } else { r }
    }

    /// `x · b`, its magnitude rounded down: less than a unit off. The
    /// product of the magnitudes has `2F` bits of fraction, of which the
    /// lowest `F`, `n - 1` limbs, are dropped.
    pub fn mul(&self, b: &Fix) -> Fix with {} {
        let x = self.abs();
        let y = b.abs();
        let n = self.n;
        let mut p: [u32; 80] = [0; 80];
        for i in 0..n {
            let mut carry: u32 = 0;
            let xi = x.d[i];
            if xi != 0 {
                for j in 0..n {
                    let t = p[i + j] as u64 + xi as u64 * y.d[j] as u64 + carry as u64;
                    p[i + j] = wrap(t);
                    carry = (t >> 32) as u32;
                }
            }
            p[i + n] = carry;
        }
        let mut r = Fix::zero(n);
        for i in 0..n {
            r.d[i] = p[i + n - 1];
        }
        if self.is_negative() != b.is_negative() { r.neg() } else { r }
    }

    /// `x · 2^k`, of a `k` below 32 times its limbs: what's moved past its
    /// top is dropped.
    pub fn shl(&self, k: u32) -> Fix with {} {
        let limbs = (k / 32) as usize;
        let bits = k % 32;
        let mut r = Fix::zero(self.n);
        for i in 0..self.n {
            if i >= limbs {
                let hi = self.d[i - limbs] as u64;
                let lo = if i >= limbs + 1 { self.d[i - limbs - 1] as u64 } else { 0 };
                let both = (hi << 32) | lo;
                r.d[i] = wrap(both >> (32 - bits));
            }
        }
        r
    }

    /// `x · 2^-k`, rounded down: arithmetic, the sign kept.
    pub fn shr(&self, k: u32) -> Fix with {} {
        let fill: u32 = if self.is_negative() { 0xFFFF_FFFF } else { 0 };
        let limbs = (k / 32) as usize;
        let bits = k % 32;
        let mut r = *self;
        for i in 0..self.n {
            let lo = if i + limbs < self.n { self.d[i + limbs] } else { fill };
            let hi = if i + limbs + 1 < self.n { self.d[i + limbs + 1] } else { fill };
            let both = (hi as u64 << 32) | lo as u64;
            r.d[i] = wrap(both >> bits);
        }
        r
    }

    /// The same number with `n` limbs: its lowest dropped (rounded down),
    /// or 0 limbs added below.
    pub fn with_limbs(&self, n: { n: usize | n >= 2 && n <= 40 }) -> Fix with {} {
        let mut r = Fix::zero(n);
        for i in 0..n {
            // Limb `i` of `r` is limb `i + self.n - n` of `self`.
            if i + self.n >= n {
                let j = i + self.n - n;
                if j < self.n {
                    r.d[i] = self.d[j];
                }
            }
        }
        r
    }

    /// The value of `x`, exactly where it has no bits below `2^-F`, else
    /// its magnitude rounded down; of an `x` less than `2^31` either way,
    /// else 0. `x` is `m · 2^e`: its significand `m`, of 53 bits, put at
    /// bit `e + F` of the integer.
    pub fn of(n: { n: usize | n >= 2 && n <= 40 }, x: f64) -> Fix with {} {
        let b = x.to_bits();
        let exp = ((b >> 52) & 0x7FF) as i64;
        let frac = b & 0x000F_FFFF_FFFF_FFFF;
        let (m, e) = if exp == 0 {
            (frac, -1074i64)
        } else {
            (frac | 0x0010_0000_0000_0000, exp - 1075)
        };
        let mut r = Fix::zero(n);
        let at = e + 32 * (n as i64 - 1);
        if exp >= 1023 + 31 || m == 0 {
            return r;
        }
        if at < 0 {
            let s = -at;
            if s < 64 {
                let v = m >> s as u32;
                r.d[0] = wrap(v);
                r.d[1] = wrap(v >> 32);
            }
        } else {
            let limb = (at / 32) as usize;
            let bit = (at % 32) as u32;
            let lo = (m & 0xFFFF_FFFF) * pow2(bit) as u64;
            let hi = (m >> 32) * pow2(bit) as u64;
            // `m · 2^bit` is `lo + hi · 2^32`: three limbs, from `limb` up.
            let w0: u32 = wrap(lo);
            let mid = (lo >> 32) + (hi & 0xFFFF_FFFF);
            let w1: u32 = wrap(mid);
            let w2: u32 = wrap((hi >> 32) + (mid >> 32));
            if limb < n {
                r.d[limb] = w0;
            }
            if limb + 1 < n {
                r.d[limb + 1] = w1;
            }
            if limb + 2 < n {
                r.d[limb + 2] = w2;
            }
        }
        if b >= 0x8000_0000_0000_0000 { r.neg() } else { r }
    }

    /// About its value, as an `f64`, to 2^-60 of it: its three limbs from
    /// its highest that isn't 0, each exactly an `f64`, scaled by a power
    /// of two exactly, and added up.
    pub fn to_f64(&self) -> f64 with {} {
        let a = self.abs();
        let n = self.n;
        let found = a.top();
        let t0 = if found < 0 { 0 } else { (found / 32) as usize };
        let top = if t0 >= n { n - 1 } else { t0 };
        // Limb `i` is worth `2^(32 (i - (n - 1)))`.
        let at = 32 * (top as i64 - (n as i64 - 1));
        let mut v = a.d[top] as f64 * power_of_two_i(at);
        if top >= 1 {
            v = v + a.d[top - 1] as f64 * power_of_two_i(at - 32);
        }
        if top >= 2 {
            v = v + a.d[top - 2] as f64 * power_of_two_i(at - 64);
        }
        if self.is_negative() { -v } else { v }
    }

    /// `2^-k`, of a `k` from 0 to its bits of fraction, else 0.
    pub fn unit_at(
        n: { n: usize | n >= 2 && n <= 40 },
        k: { k: i64 | k >= -5000 && k <= 5000 },
    ) -> Fix with {} {
        let mut r = Fix::zero(n);
        let at = 32 * (n as i64 - 1) - k;
        if at >= 0 && at < 32 * n as i64 {
            let i = (at / 32) as usize;
            if i < n {
                r.d[i] = pow2((at % 32) as u32);
            }
        }
        r
    }

    /// Bit `i` of the integer it is, 0 past its end.
    fn bit(&self, i: i64) -> u64 with {} {
        if i < 0 || i >= 32 * self.n as i64 {
            return 0;
        }
        let limb = (i / 32) as usize;
        if limb < self.n { ((self.d[limb] >> (i % 32) as u32) & 1) as u64 } else { 0 }
    }

    /// Where its highest 1 is, of a number above 0; -1 of 0.
    fn top(&self) -> { t: i64 | t >= -1 && t <= 1279 } with {} {
        for k in 0..self.n {
            let i = self.n - 1 - k;
            let x = self.d[i];
            if x != 0 {
                for j in 0..32u32 {
                    if x >> (31 - j) != 0 {
                        return 32 * i as i64 + 31 - j as i64;
                    }
                }
            }
        }
        -1
    }

    /// Are any of its bits below bit `s` 1?
    fn any_below(&self, s: i64) -> bool with {} {
        for i in 0..self.n {
            let lowest = 32 * i as i64;
            if lowest >= s {
                return false;
            }
            let x = self.d[i];
            let mask: u32 = if s - lowest >= 32 {
                0xFFFF_FFFF
            } else {
                wrapping_sub(pow2((s - lowest) as u32), 1)
            };
            if x & mask != 0 {
                return true;
            }
        }
        false
    }
}

/// Bits `s` to `s + count` of `x`'s integer, of a `count` of at most 60.
fn bits(x: &Fix, s: { s: i64 | s >= -20000 && s <= 20000 }, count: { c: u32 | c <= 60 }) -> u64
with {}
{
    let mut q: u64 = 0;
    for j in 0..count {
        q = wrapping_mul(q, 2) | x.bit(s + count as i64 - 1 - j as i64);
    }
    q
}

/// The bits of the float of format `f` nearest `x · 2^k`, of an `x` at
/// least 0, ties to even: an infinity past the greatest. `None` where `x`
/// hasn't the bits to say, which a slow path never makes.
fn round(x: &Fix, k: { k: i64 | k >= -5000 && k <= 5000 }, f: Format) -> Option<u64> with {} {
    let top = x.top();
    if top < 0 {
        return Option::Some(0);
    }
    let frac = 32 * (x.n as i64 - 1);
    let p = f.p as i64;
    // `x · 2^k` is in `[2^e, 2^(e + 1))`; its last bit, at `f`'s
    // precision, is at `g`, but no lower than a subnormal's.
    let e = top - frac + k;
    let least = f.least as i64;
    let most = f.most as i64;
    let g0 = e - p + 1;
    let g = if g0 < least { least } else { g0 };
    let infinity = wrapping_add(wrapping_mul((most - least) as u64, f.low), f.high);
    if g > most {
        return Option::Some(infinity);
    }
    // In `x`'s integer, the bits of the float's significand start at `s`.
    let s = g - k + frac;
    if s < 1 {
        return Option::None;
    }
    let q = bits(x, s - 1, f.p + 1);
    let half = q & 1;
    let mut m = q >> 1;
    if half == 1 && (m & 1 == 1 || x.any_below(s - 1)) {
        m = m + 1;
    }
    // `m · 2^g`, `m` at most `2^p`: of a normal value, the bits are its
    // exponent field, `g - least + 1`, then `m` without its leading 1,
    // which is `(g - least) · 2^(p - 1) + m`; of a subnormal, `m`; and a
    // carry into the next exponent comes out right too.
    if g > most - 1 && m >= f.high {
        return Option::Some(infinity);
    }
    // At most `2^64`'s tenth: wrapping only as the logic can't multiply
    // numbers that big.
    let bits = wrapping_add(wrapping_mul((g - least) as u64, f.low), m);
    Option::Some(if bits > infinity { infinity } else { bits })
}

/// The bits of the float of format `f` nearest `±(x · 2^k)`, negative if
/// `negative`, where `x` is the exact value but for at most `e` units
/// either way: if every value that close rounds to the same float, that;
/// else `None`, as it's too close to halfway between two to tell (lemma
/// M3). Rounding to nearest is monotone, so the ends of the interval
/// rounding the same means everything between does.
pub fn decide(
    x: &Fix,
    e: u32,
    k: { k: i64 | k >= -5000 && k <= 5000 },
    negative: bool,
    f: Format,
) -> Option<u64> with {} {
    let lo = x.sub_units(e);
    let hi = x.add_units(e);
    if lo.is_negative() {
        return Option::None;
    }
    match (round(&lo, k, f), round(&hi, k, f)) {
        (Option::Some(a), Option::Some(b)) => {
            if a == b {
                let sign = if f.p == 53 { 0x8000_0000_0000_0000 } else { 0x8000_0000 };
                Option::Some(if negative { a | sign } else { a })
            } else {
                Option::None
            }
        },
        _ => Option::None,
    }
}

/// `x` rounded as `decide` would if it could, of the value at the
/// highest precision: what a slow path gives where even that can't tell,
/// which no known input of any function needs (lemma M3).
pub fn nearest(x: &Fix, k: { k: i64 | k >= -5000 && k <= 5000 }, negative: bool, f: Format) -> u64
with {}
{
    let a = if x.is_negative() {
        0
    } else {
        match round(x, k, f) {
            Option::Some(a) => a,
            Option::None => 0,
        }
    };
    let sign = if f.p == 53 { 0x8000_0000_0000_0000 } else { 0x8000_0000 };
    if negative { a | sign } else { a }
}

/// ln 2, to less than 2 units: `2·atanh(1/3)`, the sum of `2 / ((2i + 1) ·
/// 3^(2i + 1))`, worked out with a limb more than asked for. Each of its
/// terms, fewer than `F / 3`, is off by less than 2 units of those, and
/// the terms left out add up to less than one: so less than a unit, all
/// told, of `n` limbs, and one more for dropping the extra limb.
pub fn ln2(n: { n: usize | n >= 2 && n <= 36 }) -> Fix with {} {
    let wide = n + 1;
    let mut sum = Fix::zero(wide);
    let mut p = Fix::int(wide, 2).div_small(3);
    for i in 0..1000u32 {
        if p.is_zero() {
            break;
        }
        sum = sum.add(&p.div_small(2 * i + 1));
        p = p.div_small(9);
    }
    sum.with_limbs(n)
}

/// `e^r`, of an `|r|` at most 1/2, to less than `4t` units, where `t` is
/// how many terms it took: the sum of `r^i / i!` until they're 0. The
/// term `i` is the one before it times `r`, then over `i`, each off by
/// less than a unit, and so are those before it, times `|r| / i`, at most
/// a half: so less than 2 units off itself, and each sum one more. What's
/// left out once a term is 0 is less than a unit, as `|r| ≤ 1/2`.
fn exp_small(r: &Fix) -> (Fix, { e: u32 | e <= 1600 }) with {} {
    let mut sum = Fix::int(r.n, 1).add(r);
    let mut term = *r;
    let mut t: { t: u32 | t <= 400 } = 2;
    for i in 2..400u32 {
        term = term.mul(r).div_small(i);
        if term.is_zero() {
            break;
        }
        sum = sum.add(&term);
        t = i + 1;
    }
    (sum, 4 * t)
}

/// The float nearest `e^x`, of format `f`, of an `|x|` below 1000: `x` is
/// `k · ln 2 + r`, `k` the integer nearest `x / ln 2`, so `|r|` is at most
/// a little over `ln 2 / 2`, and `e^x = 2^k · e^r`. `k · ln 2` is off by
/// `2|k|` units, so `r` is, and so `e^r` by that times `e^r < 1.5`, plus
/// what `exp_small` says, all of it times `e^r` again where it's a
/// relative error: less than `2^14` units, of `e^r > 0.7`. At `LOW` limbs,
/// then `HIGH`.
pub fn exp(x: f64, f: Format) -> u64 with {} {
    let k = clamp(nearest_int(x * 1.4426950408889634), 2000);
    let mut v = Fix::zero(LOW);
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let kl = ln2(n).mul_small(abs32(k));
        let r = if k < 0 { Fix::of(n, x).add(&kl) } else { Fix::of(n, x).sub(&kl) };
        let (e, err) = exp_small(&r);
        match decide(&e, err + 4 * abs32(k) + 16384, k as i64, false, f) {
            Option::Some(bits) => return bits,
            Option::None => v = e,
        }
    }
    nearest(&v, k as i64, false, f)
}

/// `k`, but no further from 0 than `most`: the integer nearest some
/// float, which the caller knows is closer.
pub fn clamp(k: i32, most: { m: i32 | m >= 0 }) -> { v: i32 | v >= -most && v <= most } with {} {
    if k < -most { -most } else if k > most { most } else { k }
}

/// `|k|`.
fn abs32(k: i32) -> { v: u32 | v == k || v == -k } with {} {
    if k < 0 { -(k as i64) as u32 } else { k as u32 }
}

/// The integer nearest `x`, ties to even, of an `|x|` below 2^31, else
/// some other: `x` plus `1.5 · 2^52` rounds it to an integer, in the last
/// bits of the sum's significand (the add-and-subtract trick).
pub fn nearest_int(x: f64) -> i32 with {} {
    let t = x + 6755399441055744.0;
    wrap(t.to_bits())
}

/// As `decide`, of a value `v` of either sign.
fn decide_signed(v: &Fix, e: u32, k: { k: i64 | k >= -5000 && k <= 5000 }, f: Format) -> Option<u64>
with {}
{
    if v.is_negative() { decide(&v.neg(), e, k, true, f) } else { decide(v, e, k, false, f) }
}

/// As `nearest`, of a value of either sign.
fn nearest_signed(v: &Fix, k: { k: i64 | k >= -5000 && k <= 5000 }, f: Format) -> u64 with {} {
    if v.is_negative() { nearest(&v.neg(), k, true, f) } else { nearest(v, k, false, f) }
}

/// `n / d`, of integers with `n` below `d`, as a `Fix` of `m` limbs: bit by
/// bit, long division, so exact but for the bits past its last, dropped.
fn ratio(
    n: u64,
    d: { d: u64 | d > 0 && d < 4611686018427387904 },
    m: { m: usize | m >= 2 && m <= 40 },
) -> Fix with {} {
    let mut rem: { r: u64 | r < d } = n % d;
    let mut r = Fix::zero(m);
    let frac = 32 * (m - 1);
    for b in 0..frac {
        let twice = rem * 2;
        let at = frac - 1 - b;
        if twice >= d {
            rem = twice - d;
            r.d[at / 32] = r.d[at / 32] | pow2((at % 32) as u32);
        } else {
            rem = twice;
        }
    }
    r
}

/// `x` as `m · 2^e`, `m` an integer of 53 bits, its top one set, and `e`,
/// of a positive finite `x`.
fn significand(x: f64) -> ({ m: u64 | m < 9007199254740992 }, { e: i64 | e >= -1200 && e <= 1000 })
with {}
{
    let b = x.to_bits();
    let exp = ((b >> 52) % 2048) as i64;
    let frac = b % 0x0010_0000_0000_0000;
    if exp != 0 {
        return (frac + 0x0010_0000_0000_0000, exp - 1075);
    }
    // A subnormal: its bits moved up until the top one is bit 52.
    let mut m: { m: u64 | m < 9007199254740992 } = frac;
    let mut e: { e: i64 | e >= -1200 && e <= -1074 } = -1074;
    for _ in 0..53u32 {
        if m >= 0x0010_0000_0000_0000 || m == 0 {
            break;
        }
        m = m % 0x0010_0000_0000_0000 * 2;
        e = if e > -1127 { e - 1 } else { e };
    }
    (m % 9007199254740992, e)
}

/// `ln x`, of a positive finite `x`, to less than `err` units: `x` is `2^e
/// · m`, `m` in `[1/sqrt 2, sqrt 2)`, and `ln m` is `2 atanh z`, `z = (m - 1) /
/// (m + 1)`, whose magnitude is below 0.172: the sum of `z^(2i+1) /
/// (2i + 1)`, each term off by at most 3 units, as `z^2 < 0.03` shrinks
/// what its powers were off by, and the sum by one more; then `e · ln 2`,
/// off by `2|e|`.
fn ln_fix(x: f64, n: { n: usize | n >= 2 && n <= 36 }) -> (Fix, { e: u32 | e <= 100000 }) with {} {
    let (m, e0) = significand(x);
    // `m / 2^52`, or `m / 2^53` where that's above sqrt 2.
    let high = m >= 6369051672525773;
    let d: { d: u64 | d > 0 && d <= 9007199254740992 } = if high {
        9007199254740992
    } else {
        4503599627370496
    };
    let e = if high { e0 + 53 } else { e0 + 52 };
    let z = if m >= d { ratio(m - d, m + d, n) } else { ratio(d - m, m + d, n).neg() };
    let z2 = z.mul(&z);
    let mut sum = z;
    let mut p = z;
    let mut terms: { t: u32 | t <= 2000 } = 1;
    for i in 1..2000u32 {
        p = p.mul(&z2);
        if p.is_zero() {
            break;
        }
        sum = sum.add(&p.div_small(2 * i + 1));
        terms = i + 1;
    }
    let ln_m = sum.mul_small(2);
    let ea = if e < 0 { -e as u32 } else { e as u32 };
    let el = ln2(n).mul_small(ea);
    let v = if e < 0 { ln_m.sub(&el) } else { ln_m.add(&el) };
    (v, 8 * terms + 2 * ea + 64)
}

/// The float nearest `ln x`, of format `f`, of a positive finite `x` not 1.
pub fn ln(x: f64, f: Format) -> u64 with {} {
    let mut v = Fix::zero(LOW);
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (l, err) = ln_fix(x, n);
        match decide_signed(&l, err, 0, f) {
            Option::Some(bits) => return bits,
            Option::None => v = l,
        }
    }
    nearest_signed(&v, 0, f)
}

/// `1 / d`, of a `d` from 1/4 to 4, to less than 8 units: Newton's
/// iteration `r (2 - d r)` from the `f64` nearest, which doubles the bits
/// it's right to each time, 53 to more than 1088 in five, the last off by
/// its own roundings only, 3 units.
fn recip(d: &Fix) -> Fix with {} {
    let n = d.n;
    let mut r = Fix::of(n, 1.0 / d.to_f64());
    let two = Fix::int(n, 2);
    for _ in 0..6u32 {
        r = r.mul(&two.sub(&d.mul(&r)));
    }
    r
}

/// The float nearest the logarithm of `x` to base `b`, 2 or 10, of format
/// `f`: `ln x / ln b`. `ln x` is below 745, so `1 / ln b`'s 8 units are
/// fewer than 6000 of the quotient, and `ln x`'s own times `1 / ln b`, at
/// most 1.45.
pub fn log(x: f64, b: u32, f: Format) -> u64 with {} {
    let mut v = Fix::zero(LOW);
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (l, err) = ln_fix(x, n);
        let lb = if b == 2 { ln2(n) } else { ln_fix(10.0, n).0 };
        let q = l.mul(&recip(&lb));
        match decide_signed(&q, 2 * err + 8192, 0, f) {
            Option::Some(bits) => return bits,
            Option::None => v = q,
        }
    }
    nearest_signed(&v, 0, f)
}

/// `a / b`, of finite floats with `|a| < |b|`, `b` not 0: `a` is `ma ·
/// 2^ea` and `b` is `mb · 2^eb`, so it's `ma / (2 mb)`, from 1/4 to 1, by
/// long division, times `2^(ea - eb + 1)`, a shift right.
fn quotient(a: f64, b: f64, n: { n: usize | n >= 2 && n <= 34 }) -> Fix with {} {
    if a == 0.0 {
        return Fix::zero(n);
    }
    let (ma, ea) = significand(if a < 0.0 { -a } else { a });
    let (mb, eb) = significand(if b < 0.0 { -b } else { b });
    let q = ratio(ma, if mb > 0 { 2 * mb } else { 1 }, n);
    let shift = eb - ea - 1;
    let r = if shift <= 0 { q } else if shift >= 2000 { Fix::zero(n) } else { q.shr(shift as u32) };
    if (a < 0.0) != (b < 0.0) { r.neg() } else { r }
}

/// `ln(1 + d)`, of a `|d|` at most 2^-50: the sum of `-(-d)^i / i`, each
/// term less than 2 units off, until they're 0.
fn ln1p_series(d: &Fix) -> (Fix, { e: u32 | e <= 2000 }) with {} {
    let mut sum = *d;
    let mut p = *d;
    let mut terms: { t: u32 | t <= 400 } = 1;
    for i in 2..400u32 {
        p = p.mul(d);
        if p.is_zero() {
            break;
        }
        let t = p.div_small(i);
        sum = if i % 2 == 0 { sum.sub(&t) } else { sum.add(&t) };
        terms = i;
    }
    (sum, 2 * terms + 2)
}

/// The float nearest `ln(1 + x)`, of format `f`, of an `x` above -1 of
/// magnitude at least 2^-53: `1 + x` is `s + se`, exactly, of floats, and
/// it's `ln s + ln(1 + se/s)`, the second below 2^-52.
pub fn ln_1p(x: f64, f: Format) -> u64 with {} {
    let s = 1.0 + x;
    let bb = s - 1.0;
    let se = 1.0 - (s - bb) + (x - bb);
    let mut v = Fix::zero(LOW);
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (l, err) = ln_fix(s, n);
        let (c, cerr) = ln1p_series(&quotient(se, s, n));
        let t = l.add(&c);
        match decide_signed(&t, err + cerr + 4, 0, f) {
            Option::Some(bits) => return bits,
            Option::None => v = t,
        }
    }
    nearest_signed(&v, 0, f)
}

/// The float nearest `2^x`, of format `f`, of an `x` not an integer, of
/// magnitude below 1100: `x = k + g`, `k` the nearest integer and `|g| ≤
/// 1/2` exact, and it's `2^k · e^(g ln 2)`, `g ln 2` less than 3 units off.
pub fn exp2(x: f64, f: Format) -> u64 with {} {
    let k = clamp(nearest_int(x), 2000);
    let g = x - k as f64;
    let mut v = Fix::zero(LOW);
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let r = Fix::of(n, g).mul(&ln2(n));
        let (e, err) = exp_small(&r);
        match decide(&e, err + 16, k as i64, false, f) {
            Option::Some(bits) => return bits,
            Option::None => v = e,
        }
    }
    nearest(&v, k as i64, false, f)
}

/// The float nearest `e^x - 1`, of format `f`, of an `x` from -38 to 710
/// of magnitude at least 2^-54: as `exp`, `e^x = 2^k · e^r`; then of a `k`
/// above 0, `(e^r - 2^-k) · 2^k`, and else `e^r · 2^k - 1`, its shift
/// right one more unit off.
pub fn exp_m1(x: f64, f: Format) -> u64 with {} {
    let k = clamp(nearest_int(x * 1.4426950408889634), 2000);
    let mut v = Fix::zero(LOW);
    let mut scale: { s: i64 | s >= -5000 && s <= 5000 } = 0;
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let kl = ln2(n).mul_small(abs32(k));
        let r = if k < 0 { Fix::of(n, x).add(&kl) } else { Fix::of(n, x).sub(&kl) };
        let (e, err) = exp_small(&r);
        let (t, s) = if k > 0 {
            (e.sub(&Fix::unit_at(n, k as i64)), k as i64)
        } else {
            (e.shr(abs32(k)).sub(&Fix::int(n, 1)), 0)
        };
        match decide_signed(&t, err + 4 * abs32(k) + 16386, s, f) {
            Option::Some(bits) => return bits,
            Option::None => {
                v = t;
                scale = s;
            },
        }
    }
    nearest_signed(&v, scale, f)
}

/// `e^v`, of a `Fix` `v` below 750 either way, `err` units off, as `m · 2^k`
/// and how many units `m` is off: as `exp` of an `f64`.
fn exp_of(
    v: &Fix,
    err: { e: u32 | e <= 200000 },
) -> (Fix, { k: i32 | k >= -2000 && k <= 2000 }, { e: u32 | e <= 1000000 }) with {} {
    let n = v.n;
    let k = clamp(nearest_int(v.to_f64() * 1.4426950408889634), 1100);
    let ka = abs32(k);
    let kl = if n <= 36 { ln2(n).mul_small(ka) } else { Fix::zero(n) };
    let r = if k < 0 { v.add(&kl) } else { v.sub(&kl) };
    let (e, terms) = exp_small(&r);
    (e, k, 2 * (err + 2 * ka + 2) + terms)
}

/// `x · m`, of a `0 ≤ m < 2^53`: `m` in two halves, the high one's
/// product moved up a limb. Exact while it fits.
fn mul_u53(x: &Fix, m: u64) -> Fix with {} {
    let lo = x.mul_small(wrap(m));
    let hi = x.mul_small(wrap(m >> 32));
    let mut up = Fix::zero(x.n);
    for i in 1..x.n {
        up.d[i] = hi.d[i - 1];
    }
    lo.add(&up)
}

/// The float nearest `x^y`, of format `f`, negative if `negative`, of a
/// positive finite `x` not 1 and a finite `y` whose product with `ln x` is
/// below 750 either way, and which `pow_exact` couldn't do: `e^(y ln x)`.
/// `ln x` is worked out with two limbs more, so that times `|y|`, below
/// 2^63, its error is still less than its own count of units; `y = my ·
/// 2^ey`, and `ln x · 2^ey`, a shift, is then times `my`.
pub fn pow(x: f64, y: f64, negative: bool, f: Format) -> u64 with {} {
    let (my, ey) = significand(if y < 0.0 { -y } else { y });
    let mut v = Fix::zero(LOW);
    let mut scale: { s: i32 | s >= -2000 && s <= 2000 } = 0;
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (l, err) = ln_fix(x, n + 2);
        let shifted = if ey >= 0 {
            mul_u53(&l, if ey < 11 { pow2(ey as u32) as u64 } else { 1 })
        } else if ey > -2000 {
            l.shr(-ey as u32)
        } else {
            Fix::zero(n + 2)
        };
        let p0 = mul_u53(&shifted, my).with_limbs(n);
        let p = if y < 0.0 { p0.neg() } else { p0 };
        let (e, k, eerr) = exp_of(&p, err + 4);
        match decide(&e, eerr, k as i64, negative, f) {
            Option::Some(bits) => return bits,
            Option::None => {
                v = e;
                scale = k;
            },
        }
    }
    nearest(&v, scale as i64, negative, f)
}

/// The integer square root of `x`, rounded down, of an `x` below 2^53: by
/// bits, from the top.
fn isqrt(x: u64) -> { r: u64 | r < 4294967296 } with {} {
    let mut r: { r: u64 | r < 4294967296 } = 0;
    for i in 0..27u32 {
        let b = pow2(26 - i) as u64;
        let t = r + b;
        if t < 4294967296 && t * t <= x {
            r = t;
        }
    }
    r
}

/// `x^y` exactly, of format `f`, where it's a float or halfway between
/// two, which is then rounded to the even one; else `None`, where it's
/// neither, which `pow` then works out (lemma M4). `x` positive finite,
/// not 1, and `y` finite, not 0. `x = mx · 2^ex`, `mx` odd, `y = n / 2^g`,
/// `n` odd. Where `mx` is 1, `x^y = 2^(ex · y)`: a power of two if `ex ·
/// y` is an integer, else irrational. Else `y` must be positive, as `mx^y`
/// isn't a dyadic number otherwise, and `mx` must be some `r^(2^g)`, as
/// it isn't rational otherwise, and then `x^y = r^n · 2^(ex · y)`, which
/// is neither a float nor halfway unless `r^n` has at most `p + 1` bits,
/// so `n ≤ 34`, as `r ≥ 3`, and `2^g ≤ 32`, as `r^(2^g) < 2^53`.
pub fn pow_exact(x: f64, y: f64, negative: bool, f: Format) -> Option<u64> with {} {
    let (m0, e0) = significand(x);
    let (mx, ex) = odd(m0, e0);
    let exact = if mx == 1 { power_of_two_exp(ex, y) } else { odd_power(mx, ex, y, f) };
    match exact {
        Option::Some((r, e)) => {
            // `r · 2^e`, exact: `r · 2^-64` in a `Fix`, the two limbs below
            // its integer part.
            let mut v = Fix::zero(LOW);
            v.d[LOW - 3] = wrap(r);
            v.d[LOW - 2] = wrap(r >> 32);
            let k = if e < -4900 { -4900 } else if e > 4900 { 4900 } else { e };
            Option::Some(nearest(&v, k + 64, negative, f))
        },
        Option::None => Option::None,
    }
}

/// `m · 2^e` as `mx · 2^ex`, `mx` odd: `m`'s trailing zeros moved into the
/// exponent.
fn odd(
    m: { m: u64 | m < 9007199254740992 },
    e: { e: i64 | e >= -1200 && e <= 1000 },
) -> ({ m: u64 | m < 9007199254740992 }, { e: i64 | e >= -1200 && e <= 1100 }) with {} {
    let mut mx: { m: u64 | m < 9007199254740992 } = m;
    let mut ex: { e: i64 | e >= -1200 && e <= 1100 } = e;
    for _ in 0..53u32 {
        if mx % 2 == 1 || mx == 0 {
            break;
        }
        mx = mx / 2;
        ex = if ex < 1100 { ex + 1 } else { ex };
    }
    (mx, ex)
}

/// `(2^ex)^y = 2^(ex · y)`, as `(1, ex · y)`, if `ex · y` is an integer:
/// `y` an integer, or below 2^20 with `y · 1024` one, as `ex` has at most
/// 11 bits; else it's irrational, and `None`. Of `x = -1`, `ex = 0`, it's 1.
fn power_of_two_exp(ex: { e: i64 | e >= -1200 && e <= 1100 }, y: f64) -> Option<(u64, i64)>
with {}
{
    if ex == 0 {
        return Option::Some((1, 0));
    }
    let small = y > -1048576.0 && y < 1048576.0;
    if !small {
        // An integer, as every float that big is: so big a power is past
        // every float either way.
        return Option::Some((1, if (y > 0.0) == (ex > 0) { 4900 } else { -4900 }));
    }
    if y * 1024.0 != (y * 1024.0).trunc() {
        return Option::None;
    }
    let t = ex * clamp(nearest_int(y * 1024.0), 1073741824) as i64;
    if t % 1024 != 0 { Option::None } else { Option::Some((1, t / 1024)) }
}

/// `(mx · 2^ex)^y` of an odd `mx` above 1, as `(r^n, ex · y)`, where it's a
/// float or halfway between two (lemma M4): `y = n / 2^g` from 1/32 to 64,
/// `mx = r^(2^g)`, `ex · y` an integer and `r^n` at most `2^(p+1)`.
fn odd_power(
    mx: u64,
    ex: { e: i64 | e >= -1200 && e <= 1100 },
    y: f64,
    f: Format,
) -> Option<(u64, i64)> with {} {
    if !(y > 0.0 && y <= 64.0) || y * 32.0 != (y * 32.0).trunc() {
        return Option::None;
    }
    let (n, g) = dyadic(y);
    let root = match root_of(mx, g) {
        Option::Some(r) => r,
        Option::None => return Option::None,
    };
    let t = ex * n;
    let div = two_to_the(g);
    if t % div != 0 {
        return Option::None;
    }
    match power_within(root, n, 2 * f.high) {
        Option::Some(big) => Option::Some((big, t / div)),
        Option::None => Option::None,
    }
}

/// `y`, a multiple of 1/32 from 1/32 to 64, as `n / 2^g`, `n` odd or `g` 0.
fn dyadic(y: f64) -> ({ n: i64 | n >= 0 && n <= 2048 }, { g: u32 | g <= 5 }) with {} {
    let n0 = clamp(nearest_int(y * 32.0), 2048) as i64;
    let mut n: { n: i64 | n >= 0 && n <= 2048 } = if n0 < 0 { 0 } else { n0 };
    let mut g: { g: u32 | g <= 5 } = 5;
    for _ in 0..5u32 {
        if g == 0 || n % 2 == 1 || n == 0 {
            break;
        }
        n = n / 2;
        g = g - 1;
    }
    (n, g)
}

/// `2^g`, of a `g` at most 5.
fn two_to_the(g: { g: u32 | g <= 5 }) -> { d: i64 | d >= 1 && d <= 32 } with {} {
    if g == 0 {
        1
    } else if g == 1 {
        2
    } else if g == 2 {
        4
    } else if g == 3 {
        8
    } else if g == 4 {
        16
    } else {
        32
    }
}

/// The `r` whose `2^g`-th power `m` is, if there's one.
fn root_of(m: u64, g: { g: u32 | g <= 5 }) -> Option<u64> with {} {
    let mut root = m;
    for _ in 0..g {
        let s = isqrt(root);
        if s * s != root {
            return Option::None;
        }
        root = s;
    }
    Option::Some(root)
}

/// `r^n`, if it's at most `limit`.
fn power_within(r: u64, n: { n: i64 | n >= 0 && n <= 2048 }, limit: u64) -> Option<u64> with {} {
    let mut big: u64 = 1;
    for _ in 0..n {
        if r == 0 || big > limit / r {
            return Option::None;
        }
        big = big * r;
    }
    Option::Some(big)
}

/// `e^a` and `e^-a`, of an `a` from 2^-26 to 711, as `e^r · 2^k` and `e^-r ·
/// 2^-k`, each within `err` units: `e^-r` is `1 / e^r`, 8 units off, and
/// more by what `e^r` is, relatively.
fn exp_both(
    a: f64,
    n: { n: usize | n >= 2 && n <= 34 },
) -> (Fix, Fix, { k: i32 | k >= -2000 && k <= 2000 }, { e: u32 | e <= 4000000 }) with {} {
    let (e, k, err) = exp_of(&Fix::of(n, a), 2);
    (e, recip(&e), k, 2 * err + 16)
}

/// The float nearest `sinh a`, negative if `negative`, of an `a` from
/// 2^-26 to 711: `2^(k-1) (e^r - e^-r · 2^-2k)`.
pub fn sinh(a: f64, negative: bool, f: Format) -> u64 with {} {
    let mut v = Fix::zero(LOW);
    let mut scale: { s: i32 | s >= -2001 && s <= 2001 } = 0;
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (e, ei, k, err) = exp_both(a, n);
        let t = e.sub(&ei.shr(2 * abs32(k)));
        match decide(&t, 2 * err + 2, k as i64 - 1, negative, f) {
            Option::Some(bits) => return bits,
            Option::None => {
                v = t;
                scale = k - 1;
            },
        }
    }
    nearest(&v, scale as i64, negative, f)
}

/// The float nearest `cosh a`, of an `a` from 2^-26 to 711: `2^(k-1) (e^r
/// + e^-r · 2^-2k)`.
pub fn cosh(a: f64, f: Format) -> u64 with {} {
    let mut v = Fix::zero(LOW);
    let mut scale: { s: i32 | s >= -2001 && s <= 2001 } = 0;
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (e, ei, k, err) = exp_both(a, n);
        let t = e.add(&ei.shr(2 * abs32(k)));
        match decide(&t, 2 * err + 2, k as i64 - 1, false, f) {
            Option::Some(bits) => return bits,
            Option::None => {
                v = t;
                scale = k - 1;
            },
        }
    }
    nearest(&v, scale as i64, false, f)
}

/// The float nearest `tanh a`, negative if `negative`, of an `a` from
/// 2^-27 to 19.1: `(1 - t)/(1 + t)`, `t = e^-2a = e^-r · 2^-k`, at most 1.
pub fn tanh(a: f64, negative: bool, f: Format) -> u64 with {} {
    let mut v = Fix::zero(LOW);
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let (_, ei, k, err) = exp_both(2.0 * a, n);
        let t = ei.shr(abs32(k));
        let one = Fix::int(n, 1);
        let q = one.sub(&t).mul(&recip(&one.add(&t)));
        match decide(&q, 4 * err + 32, 0, negative, f) {
            Option::Some(bits) => return bits,
            Option::None => v = q,
        }
    }
    nearest(&v, 0, negative, f)
}

/// The bits of 2/π after its point, 32 to a word, the first word's top bit
/// the first (scripts/math/tables.c `bits 2/pi 72`): 2304 of them, more
/// than `reduce` reads of any `f64`, 971 + 1120 + 31.
const TWO_OVER_PI: [u32; 72] = [
    0xA2F9836E,
    0x4E441529,
    0xFC2757D1,
    0xF534DDC0,
    0xDB629599,
    0x3C439041,
    0xFE5163AB,
    0xDEBBC561,
    0xB7246E3A,
    0x424DD2E0,
    0x06492EEA,
    0x09D1921C,
    0xFE1DEB1C,
    0xB129A73E,
    0xE88235F5,
    0x2EBB4484,
    0xE99C7026,
    0xB45F7E41,
    0x3991D639,
    0x835339F4,
    0x9C845F8B,
    0xBDF9283B,
    0x1FF897FF,
    0xDE05980F,
    0xEF2F118B,
    0x5A0A6D1F,
    0x6D367ECF,
    0x27CB09B7,
    0x4F463F66,
    0x9E5FEA2D,
    0x7527BAC7,
    0xEBE5F17B,
    0x3D0739F7,
    0x8A5292EA,
    0x6BFB5FB1,
    0x1F8D5D08,
    0x56033046,
    0xFC7B6BAB,
    0xF0CFBC20,
    0x9AF4361D,
    0xA9E39161,
    0x5EE61B08,
    0x6599855F,
    0x14A06840,
    0x8DFFD880,
    0x4D732731,
    0x06061556,
    0xCA73A8C9,
    0x60E27BC0,
    0x8C6B47C4,
    0x19C367CD,
    0xDCE8092A,
    0x8359C476,
    0x8B961CA6,
    0xDDAF44D1,
    0x5719053E,
    0xA5FF0705,
    0x3F7E33E8,
    0x32C2DE4F,
    0x98327DBB,
    0xC33D26EF,
    0x6B1E5EF8,
    0x9F3A1F35,
    0xCAF27F1D,
    0x87F12190,
    0x7C7C246A,
    0xFA6ED577,
    0x2D30433B,
    0x15C614B5,
    0x9D19C3C2,
    0xC4AD414D,
    0x2C5D000C,
];

/// The bits of π/2 after its point (`bits pi/2 37`): it's 1 and these.
const HALF_PI: [u32; 37] = [
    0x921FB544,
    0x42D18469,
    0x898CC517,
    0x01B839A2,
    0x52049C11,
    0x14CF98E8,
    0x04177D4C,
    0x76273644,
    0xA29410F3,
    0x1C6809BB,
    0xDF2A3367,
    0x9A748636,
    0x605614DB,
    0xE4BE286E,
    0x9FC26ADA,
    0xDAA3848B,
    0xC90B6AEC,
    0xC4BCFD8D,
    0xE89885D3,
    0x4C6FDAD6,
    0x17FEB96D,
    0xE80D6FDB,
    0xDC70D7F6,
    0xB5133F4B,
    0x5D3E4822,
    0xF8963FCC,
    0x9250CCA3,
    0xD9C8B67B,
    0x8400F971,
    0x42C77E0B,
    0x31B4906C,
    0x38ABA734,
    0xD22C7F51,
    0xFA499EBF,
    0x06CABA47,
    0xB9475B2C,
    0x38C5E6AC,
];

/// π/2, of `n` limbs, rounded down: less than a unit off.
fn half_pi(n: { n: usize | n >= 2 && n <= 38 }) -> Fix with {} {
    let mut r = Fix::zero(n);
    r.d[n - 1] = 1;
    for i in 0..n - 1 {
        r.d[n - 2 - i] = HALF_PI[i];
    }
    r
}

/// The 32 bits of 2/π from bit `i` after its point, the first of them the
/// word's top bit: 0 past the table, and the bits before the point, all 0.
fn two_over_pi(i: { i: i64 | i >= -10000 && i <= 10000 }) -> u32 with {} {
    if i < 1 {
        // The window starts before the point: what's after it, moved down.
        let shift = 1 - i;
        return if shift >= 32 { 0 } else { wrap(two_over_pi(1) as u64 >> shift as u32) };
    }
    let w = ((i - 1) / 32) as usize;
    let o = ((i - 1) % 32) as u32;
    let hi = if w < 72 { TWO_OVER_PI[w] } else { 0 };
    let lo = if w + 1 < 72 { TWO_OVER_PI[w + 1] } else { 0 };
    wrap(((hi as u64 << 32) | lo as u64) >> (32 - o))
}

/// `x` less the nearest multiple of π/2, of a positive finite `x`: `r`, at
/// most π/4 either way, in `n` limbs, less than 4 units off, and which
/// multiple, mod 4 (Payne and Hanek's reduction). `x = M · 2^E`, so `x ·
/// 2/π` mod 4 is `M` times the bits of 2/π from the one worth `2^(E-i) =
/// 2` on, as the ones before give multiples of 4: in a `Fix` of two limbs
/// more, where its bit `i` is at position `E - i + F`, and what would be
/// past its top, worth `2^32` and more, is dropped, as the product wraps
/// there anyway. What's past its last bit, less than a unit, is less than
/// `2^53` units of the product, less than one of `n` limbs.
pub fn reduce(x: f64, n: { n: usize | n >= 2 && n <= 36 }) -> (Fix, { q: u32 | q < 4 }) with {} {
    let (m, e) = significand(x);
    let wide = n + 2;
    let frac = 32 * (wide as i64 - 1);
    let mut w = Fix::zero(wide);
    for j in 0..wide {
        w.d[j] = two_over_pi(e + frac - 32 * j as i64 - 31);
    }
    let mut p = mul_u53(&w, m);
    let mut q = p.d[wide - 1] % 4;
    p.d[wide - 1] = 0;
    if p.d[wide - 2] >= 0x8000_0000 {
        p = p.sub(&Fix::int(wide, 1));
        q = (q + 1) % 4;
    }
    (p.with_limbs(n).mul(&half_pi(n)), q)
}

/// `sin r` and `cos r`, of an `|r|` at most π/4 + a little, each to less than
/// `3t` units, `t` how many terms it took: their Taylor series, each term
/// the one before times `r^2 / ((2i)(2i ± 1))`, which shrinks what it was
/// off by, as `r^2 / 6 < 1/9`.
fn sincos(r: &Fix) -> (Fix, Fix, { e: u32 | e <= 4000 }) with {} {
    let n = r.n;
    let r2 = r.mul(r);
    let mut s = *r;
    let mut c = Fix::int(n, 1);
    let mut ts = *r;
    let mut tc = Fix::int(n, 1);
    let mut terms: { t: u32 | t <= 1000 } = 1;
    for j in 1..1000u32 {
        let i: { i: u32 | i >= 1 && i < 1000 } = if j >= 1 && j < 1000 { j } else { 1 };
        ts = ts.mul(&r2).div_small(2 * i * (2 * i + 1));
        tc = tc.mul(&r2).div_small((2 * i - 1) * (2 * i));
        if ts.is_zero() && tc.is_zero() {
            break;
        }
        if i % 2 == 1 {
            s = s.sub(&ts);
            c = c.sub(&tc);
        } else {
            s = s.add(&ts);
            c = c.add(&tc);
        }
        terms = i + 1;
    }
    (s, c, 3 * terms + 2)
}

/// Which of `sin x`, `cos x` and `tan x`.
#[derive(Copy, Discard)]
pub enum Trig { Sin, Cos, Tan }

/// `v · 2^s`, of an `|v|` above 2^-62, in `[1/2, 1)` either way, and `s`:
/// what `tan` divides by.
fn normalized(v: &Fix) -> (Fix, { s: u32 | s <= 64 }) with {} {
    let a = v.abs();
    let frac = 32 * (a.n as i64 - 1);
    let top = a.top();
    let s = if top < 0 {
        64
    } else if frac - 1 - top > 64 {
        64
    } else if frac - 1 - top < 0 {
        0
    } else {
        (frac - 1 - top) as u32
    };
    let r = a.shl(s);
    (if v.is_negative() { r.neg() } else { r }, s)
}

/// The float nearest `sin x`, `cos x` or `tan x`, of format `f`, of a
/// finite `x` of magnitude at least 2^-27: reduced (`reduce`), then by the
/// quadrant, `±sin r` or `±cos r`, or of `tan`, `sin r / cos r`, or `-cos r
/// / sin r`, which is worked out as `-1 / (tan r · 2^s) · 2^s`, `tan r ·
/// 2^s` in `[1/2, 1)`, two limbs more precise, so its error times `2^s` is
/// still a few units.
pub fn trig(x: f64, which: Trig, f: Format) -> u64 with {} {
    let negative = x < 0.0;
    let a = if negative { -x } else { x };
    let mut v = Fix::zero(LOW);
    let mut scale: { s: i64 | s >= -64 && s <= 64 } = 0;
    for step in 0..2u32 {
        let n = if step == 0 { LOW } else { HIGH };
        let wide = match which {
            Trig::Tan => n + 2,
            _ => n,
        };
        let (r, q) = reduce(a, wide);
        let (s, c, err) = sincos(&r);
        let (t, e, k) = match which {
            Trig::Sin => (if q % 2 == 0 { s } else { c }, err + 4, 0),
            Trig::Cos => (if q % 2 == 0 { c } else { s }, err + 4, 0),
            Trig::Tan => {
                let tr = s.mul(&recip(&c));
                if q % 2 == 0 {
                    (tr.with_limbs(n), 4 * err + 32, 0)
                } else {
                    let (u, sh) = normalized(&tr);
                    (recip(&u).neg().with_limbs(n), 4 * err + 32, sh as i64)
                }
            },
        };
        // The sign: sin and tan are odd, cos even; and each quadrant's.
        let flip = match which {
            Trig::Sin => (q >= 2) != negative,
            Trig::Cos => q == 1 || q == 2,
            Trig::Tan => negative,
        };
        let t2 = if flip { t.neg() } else { t };
        match decide_signed(&t2, e, k, f) {
            Option::Some(bits) => return bits,
            Option::None => {
                v = t2;
                scale = k;
            },
        }
    }
    nearest_signed(&v, scale, f)
}
