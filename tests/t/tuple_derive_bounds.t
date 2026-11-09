// A derive on a struct with a tuple field needs of its parameters what
// the tuple's values need: `W<T>` is `Copy` and `Eq` when `T` is.
#[derive(Copy, Discard, Eq, Ord, Debug)]
struct W<T> { p: (T, u8) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w = W { p: ('x', 1) };
    let a = w;
    let b = w;
    out.line(&(a == b))?;                     // expect: true
    out.line(&(a < W { p: ('x', 2) }))?;      // expect: true
    out.line(&f"{b:?}")                       // expect: W { p: ('x', 1) }
}
