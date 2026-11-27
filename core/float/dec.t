//! Floats as decimal text, and decimal text as floats: worked out exactly,
//! on big natural numbers, so a float is written as the fewest digits that
//! read back as it, and text is read as the float nearest it (see
//! docs/101-floats.md).
//!
//! A `Big` has room for 4096 bits, and the greatest number made here has
//! fewer: about 1100 bits to write an `f64` the shortest way, 1900 to
//! write it with 255 digits after the point, and 3830 to read 800 digits
//! (lemma F6 of docs/101-floats.md). What would go past that is dropped
//! instead, which never happens, so that none of this can panic.

use crate::cmp::Ordering;
use crate::fmt::{Display, Write};
use crate::num::{saturating_add, saturating_sub, wrap, wrapping_mul};
use crate::option::Option;
use crate::result::Result;

/// A natural number: `d[0] + d[1]·2^32 + d[2]·2^64 + ..`, of `n` limbs at
/// most, all 0 past those.
#[derive(Copy, Discard)]
struct Big { d: [u32; 128], n: { n: usize | n <= 128 } }

impl Big {
    fn zero() -> Big { Big { d: [0; 128], n: 0 } }

    /// `x`.
    fn of(x: u64) -> Big {
        let mut b = Big::zero();
        let lo: u32 = wrap(x);
        let hi = (x >> 32) as u32;
        b.d[0] = lo;
        b.d[1] = hi;
        b.n = if hi != 0 { 2 } else if lo != 0 { 1 } else { 0 };
        b
    }

    fn is_zero(&self) -> bool { self.n == 0 }

    /// How many bits it has, up to its highest 1: 0 of 0.
    fn bits(&self) -> { b: usize | b <= 4096 } {
        if self.n == 0 {
            return 0;
        }
        (self.n - 1) * 32 + (width32(self.d[self.n - 1]) as usize)
    }

    /// Drops the 0 limbs at its top, so `n` counts up to its highest.
    fn trim(&mut self) {
        while self.n > 0 && self.d[self.n - 1] == 0 {
            self.n = self.n - 1;
        }
    }

    /// Its limb `i`, 0 past the end.
    fn limb(&self, i: usize) -> u32 { if i < 128 { self.d[i] } else { 0 } }

    /// It times `m`.
    fn mul_small(&mut self, m: u32) {
        let mut carry: u32 = 0;
        let n = self.n;
        for i in 0..n {
            let t = (self.d[i] as u64) * (m as u64) + (carry as u64);
            self.d[i] = wrap(t);
            carry = (t >> 32) as u32;
        }
        if carry != 0 && self.n < 128 {
            self.d[self.n] = carry;
            self.n = self.n + 1;
        }
        self.trim();
    }

    /// It times 10^k.
    fn mul_pow10(&mut self, k: usize) {
        for _ in 0..k / 9 {
            self.mul_small(1_000_000_000);
        }
        self.mul_small(pow10(k % 9));
    }

    /// It times 2^k.
    fn shl(&mut self, k: usize) {
        let limbs = k / 32;
        let bits = (k % 32) as u32;
        if bits > 0 {
            self.mul_small(pow2(bits));
        }
        if limbs == 0 || self.n == 0 {
            return;
        }
        if limbs >= 128 {
            *self = Big::zero();
            return;
        }
        for i in (0..self.n).rev() {
            if i + limbs < 128 {
                self.d[i + limbs] = self.d[i];
            }
            self.d[i] = 0;
        }
        self.n = if self.n + limbs < 128 { self.n + limbs } else { 128 };
        self.trim();
    }

    /// It divided by 2^k, rounding down.
    fn shr(&mut self, k: usize) {
        let limbs = k / 32;
        let bits = (k % 32) as u32;
        if limbs >= 128 {
            *self = Big::zero();
            return;
        }
        for i in 0..128usize {
            let lo = self.limb(i + limbs) as u64;
            let hi = self.limb(i + limbs + 1) as u64;
            let both = (hi << 32) | lo;
            self.d[i] = wrap(both >> bits);
        }
        self.n = if self.n > limbs { self.n - limbs } else { 0 };
        self.trim();
    }

    /// It plus `b`.
    fn add(&mut self, b: &Big) {
        let n = if self.n > b.n { self.n } else { b.n };
        let mut carry: u32 = 0;
        for i in 0..n {
            let t = (self.d[i] as u64) + (b.d[i] as u64) + (carry as u64);
            self.d[i] = wrap(t);
            carry = (t >> 32) as u32;
        }
        self.n = n;
        if carry != 0 && self.n < 128 {
            self.d[self.n] = carry;
            self.n = self.n + 1;
        }
    }

    /// It minus `b`, which is at most it.
    fn sub(&mut self, b: &Big) {
        let mut borrow = false;
        let n = self.n;
        for i in 0..n {
            let t = (self.d[i] as u64) + 0x1_0000_0000 - (b.d[i] as u64) - (if borrow { 1 } else { 0 });
            self.d[i] = wrap(t);
            borrow = t < 0x1_0000_0000;
        }
        self.trim();
    }

    /// It divided by `m`, rounding down, and what's left over.
    fn div_small(&mut self, m: { m: u32 | m > 0 }) -> u32 {
        let mut rem: u32 = 0;
        for i in (0..self.n).rev() {
            let cur = ((rem as u64) << 32) | (self.d[i] as u64);
            self.d[i] = wrap(cur / (m as u64));
            rem = wrap(cur % (m as u64));
        }
        self.trim();
        rem
    }

    /// How it compares with `b`.
    fn cmp(&self, b: &Big) -> Ordering {
        if self.n != b.n {
            return if self.n < b.n { Ordering::Less } else { Ordering::Greater };
        }
        for i in (0..self.n).rev() {
            if self.d[i] != b.d[i] {
                return if self.d[i] < b.d[i] { Ordering::Less } else { Ordering::Greater };
            }
        }
        Ordering::Equal
    }

    /// Is it less than `b`?
    fn lt(&self, b: &Big) -> bool {
        match self.cmp(b) {
            Ordering::Less => true,
            _ => false,
        }
    }

    /// Is it `b`?
    fn eq(&self, b: &Big) -> bool {
        match self.cmp(b) {
            Ordering::Equal => true,
            _ => false,
        }
    }

    /// Its lowest 64 bits.
    fn low(&self) -> u64 { (self.d[0] as u64) | ((self.d[1] as u64) << 32) }
}

/// 10^k, of a `k` less than 9.
fn pow10(k: usize) -> u32 {
    match k {
        0 => 1,
        1 => 10,
        2 => 100,
        3 => 1_000,
        4 => 10_000,
        5 => 100_000,
        6 => 1_000_000,
        7 => 10_000_000,
        _ => 100_000_000,
    }
}

/// 2^k, of a `k` less than 32: doubled `k` times, which can't wrap.
fn pow2(k: u32) -> u32 {
    let mut p: u32 = 1;
    for _ in 0..k {
        p = wrapping_mul(p, 2);
    }
    p
}

/// 2^k, of a `k` less than 64.
fn pow2_64(k: usize) -> u64 {
    let mut p: u64 = 1;
    for _ in 0..k {
        p = wrapping_mul(p, 2);
    }
    p
}

/// How many bits `x` has, up to its highest 1: 0 of 0.
fn width32(x: u32) -> { w: u32 | w <= 32 } {
    for i in 0..32u32 {
        if x >> i == 0 {
            return i;
        }
    }
    32
}

/// How many bits `x` has, up to its highest 1.
fn width64(x: u64) -> { w: u32 | w <= 64 } {
    let hi = (x >> 32) as u32;
    if hi != 0 { 32 + width32(hi) } else { width32(wrap(x)) }
}

/// `a + b`.
fn sum(a: &Big, b: &Big) -> Big {
    let mut s = *a;
    s.add(b);
    s
}

/// What a float is, to be written or read: a sign, and a finite value
/// `m · 2^e`, an infinity or a NaN.
#[derive(Copy, Discard)]
pub enum Parts {
    Finite(bool, u64, { e: i32 | e >= -1100 && e <= 1000 }),
    Infinite(bool),
    NaN,
}

/// The shape of one of the float types: how many bits of significand it
/// has, the leading one included, and the least exponent of its last bit,
/// a subnormal's.
#[derive(Copy, Discard)]
pub struct Format {
    p: { p: u32 | p >= 2 && p <= 53 },
    least: { l: i32 | l >= -1100 && l <= 0 },
    most: { m: i32 | m >= 0 && m <= 1000 },
    /// 2^(p-1), the least significand of a normal value.
    low: { v: u64 | v >= 2 && v <= 4503599627370496 },
    /// 2^p, past the greatest.
    high: { v: u64 | v >= 4 && v <= 9007199254740992 },
}

/// `f32`'s: 24 bits, the least subnormal 2^-149, the greatest finite
/// value's last bit 2^104.
pub fn f32_format() -> Format { Format { p: 24, least: -149, most: 104, low: 8388608, high: 16777216 } }

/// `f64`'s: 53 bits, the least subnormal 2^-1074, the greatest's last bit
/// 2^971.
pub fn f64_format() -> Format {
    Format { p: 53, least: -1074, most: 971, low: 4503599627370496, high: 9007199254740992 }
}

/// The decimal digits of a positive value, `0.d₁d₂..dₙ · 10^k`, `d₁` not 0.
#[derive(Copy, Discard)]
struct Digits { d: [u8; 24], n: { n: usize | n <= 24 }, k: { k: i32 | k >= -400 && k <= 400 } }

impl Digits {
    /// One more digit, `x`, at the end: 10 carries into the one before.
    fn push(&mut self, x: u32) {
        if x < 10 {
            if self.n < 24 {
                self.d[self.n] = x as u8;
                self.n = self.n + 1;
            }
            return;
        }
        // A 10: the digits before, plus one, then a 0, which is dropped
        // at the end of the digits, as it says nothing.
        for i in (0..self.n).rev() {
            let x = self.d[i];
            if x < 9 {
                self.d[i] = x + 1;
                self.n = i + 1;
                return;
            }
        }
        // All were 9: it's 1, a place higher.
        self.d[0] = 1;
        self.n = 1;
        self.k = if self.k < 400 { self.k + 1 } else { 400 };
    }
}

/// `x`, at least 0 and at most 5000, as a `usize`: a number of bits or
/// digits, which is never more.
fn index(x: i64) -> { u: usize | u <= 5000 } {
    if x <= 0 {
        0
    } else if x >= 5000 {
        5000
    } else {
        x as usize
    }
}

/// `-x`, at least 0 and at most 5000, as `index` says.
fn neg_index(x: i64) -> { u: usize | u <= 5000 } {
    if x >= 0 {
        0
    } else if x <= -5000 {
        5000
    } else {
        (-x) as usize
    }
}

/// The fewest digits that read back as `m · 2^e`, `m` positive, of type
/// `f`, and of those the nearest: Burger and Dybvig's free-format
/// algorithm, exactly. `r / s` is the value, and `plus / s` and `minus /
/// s` half the gaps to the floats above and below; the gap below is half
/// the one above where `m` is the least of its exponent, but not of the
/// least exponent. Where `m` is even, a decimal right at the middle of a
/// gap reads back as it (ties go to even), so may be written.
fn shortest(m: u64, e: { e: i32 | e >= -1100 && e <= 1000 }, f: Format) -> Digits {
    let even = m % 2 == 0;
    let narrow = m == f.low && e > f.least;
    let mut r = Big::of(m);
    let mut s = Big::of(1);
    let mut plus = Big::of(1);
    let mut minus = Big::of(1);
    if e >= 0 {
        let up = index(e as i64);
        if narrow {
            r.shl(up + 2);
            s = Big::of(4);
            plus.shl(up + 1);
            minus.shl(up);
        } else {
            r.shl(up + 1);
            s = Big::of(2);
            plus.shl(up);
            minus.shl(up);
        }
    } else {
        let down = index(-(e as i64));
        if narrow {
            r.shl(2);
            s.shl(down + 2);
            plus = Big::of(2);
        } else {
            r.shl(1);
            s.shl(down + 1);
        }
    }
    // `k` with `10^(k-1)` at most the top of the gap, from the bits: the
    // value's magnitude, `log₁₀ 2` a little under or over so it's never
    // too great; then up while it's too small.
    let top = e + (width64(m) as i32) - 1;
    let mut k = if top >= 0 { (top * 78913) >> 18 } else { (top * 78914) >> 18 } + 1;
    if k >= 0 {
        s.mul_pow10(index(k as i64));
    } else {
        let up = index(-(k as i64));
        r.mul_pow10(up);
        plus.mul_pow10(up);
        minus.mul_pow10(up);
    }
    loop {
        let high = sum(&r, &plus);
        let past = match high.cmp(&s) {
            Ordering::Greater => true,
            Ordering::Equal => even,
            Ordering::Less => false,
        };
        if !past {
            break;
        }
        s.mul_small(10);
        k = saturating_add(k, 1);
    }
    let k = if k < -400 { -400 } else if k > 400 { 400 } else { k };
    let mut out = Digits { d: [0; 24], n: 0, k };
    loop {
        r.mul_small(10);
        plus.mul_small(10);
        minus.mul_small(10);
        let mut digit: u32 = 0;
        while !r.lt(&s) && digit < 10 {
            r.sub(&s);
            digit = digit + 1;
        }
        // At most 9: `r` was less than `s` before it was times 10.
        let digit = if digit > 9 { 9 } else { digit };
        let low = match r.cmp(&minus) {
            Ordering::Less => true,
            Ordering::Equal => even,
            Ordering::Greater => false,
        };
        let high = match sum(&r, &plus).cmp(&s) {
            Ordering::Greater => true,
            Ordering::Equal => even,
            Ordering::Less => false,
        };
        if !low && !high && out.n < 24 {
            out.push(digit);
            continue;
        }
        let up = if low && high {
            let twice = sum(&r, &r);
            match twice.cmp(&s) {
                Ordering::Less => false,
                Ordering::Greater => true,
                Ordering::Equal => digit % 2 == 1,
            }
        } else {
            high
        };
        out.push(if up { digit + 1 } else { digit });
        return out;
    }
}


/// Writes `n` `c`s.
#[blame_caller]
fn repeat<W: Write>(c: char, n: usize, w: &mut W) -> Result<(), W::Error> with W::e {
    for _ in 0..n {
        w.write_char(c)?;
    }
    Ok(())
}

/// Digit `x`, 0 to 9, as a character.
fn digit(x: u8) -> char { if x < 10 { ((x as u32) + 0x30) as char } else { '?' } }

/// Writes `parts`, of type `f`, the shortest way that reads back as it:
/// `0.1`, `1.0`, `-0.0`, `1e16`, `1.5e-7`, `inf`, `NaN`. In decimal, with
/// at least one digit after the point, from `0.0001` up to but not
/// including `1e16`; else in scientific notation, its digits with a point
/// after the first, if there's more than one, then `e` and the exponent.
#[blame_caller]
pub fn write_shortest<W: Write>(parts: Parts, f: Format, w: &mut W) -> Result<(), W::Error> with W::e {
    let (negative, m, e) = match parts {
        Parts::NaN => return w.write_str("NaN"),
        Parts::Infinite(negative) => return w.write_str(if negative { "-inf" } else { "inf" }),
        Parts::Finite(negative, m, e) => (negative, m, e),
    };
    if negative {
        w.write_char('-')?;
    }
    if m == 0 {
        return w.write_str("0.0");
    }
    let ds = shortest(m, e, f);
    let sci = ds.k - 1;
    if sci < -4 || sci >= 16 {
        w.write_char(digit(ds.d[0]))?;
        if ds.n > 1 {
            w.write_char('.')?;
            for i in 1..ds.n {
                w.write_char(digit(ds.d[i]))?;
            }
        }
        w.write_char('e')?;
        return Display::fmt(&sci, w);
    }
    if sci < 0 {
        w.write_str("0.")?;
        repeat('0', index(-(sci as i64) - 1), w)?;
        for i in 0..ds.n {
            w.write_char(digit(ds.d[i]))?;
        }
        return Ok(());
    }
    let whole = index((sci as i64) + 1);
    for i in 0..ds.n {
        if i == whole {
            w.write_char('.')?;
        }
        w.write_char(digit(ds.d[i]))?;
    }
    if ds.n <= whole {
        repeat('0', whole - ds.n, w)?;
        w.write_str(".0")?;
    }
    Ok(())
}

/// The most digits after the point `{x:.n}` writes.
pub const MOST_PLACES: usize = 255;

/// Writes `parts` with `places` digits after the point, `places` at most
/// `MOST_PLACES`, of its exact value rounded there, ties to even: `{x:.2}`
/// of `0.125` is `0.12`, and of `2.675`, which is a little under that, is
/// `2.67`. No point if `places` is 0.
#[blame_caller]
pub fn write_fixed<W: Write>(parts: Parts, places: usize, w: &mut W) -> Result<(), W::Error> with W::e {
    let (negative, m, e) = match parts {
        Parts::NaN => return w.write_str("NaN"),
        Parts::Infinite(negative) => return w.write_str(if negative { "-inf" } else { "inf" }),
        Parts::Finite(negative, m, e) => (negative, m, e),
    };
    let places = if places > MOST_PLACES { MOST_PLACES } else { places };
    if negative {
        w.write_char('-')?;
    }
    // `m · 2^e · 10^places`, rounded to an integer, ties to even.
    let mut q = Big::of(m);
    q.mul_pow10(places);
    if e >= 0 {
        q.shl(index(e as i64));
    } else {
        let down = index(-(e as i64));
        let mut whole = q;
        whole.shr(down);
        let mut back = whole;
        back.shl(down);
        let mut rest = q;
        rest.sub(&back);
        rest.shl(1);
        let mut half = Big::of(1);
        half.shl(down);
        let up = match rest.cmp(&half) {
            Ordering::Greater => true,
            Ordering::Equal => whole.low() % 2 == 1,
            Ordering::Less => false,
        };
        q = whole;
        if up {
            q.add(&Big::of(1));
        }
    }
    // Its digits, last first, nine at a time.
    let mut digits: [u8; 600] = [0; 600];
    let mut n: usize = 0;
    while !q.is_zero() && n < 591 {
        let mut chunk = q.div_small(1_000_000_000);
        for _ in 0..9u32 {
            if n < 600 {
                digits[n] = (chunk % 10) as u8;
                n = n + 1;
            }
            chunk = chunk / 10;
        }
    }
    while n > 1 && n <= 600 && digits[n - 1] == 0 {
        n = n - 1;
    }
    // At least one digit before the point.
    let n = if n < places + 1 { places + 1 } else { n };
    let mut i = n;
    while i > 0 {
        i = i - 1;
        if i + 1 == places {
            w.write_char('.')?;
        }
        w.write_char(digit(if i < 600 { digits[i] } else { 0 }))?;
    }
    Ok(())
}

/// What's wrong with text that isn't a float.
#[derive(Copy, Debug, Discard, Eq, Send, Sync)]
export enum ParseFloatError {
    /// It has no characters.
    Empty,
    /// It isn't digits with a point, an exponent or both, or `inf`,
    /// `infinity` or `nan`, with a sign or not.
    Invalid,
}

impl Display for ParseFloatError {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str(match self {
            ParseFloatError::Empty => "cannot parse float from empty string",
            ParseFloatError::Invalid => "invalid float literal",
        })
    }
}

/// The most significant digits reading looks at: one past them, if any is
/// not 0, is kept as a 1, which says the value is a little more than the
/// digits, as it is, and so rounds as it does (lemma F6).
const MOST_DIGITS: usize = 800;

/// Is `b`, ASCII, `lower` in either case?
fn is(b: u8, lower: u8) -> bool { b == lower || (b < 0x80 && b + 0x20 == lower) }

/// Is `s` the word `word`, lowercase letters, in either case?
fn is_word(s: &[u8], word: &[u8]) -> bool {
    if s.len() != word.len() {
        return false;
    }
    for i in 0..s.len() {
        if i < word.len() && !is(s[i], word[i]) {
            return false;
        }
    }
    true
}

/// Text `s` as the float of type `f` nearest it, ties to even: digits,
/// with a `.` and more digits or not, or a `.` and digits, then an exponent
/// or not, `e` or `E` and digits, with a sign or not; or `inf`, `infinity`
/// or `nan`, in either case; with a `+` or `-` first, or not.
pub fn parse(s: &[u8], f: Format) -> Result<Parts, ParseFloatError> with {} {
    if s.len() == 0 {
        return Err(ParseFloatError::Empty);
    }
    let negative = s[0] == 0x2D;
    let mut i: usize = if negative || s[0] == 0x2B { 1 } else { 0 };
    let rest = match s.split_at(i) {
        Some((_, rest)) => rest,
        None => return Err(ParseFloatError::Invalid),
    };
    if is_word(rest, b"inf") || is_word(rest, b"infinity") {
        return Ok(Parts::Infinite(negative));
    }
    if is_word(rest, b"nan") {
        return Ok(Parts::NaN);
    }
    // The significant digits, from the first that isn't 0, as a `Big`;
    // `point` how many digits of the value come before the point, of
    // those kept, and whether any after those kept isn't 0.
    let mut big = Big::zero();
    let mut kept: usize = 0;
    let mut point: i64 = 0;
    let mut beyond = false;
    let mut any = false;
    let mut seen_point = false;
    while i < s.len() invariant kept <= 800 && point >= -1_000_000_000 && point <= 1_000_000_000 {
        let c = s[i];
        if c == 0x2E && !seen_point {
            seen_point = true;
        } else if c >= 0x30 && c <= 0x39 {
            any = true;
            let d = (c - 0x30) as u32;
            if kept == 0 && d == 0 {
                if seen_point && point > -1_000_000_000 {
                    point = point - 1;
                }
            } else if kept < MOST_DIGITS {
                big.mul_small(10);
                big.add(&Big::of(d as u64));
                kept = kept + 1;
                if !seen_point && point < 1_000_000_000 {
                    point = point + 1;
                }
            } else {
                beyond = beyond || d != 0;
                if !seen_point && point < 1_000_000_000 {
                    point = point + 1;
                }
            }
        } else {
            break;
        }
        i = i + 1;
    }
    if !any {
        return Err(ParseFloatError::Invalid);
    }
    let mut exp: i64 = 0;
    if i < s.len() && (s[i] == 0x65 || s[i] == 0x45) {
        i = i + 1;
        let minus = i < s.len() && s[i] == 0x2D;
        if i < s.len() && (s[i] == 0x2D || s[i] == 0x2B) {
            i = i + 1;
        }
        let mut digits = false;
        while i < s.len() invariant exp >= 0 && exp < 1_000_000 {
            let c = s[i];
            if c < 0x30 || c > 0x39 {
                break;
            }
            // Past this, it's infinite or 0 whatever the digits are.
            if exp >= 0 && exp < 100_000 {
                exp = exp * 10 + ((c - 0x30) as i64);
            }
            digits = true;
            i = i + 1;
        }
        if !digits {
            return Err(ParseFloatError::Invalid);
        }
        if minus {
            exp = -exp;
        }
    }
    if i != s.len() {
        return Err(ParseFloatError::Invalid);
    }
    if kept == 0 {
        return Ok(Parts::Finite(negative, 0, 0));
    }
    if beyond {
        big.mul_small(10);
        big.add(&Big::of(1));
        kept = kept + 1;
    }
    // The value is `big · 10^ten`, and about `10^(point + exp)`.
    let size = point + exp;
    if size > 310 {
        return Ok(Parts::Infinite(negative));
    }
    if size < -330 {
        return Ok(Parts::Finite(negative, 0, 0));
    }
    // `kept` is at most `MOST_DIGITS + 1`, which a loop's checking doesn't
    // follow.
    let kept = if kept > 1000 { 1000 } else { kept };
    Ok(nearest(negative, big, size - (kept as i64), f))
}

/// The float of type `f` nearest `big · 10^ten`, positive, ties to even,
/// of the sign `negative` says.
fn nearest(negative: bool, big: Big, ten: { t: i64 | t >= -2000 && t <= 400 }, f: Format) -> Parts {
    // `u / v` is the value.
    let mut u = big;
    let mut v = Big::of(1);
    if ten >= 0 {
        u.mul_pow10(index(ten));
    } else {
        v.mul_pow10(neg_index(ten));
    }
    // `e`, with `u / (v · 2^e)` from 2^(p-1) to just under 2^p, or the
    // least exponent, for a subnormal.
    let p = f.p as i64;
    let mut e = (u.bits() as i64) - (v.bits() as i64) - p;
    for _ in 0..8u32 {
        let q = quotient(&u, &v, e);
        if q >= f.high {
            e = saturating_add(e, 1);
        } else if q < f.low && e > (f.least as i64) {
            e = saturating_sub(e, 1);
        } else {
            break;
        }
    }
    if e < (f.least as i64) {
        e = f.least as i64;
    }
    // `q`, and whether what's left is past half, half, or less.
    let mut num = u;
    let mut den = v;
    if e >= 0 {
        den.shl(index(e));
    } else {
        num.shl(neg_index(e));
    }
    let q = divide(&mut num, &den);
    num.shl(1);
    let up = match num.cmp(&den) {
        Ordering::Greater => true,
        Ordering::Equal => q % 2 == 1,
        Ordering::Less => false,
    };
    let mut m = if up && q < 0xFFFF_FFFF_FFFF_FFFF { q + 1 } else { q };
    if m == f.high {
        m = f.low;
        e = saturating_add(e, 1);
    }
    if e > (f.most as i64) {
        return Parts::Infinite(negative);
    }
    Parts::Finite(negative, m, e as i32)
}

/// `u / (v · 2^e)`, rounding down, or 2^63 if that's more.
fn quotient(u: &Big, v: &Big, e: i64) -> u64 {
    let mut num = *u;
    let mut den = *v;
    if e >= 0 {
        den.shl(index(e));
    } else {
        num.shl(neg_index(e));
    }
    divide(&mut num, &den)
}

/// `num / den`, rounding down, leaving the remainder in `num`; 2^63 if the
/// quotient is that or more.
fn divide(num: &mut Big, den: &Big) -> u64 {
    if den.is_zero() {
        return 0;
    }
    let mut q: u64 = 0;
    let mut bit: usize = 64;
    while bit > 0 {
        bit = bit - 1;
        let mut shifted = *den;
        shifted.shl(bit);
        if !num.lt(&shifted) {
            if bit == 63 {
                return 0x8000_0000_0000_0000;
            }
            num.sub(&shifted);
            q = q | pow2_64(bit);
        }
    }
    q
}
