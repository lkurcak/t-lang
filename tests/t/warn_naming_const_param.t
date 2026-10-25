// A const parameter is a constant, so its name is upper case, as `N` or
// `CAP`.
fn f<const cap: usize>() -> usize { cap }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f::<2>())?; // expect: 2
    Result::Ok(())
}
// warning: const parameter `cap` should have an upper case name: `CAP`
