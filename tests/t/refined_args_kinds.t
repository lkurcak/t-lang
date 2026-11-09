// A refinement can be the type behind a reference or a box, or a slice's
// or an array's elements (docs/60-refined-type-arguments.md).
use alloc::boxed::Box;

fn deref(a: [u8; 4], r: &{ v: usize | v < 4 }) -> u8 { a[*r] }

fn unbox(a: [u8; 4], b: Box<{ v: usize | v < 4 }>) -> u8 { a[*b] }

fn first(a: [u8; 4], s: &[{ v: usize | v < 4 }]) -> u8 {
    match s.first() {
        Some(i) => a[*i],
        None => 0,
    }
}

fn pick(a: [u8; 4], idx: [{ v: usize | v < 4 }; 2]) -> u8 { a[idx[1]] }

// Weakened behind a shared reference, as nothing can write through one.
fn wider(r: &{ v: usize | v < 2 }) -> &{ v: usize | v < 4 } { r }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u8; 4] = [5, 6, 7, 8];
    let i: { v: usize | v < 4 } = 3;
    out.line(&deref(a, &i))?; // expect: 8
    out.line(&unbox(a, Box::new(2)))?; // expect: 7
    let idx: [{ v: usize | v < 4 }; 2] = [1, 0];
    out.line(&first(a, idx.as_slice()))?; // expect: 6
    out.line(&pick(a, idx))?; // expect: 5
    let one: { v: usize | v < 2 } = 1;
    out.line(&deref(a, wider(&one)))?; // expect: 6
    Ok(())
}
