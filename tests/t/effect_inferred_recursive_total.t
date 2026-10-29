// A recursive group whose bodies have no effects of their own has none:
// the least solution, not the most. So an exported function that says
// nothing can call it.
fn even(n: u64) -> bool {
    if n == 0 { true } else { odd(n - 1) }
}

fn odd(n: u64) -> bool {
    if n == 0 { false } else { even(n - 1) }
}

export fn parity(n: u64) -> bool { even(n) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&parity(7))?; // expect: false
    Result::Ok(())
}
