// Comparing and ordering.

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

impl Ord for int {
    fn cmp(&self, other: &int) -> Ordering {
        if *self < *other {
            Ordering::Less
        } else if *self == *other {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
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
