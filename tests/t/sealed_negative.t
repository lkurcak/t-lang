// A sealed trait's impls are all in its package, so what has none can't
// get one: `impl<T: Small> Describe for T` doesn't overlap `impl Describe
// for char`, as `char` isn't `Small` and never will be. Without `sealed`
// that's an overlap (tests/t/err_sealed_overlap_open.t). See
// docs/29-closed-and-sealed-traits.md.
sealed trait Small {}

impl Small for u8 {}

impl Small for bool {}

trait Describe {
    fn describe(&self) -> i64;
}

impl<T: Small> Describe for T {
    fn describe(&self) -> i64 { 1 }
}

impl Describe for char {
    fn describe(&self) -> i64 { 2 }
}

impl<'a, T: Describe> Describe for &'a T {
    fn describe(&self) -> i64 { 20 + (**self).describe() }
}

impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> i64 {
        match self {
            Option::Some(x) => 10 + x.describe(),
            Option::None => 10,
        }
    }
}

fn describe<T: Describe>(x: &T) -> i64 { x.describe() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&describe(&5u8))?;                   // expect: 1
    out.line(&describe(&'a'))?;                   // expect: 2
    out.line(&describe(&Option::Some(true)))?;    // expect: 11
    out.line(&describe(&&Option::Some(&'b')))?;   // expect: 52
    Result::Ok(())
}
