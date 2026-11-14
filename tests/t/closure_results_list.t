// What a closure gives back is a value it made: what's said of its
// elements, and its measures, come back with it, of the value each run
// gives (docs/74).
use alloc::list::List;

fn call<R, effect e, F: FnOnce() -> R with e + Discard>(f: F) -> R with e { f() }

fn below(n: usize) -> List<{ t: usize | t < n }> with alloc {
    let mut l: List<{ t: usize | t < n }> = List::new();
    for i in 0..n {
        l.push(i);
    }
    l
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l: List<{ t: usize | t < 4 }> = call(|| below(4));
    let a: [u8; 4] = [1, 2, 3, 4];
    for i in &l {
        out.line(&a[*i])?;
    }
    // expect: 1
    // expect: 2
    // expect: 3
    // expect: 4
    let m: { l: List<u8> | l.len() == 3 } = call(|| List::filled(3, 7));
    out.line(&m[2])?; // expect: 7
    Ok(())
}
