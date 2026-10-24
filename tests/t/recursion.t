// Recursion and mutual recursion. Functions can be used before they're defined.

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&fact(10))?;        // expect: 3628800
    out.line(&fib(20))?;         // expect: 6765
    out.line(&b(is_even(10)))?;  // expect: 1
    out.line(&b(is_odd(7)))?;    // expect: 1
    out.line(&sum_to(5000))?;    // expect: 12502500
    Result::Ok(())
}

fn fact(n: i64) -> i64 with panic { if n <= 1 { 1 } else { n * fact(n - 1) } }
fn fib(n: i64) -> i64 with panic { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn is_even(n: i64) -> bool with panic { if n == 0 { true } else { is_odd(n - 1) } }
fn is_odd(n: i64) -> bool with panic { if n == 0 { false } else { is_even(n - 1) } }
fn sum_to(n: i64) -> i64 with panic { if n == 0 { 0 } else { n + sum_to(n - 1) } }
fn b(x: bool) -> i64 { if x { 1 } else { 0 } }
