// Code generic in an `ExactSizeIterator` counts what it's given, knowing
// `next` takes one off `len` and gives `None` only at 0: it's a law of
// `next`, which every impl keeps (docs/88-iterator-laws.md).
use alloc::list::List;

fn count<I: ExactSizeIterator + Discard>(it: I) -> { n: usize | n == it.len() }
with I::e
where
    I::Item: Discard,
{
    let mut it = it;
    let start = it.len();
    let mut n: usize = 0;
    while let Some(_x) = it.next() invariant n + it.len() == start {
        n = n + 1;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = [1u64, 2, 3];
    out.line(&count(a.iter()))?;          // expect: 3
    out.line(&count(a.iter().copied()))?; // expect: 3
    let mut l: List<u8> = List::new();
    l.push(7);
    l.push(8);
    let n: { n: usize | n == 2 } = count(l.into_iter());
    out.line(&n)?; // expect: 2
    // From the back too: `Iter` is double-ended.
    let mut it = a.iter();
    let last = it.next_back();
    let k: { k: usize | k == 2 } = it.len();
    out.line(&k)?; // expect: 2
    let some: { b: bool | b } = last.is_some();
    out.line(&some)?; // expect: true
    Ok(())
}
