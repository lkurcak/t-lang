// A const named only in types is used, and what's known of it is known to
// a function checked before anything names it in a body.
fn second(a: &[u8; WIDTH]) -> u8 with {} { a[1] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; WIDTH] = core::array::from_fn(|i| i as u8);
    out.line(&second(&a))?; // expect: 1
    Result::Ok(())
}

const WIDTH: usize = 3;
