// What's known of a const parameter is what's learned of it, as of any
// `usize`: `N > 2` tells the checker `a[2]` is there, in every copy.
fn third<const N: usize>(a: &[i64; N]) -> i64 {
    if N > 2 { a[2] } else { -1 }
}

/// A pattern looks through a reference to an array, binding references.
fn swap_pair(p: &mut [i64; 2]) {
    match p {
        [a, b] => core::mem::swap(a, b),
    }
}

fn none(e: [bool; 0]) -> i64 {
    match e {
        [] => 0,
    }
}

fn count<const N: usize>() -> usize {
    let f = || N;
    f()
}

fn shadow<const N: usize>() -> usize {
    let N = 7;
    N
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&third(&[1, 2, 3]))?;   // expect: 3
    out.line(&third(&[1, 2]))?;      // expect: -1
    let mut p = [1, 2];
    swap_pair(&mut p);
    out.line(&p[0])?;                // expect: 2
    out.line(&none([]))?;            // expect: 0
    out.line(&count::<5>())?;        // expect: 5
    out.line(&shadow::<5>())?;       // expect: 7
    Result::Ok(())
}
// warning: variable `N` should have a snake case name: `n`
