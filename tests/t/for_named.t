// A `for` can name the iterator it goes through, `for x in it: e`, and its
// invariant and body can use it: what's left, `it.len()`, says how far the
// loop has got, so a count kept in it is proved not to overflow, and an
// index kept with it is in bounds (docs/84-counting.md).
use alloc::list::List;

/// How many values of `l` are more than 1: no more than it has.
fn more_than_one(l: &List<u64>) -> { n: usize | n <= l.len() } with {} {
    let mut n: usize = 0;
    for x in it: l.iter() invariant n + it.len() <= l.len() {
        if *x > 1 {
            n = n + 1;
        }
    }
    n
}

/// A range's iterator says where it's up to: `it.start()`.
fn ones(a: &[u8; 64]) -> { m: u8 | m <= 64 } with {} {
    let mut m: u8 = 0;
    for s in it: 0..64u8 invariant m <= it.start() && it.start() <= it.end() && it.end() == 64 {
        if a[s as usize] == 1 {
            m = m + 1;
        }
    }
    m
}

/// An index kept beside the loop is less than the length while it runs:
/// so it can index another slice as long.
fn same(a: &[u64], b: { b: &[u64] | b.len() == a.len() }) -> usize with {} {
    let mut i: usize = 0;
    let mut n: usize = 0;
    for x in it: a invariant i + it.len() == a.len() && n <= i {
        if *x == b[i] {
            n = n + 1;
        }
        i = i + 1;
    }
    n
}

/// The body can take a value out itself, skipping it: what's left still
/// counts down.
fn skip_after_zero(a: &[u64]) -> { n: usize | n <= a.len() } with {} {
    let mut n: usize = 0;
    for x in it: a invariant n + it.len() <= a.len() {
        if *x == 0 {
            let _ = it.next();
        }
        n = n + 1;
    }
    n
}

/// Through `&mut`, by value, and copied from the back.
fn kinds() -> { s: usize | s <= 8 } with {} {
    let mut a: [u64; 3] = [5, 6, 7];
    let mut n: usize = 0;
    for x in it: &mut a invariant n + it.len() == 3 {
        *x = *x / 2;
        n = n + 1;
    }
    let mut b: usize = 0;
    for _x in a.iter().copied().rev() invariant b <= 1 {
        b = b + 1;
        if b == 2 {
            break;
        }
    }
    let mut c: usize = 0;
    for _x in it: a invariant c + it.len() == 3 {
        c = c + 1;
    }
    n + b + c
}

/// A list by value: it's used up, as many times round as it had values.
fn all(l: List<u64>) -> usize with {} {
    let total = l.len();
    let mut k: usize = 0;
    for _x in it: l invariant k + it.len() == total {
        k = k + 1;
    }
    k
}

/// The pattern may have the iterator's name: in the body, it's the value.
/// That warns (docs/85-values-in-predicates.md).
fn shadow(a: &[u64]) -> u64 with {} {
    let mut s: u64 = 0;
    'outer: for it in it: a.iter().copied() invariant it.len() <= a.len() { // warning: the `for`'s pattern binds `it`, which hides the iterator `it` in its body
        s = s | it;
        if s > 100 {
            break 'outer;
        }
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u64> = List::new();
    l.push(1);
    l.push(5);
    l.push(9);
    out.line(&more_than_one(&l))?; // expect: 2
    let mut a = [0u8; 64];
    a[3] = 1;
    a[60] = 1;
    out.line(&ones(&a))?;                       // expect: 2
    out.line(&same(&[1, 2, 3], &[1, 0, 3]))?;   // expect: 2
    out.line(&skip_after_zero(&[0, 7, 1, 0]))?; // expect: 3
    out.line(&kinds())?;                        // expect: 8
    out.line(&all(l))?;                         // expect: 3
    out.line(&shadow(&[1, 2, 4]))?;             // expect: 7
    Ok(())
}
