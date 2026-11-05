// Variants brought in from another file's module, and from the crate
// root, in what `if let`, `while let` and `let .. else` mean: each has an
// arm for the variants its pattern misses, named as its pattern's enum.
mod tokens;

use Step::{Done, Next};
use tokens::Tok::{End, Num};

#[derive(Discard, Copy)]
enum Step { Next(i64), Done }

fn step(n: i64) -> Step {
    if n > 0 { Next(n - 1) } else { Done }
}

fn countdown(from: i64) -> i64 {
    let mut n = from;
    let mut steps = 0i64;
    while let Next(m) = step(n) {
        n = m;
        steps = steps + 1;
    }
    steps
}

fn first(t: tokens::Tok) -> i64 {
    let Num(n) = t else {
        return -1;
    };
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = if let Done = step(0) { 1i64 } else { 0 };
    out.line(&f"{a} {countdown(3)} {first(Num(7))} {first(End)} {tokens::sum()}")?; // expect: 1 3 7 -1 5
    Ok(())
}
