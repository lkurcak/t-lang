// `Int` is sealed, so one impl covers every integer type beside impls for
// others: what t-hashmap needed ten impls for (see
// docs/29-closed-and-sealed-traits.md).
use core::num::wrap;

trait Hash {
    fn hash(&self) -> u64;
}

impl<T: Int> Hash for T {
    fn hash(&self) -> u64 { wrap(*self) }
}

impl Hash for bool {
    fn hash(&self) -> u64 { if *self { 1 } else { 0 } }
}

impl<'a> Hash for &'a str {
    fn hash(&self) -> u64 { self.len() as u64 }
}

impl<'a, T: Hash> Hash for &'a T {
    fn hash(&self) -> u64 { (**self).hash() }
}

fn hash<T: Hash>(x: &T) -> u64 { x.hash() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(-1i8).hash())?;  // expect: 18446744073709551615
    out.line(&300u16.hash())?;  // expect: 300
    out.line(&true.hash())?;    // expect: 1
    out.line(&hash(&"abc"))?;   // expect: 3
    out.line(&(&7i64).hash())?; // expect: 7
    Result::Ok(())
}
