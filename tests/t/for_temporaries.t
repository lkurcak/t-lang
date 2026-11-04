// What a `for`'s head makes and borrows lives until the statement the
// `for` is in ends: a const's value, a call's result. Then iterating a
// table needs no `let` first.
const WORDS: [&str; 3] = ["one", "two", "three"];

fn squares() -> [u32; 4] { [1, 4, 9, 16] }

fn total() -> u32 {
    let mut n: u32 = 0;
    for x in squares().iter() {
        n = n + *x;
    }
    n
}

// The `for` is the block's last expression.
fn tail(out: &mut std::io::Stdout) -> Result<(), std::io::Error> {
    'words: for w in WORDS.iter() {
        if *w == "three" { break 'words; }
        out.line(w)?;
    }
    Result::Ok(())
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for w in WORDS.iter() {
        for x in squares().as_slice().iter() {
            if *x == 4 { out.line(w)?; }
        }
    }
    out.line(&total())?;
    tail(&mut out)?;
    Result::Ok(())
}
// expect: one
// expect: two
// expect: three
// expect: 30
// expect: one
// expect: two
