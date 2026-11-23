// `enumerate` of a bounded iterator can't panic: its index and what's
// left are never more than a `usize` holds, an invariant of `Enumerate`
// where the iterator is bounded (docs/93-invariants-where.md), which its
// associated effect says: `{I::e, panic unless I: BoundedIterator}`
// (docs/96-associated-effects-unless.md).
use alloc::list::List;
use core::iter::Enumerate;

/// Over a slice: no `panic`.
fn total(s: &[u64]) -> u64 with {} {
    let mut t = 0;
    for (i, x) in s.iter().enumerate() {
        if i == 1 && *x < 100 {
            t = *x;
        }
    }
    t
}

/// Generic in a bounded iterator: only what it does.
fn last<I: BoundedIterator<Item = u64> + Discard>(it: I) -> usize with I::e {
    let mut n = 0;
    for (i, _x) in it.enumerate() {
        n = i;
    }
    n
}

/// Generic in any: it may panic.
fn last_any<I: Iterator<Item = u64> + Discard>(it: I) -> usize with {I::e, panic} {
    let mut n = 0;
    for (i, _x) in it.enumerate() {
        n = i;
    }
    n
}

/// Generic in any, but saying it can't panic of a bounded one: checked
/// again with `I: BoundedIterator`, where `enumerate`'s `next` has none.
fn last_unless<I: Iterator<Item = u64> + Discard>(it: I) -> usize
with {I::e, panic unless I: BoundedIterator}
{
    let mut n = 0;
    for (i, (j, _x)) in it.enumerate().enumerate() {
        n = i + 0 * j;
    }
    n
}

/// Saying the associated effect of two `enumerate`s, which hangs on two
/// bounds, `Enumerate<I>: BoundedIterator` and `I: BoundedIterator`: the
/// body is walked again under both at once.
fn pairs<I: Iterator<Item = u64> + Discard>(it: I) -> usize
with <Enumerate<Enumerate<I>> as Iterator>::e
{
    let mut n = 0;
    for (i, (_j, _x)) in it.enumerate().enumerate() {
        n = i;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a = [1u64, 2, 3, 4];
    out.line(&total(&a))?;                      // expect: 2
    out.line(&last(a.iter().copied()))?;        // expect: 3
    out.line(&last_any(a.iter().copied()))?;    // expect: 3
    out.line(&last_unless(a.iter().copied()))?; // expect: 3
    out.line(&pairs(a.iter().copied()))?;       // expect: 3
    let mut l: List<u64> = List::new();
    l.push(5);
    l.push(6);
    out.line(&last_unless(l.into_iter()))?; // expect: 1
    Ok(())
}
