// `with {}` says a function has no effects, where it would be inferred:
// a promise its body is checked against, as if it were exported.
fn quiet(n: i64) -> i64 with {} { core::num::wrapping_mul(n, 2) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&quiet(4))?; // expect: 8
    Result::Ok(())
}
