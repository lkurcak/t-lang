//! Comparing and ordering.

/// What `==` and `!=` compare with: `a == b` is `a.eq(&b)`.
///
/// The integers, `bool`, `char`, `()` and `str` have it, and references,
/// boxes and slices of `Eq` values, which compare what they point to. A
/// struct or enum has it with `#[derive(Eq)]`, field by field, or with an
/// impl of its own. Functions don't, and can't: whether two do the same
/// for every argument can't be decided.
#[intrinsic]
export trait Eq {
    /// Is `self` equal to `other`?
    fn eq(&self, other: &Self) -> bool;
}

/// How one value compares with another.
#[derive(Discard, Copy, Eq)]
export enum Ordering { Less, Equal, Greater }

/// A total order: any two values compare, one way. `a < b` is
/// `a.lt(&b)`, and `<=`, `>` and `>=` alike, but for the integers, whose
/// `<` is the machine's, and which the checker reasons about.
///
/// An impl must be one: `cmp` is `Equal` exactly when `==` is true, if `a`
/// is less than `b` then `b` is greater than `a`, and if `a` is less than
/// `b` and `b` than `c`, then `a` is less than `c`. Nothing checks this, and
/// nothing relies on it to be sound: an impl that breaks it only gives
/// wrong answers.
export trait Ord: Eq {
    /// How `self` compares with `other`.
    fn cmp(&self, other: &Self) -> Ordering;

    /// Is `self` less than `other`?
    fn lt(&self, other: &Self) -> bool {
        match self.cmp(other) {
            Ordering::Less => true,
            _ => false,
        }
    }

    /// Is `self` less than or equal to `other`?
    fn le(&self, other: &Self) -> bool {
        match self.cmp(other) {
            Ordering::Greater => false,
            _ => true,
        }
    }

    /// Is `self` greater than `other`?
    fn gt(&self, other: &Self) -> bool {
        match self.cmp(other) {
            Ordering::Greater => true,
            _ => false,
        }
    }

    /// Is `self` greater than or equal to `other`?
    fn ge(&self, other: &Self) -> bool {
        match self.cmp(other) {
            Ordering::Less => false,
            _ => true,
        }
    }

    /// The greater of the two, `other` if they're equal. The other one is
    /// thrown away.
    fn max(self, other: Self) -> Self where Self: Discard {
        if other.lt(&self) { self } else { other }
    }

    /// The lesser of the two, `self` if they're equal. The other one is
    /// thrown away.
    fn min(self, other: Self) -> Self where Self: Discard {
        if other.lt(&self) { other } else { self }
    }
}

/// Integers in their order, which `<` says.
fn compare<T: Int>(a: T, b: T) -> Ordering {
    if a < b { Ordering::Less } else if a == b { Ordering::Equal } else { Ordering::Greater }
}

impl Ord for i8 {
    fn cmp(&self, other: &i8) -> Ordering { compare(*self, *other) }
}

impl Ord for i16 {
    fn cmp(&self, other: &i16) -> Ordering { compare(*self, *other) }
}

impl Ord for i32 {
    fn cmp(&self, other: &i32) -> Ordering { compare(*self, *other) }
}

impl Ord for i64 {
    fn cmp(&self, other: &i64) -> Ordering { compare(*self, *other) }
}

impl Ord for isize {
    fn cmp(&self, other: &isize) -> Ordering { compare(*self, *other) }
}

impl Ord for u8 {
    fn cmp(&self, other: &u8) -> Ordering { compare(*self, *other) }
}

impl Ord for u16 {
    fn cmp(&self, other: &u16) -> Ordering { compare(*self, *other) }
}

impl Ord for u32 {
    fn cmp(&self, other: &u32) -> Ordering { compare(*self, *other) }
}

impl Ord for u64 {
    fn cmp(&self, other: &u64) -> Ordering { compare(*self, *other) }
}

impl Ord for usize {
    fn cmp(&self, other: &usize) -> Ordering { compare(*self, *other) }
}

/// Characters by their numbers, the Unicode scalar values: `'a'` before
/// `'b'`, and `'z'` before `'é'`.
impl Ord for char {
    fn cmp(&self, other: &char) -> Ordering { compare(*self as u32, *other as u32) }
}

/// References by what they point to, as `==` compares them: so `a < b` of
/// two `&str`s compares the strings.
impl<'a, T: Ord> Ord for &'a T {
    fn cmp(&self, other: &&'a T) -> Ordering { (**self).cmp(*other) }
}

/// String slices by the strings, which `&'a T` can't say, as `T` can't be
/// `str`.
impl<'a> Ord for &'a str {
    fn cmp(&self, other: &&'a str) -> Ordering { (**self).cmp(*other) }
}

/// `false` before `true`: the order in which `a <= b` means `a` implies `b`.
impl Ord for bool {
    fn cmp(&self, other: &bool) -> Ordering {
        if *self == *other {
            Ordering::Equal
        } else if *other {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}
