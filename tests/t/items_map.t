// What `map` gives is what its function gives: a closure's result, as its
// body knows it where it returns (docs/74), or a function's refined result,
// through the adapters after it too; and the closure is given only what
// the iterator gives (docs/82-item-invariants.md).
use alloc::list::List;

fn half(v: &u32) -> { r: u32 | r <= 50 } { (*v % 100) / 2 }

fn need50(r: { r: u32 | r <= 50 }) -> u32 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<u32> = List::new();
    l.push(7);
    l.push(250);
    l.push(99);
    for h in l.iter().map(|v| half(v)) {
        out.line(&need50(h))?;
    }
    // expect: 3
    // expect: 25
    // expect: 49
    for h in l.iter().map(half).rev() {
        out.line(&need50(h))?;
    }
    // expect: 49
    // expect: 25
    // expect: 3
    let a: [u8; 4] = [1, 2, 3, 4];
    for i in (0..8).map(|x| x / 2).skip(5) {
        out.line(&a[i])?;
    }
    // expect: 3
    // expect: 4
    // expect: 4
    for (k, i) in (0..20).map(|x| x % 4).skip(17).enumerate() {
        out.line(&f"{k} {a[i]}")?;
    }
    // expect: 0 2
    // expect: 1 3
    // expect: 2 4
    let mut m = (0..4).rev().map(|x| a[x]);
    match m.next() {
        Some(v) => out.line(&v)?, // expect: 4
        None => {},
    }
    Ok(())
}
