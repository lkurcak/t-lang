// A loop's invariant may have several clauses, and one may be said only
// where a bound holds: `invariant p where T: Trait`. It can name the
// bound's trait's measures, and is assumed and proved only by a walk whose
// bounds prove the bound. `old(x)` in one is what a parameter was when the
// function was called, and `greatest::<usize>()` the greatest `usize`
// (docs/92-where-a-bound-holds.md).
use core::num::greatest;

/// How many values `it` gives, at most `cap`: of any iterator.
fn capped<I: Iterator + Discard>(it: I, cap: usize) -> { n: usize | n <= cap }
with I::e
where
    I::Item: Discard,
{
    let mut n: usize = 0;
    for _x in jt: it
        invariant n <= cap
        invariant n + jt.most() <= greatest::<usize>() where I: BoundedIterator
    {
        if n < cap {
            n = n + 1;
        }
    }
    n
}

/// Half of `n`, or less: counting down from it.
fn halve(n: u8) -> { r: u8 | 2 * r <= n } {
    let mut m = n;
    while 2 * m > n invariant m <= old(n) {
        m = m - 1;
    }
    m
}

/// What's left of `it` after `k` values, by its bound: of one with one.
fn drain<I: BoundedIterator>(it: &mut I, k: usize)
with I::e
where
    I::Item: Discard,
ensures
    it.most() <= old(it.most()),
{
    let mut i: usize = 0;
    while i < k invariant it.most() <= old(it.most()) {
        match it.next() {
            Some(x) => x.discard(),
            None => return,
        }
        i = i + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1u8, 2, 3, 4, 5];
    out.line(&capped(a.iter(), 3))?; // expect: 3
    out.line(&capped(a.iter(), 9))?; // expect: 5
    out.line(&halve(9))?;            // expect: 4
    let mut it = a.iter();
    drain(&mut it, 2);
    out.line(&it.len())?; // expect: 3
    Ok(())
}
