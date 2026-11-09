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
#[derive(Discard, Copy, Debug, Eq, Send, Sync)]
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

}

/// The greater of `a` and `b`, `b` if they're equal. The other one is
/// thrown away.
///
/// A function, not a method of `Ord`: an impl could make a value in a
/// method of its own, and `Ord`'s methods only take values and give back
/// none, so a type parameter `Ord` bounds can be a refined type (see
/// docs/60-refined-type-arguments.md). What it gives is `a` or `b`.
export fn max<T: Ord + Discard>(a: T, b: T) -> T {
    if b.lt(&a) { a } else { b }
}

/// The lesser of `a` and `b`, `a` if they're equal. The other one is
/// thrown away.
export fn min<T: Ord + Discard>(a: T, b: T) -> T {
    if b.lt(&a) { b } else { a }
}

/// Integers in their order, which `<` says.
fn compare<T: Int>(a: T, b: T) -> Ordering {
    if a < b { Ordering::Less } else if a == b { Ordering::Equal } else { Ordering::Greater }
}

/// Integers in their order, which `<` says: one impl for all ten types,
/// which overlaps none of the others, as `Int` is sealed (see
/// docs/29-closed-and-sealed-traits.md).
impl<T: Int> Ord for T {
    fn cmp(&self, other: &T) -> Ordering { compare(*self, *other) }
}

/// Characters by their numbers, the Unicode scalar values: `'a'` before
/// `'b'`, and `'z'` before `'é'`.
impl Ord for char {
    fn cmp(&self, other: &char) -> Ordering { compare(*self as u32, *other as u32) }
}

/// References by what they point to, as `==` compares them: so `a < b` of
/// two `&str`s compares the strings, and of two `&[T]`s the slices.
impl<'a, unsized T: Ord> Ord for &'a T {
    fn cmp(&self, other: &&'a T) -> Ordering { (**self).cmp(*other) }
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
