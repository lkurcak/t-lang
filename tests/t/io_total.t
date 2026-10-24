// Writing has no effect of its own: what a function may do to the world is
// in what it's handed, not in its `with`. Neither this `main` nor `count`
// can panic or allocate, and both write.
use core::fmt::Write;

fn count(out: &mut std::io::Stdout, n: u8) -> Result<(), std::io::Error> {
    for i in 0..n {
        out.put(&i)?;
    }
    out.write_char('\n')
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    count(&mut out, 5) // expect: 01234
}
