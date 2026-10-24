//! Comparing and ordering.

/// How one value compares with another.
#[derive(Discard, Copy, Eq)]
pub enum Ordering { Less, Equal, Greater }

/// A total order: any two values compare, one way.
///
/// An impl must be one: `cmp` is `Equal` exactly when `==` is true, if `a`
/// is less than `b` then `b` is greater than `a`, and if `a` is less than
/// `b` and `b` than `c`, then `a` is less than `c`. Nothing checks this, and
/// nothing relies on it to be sound: an impl that breaks it only gives
/// wrong answers.
pub trait Ord: Eq {
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
