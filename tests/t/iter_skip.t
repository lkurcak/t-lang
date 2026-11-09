// `skip(n)` gives what's after the first `n` values, throwing those away;
// what it gives has what the iterator's values have (docs/61-proof-search.md).
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    for x in (0..5u8).skip(2) {
        out.line(&x)?;
    }
    // expect: 2
    // expect: 3
    // expect: 4
    for x in (0..2u8).skip(5) {
        out.line(&x)?;
    }
    let a = [10u8, 20, 30];
    let mut l: List<{ i: usize | i < 3 }> = List::new();
    l.push(0);
    l.push(2);
    for i in l.iter().skip(1) {
        out.line(&a[*i])?; // expect: 30
    }
    Ok(())
}
