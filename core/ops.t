// What operators are written with.

use crate::num::checked_add;

/// `start..end`: from `start` up to, not including, `end`. It's empty if
/// `start` isn't before `end`. Its fields are private, so only `a..b` and
/// `Range::new` make one.
#[derive(Discard where T: Discard, Eq where T: Eq)]
pub struct Range<T> { start: T, end: T }

impl<T> Range<T> {
    /// `start..end`.
    pub fn new(start: T, end: T) -> Range<T> { Range { start, end } }
}

impl<T: Copy> Range<T> {
    /// Where it starts: where an iterator over it is up to.
    pub fn start(&self) -> T { self.start }

    /// Where it ends, not included.
    pub fn end(&self) -> T { self.end }
}

/// Counts up, and can't panic: it adds one only to what's less than the
/// end, which can't overflow, even at the end of `int`. The checker can't
/// see that, so it's a `checked_add`, whose `None` never happens.
impl Iterator for Range<int> {
    type Item = int;
    effect e = {};

    fn next(&mut self) -> Option<int> {
        if self.start < self.end {
            let n = self.start;
            self.start = match checked_add(n, 1) {
                Option::Some(m) => m,
                Option::None => self.end,
            };
            Option::Some(n)
        } else {
            Option::None
        }
    }
}
