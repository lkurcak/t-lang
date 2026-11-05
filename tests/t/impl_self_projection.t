// Inside an impl of a trait, the trait's associated types of the impl's
// own type are what the impl says, even when that type is a type
// parameter, as in a blanket impl: here `Self::Out` is `u64` in the impl
// for every `T: Int`.
trait Widen {
    type Out;
    fn widen(self) -> Self::Out;
    fn zero() -> Self::Out;
}

impl<T: Int> Widen for T {
    type Out = u64;
    fn widen(self) -> u64 { core::num::wrap(self) }
    fn zero() -> Self::Out { 0 }
}

fn both<T: Widen>(a: T) -> T::Out { a.widen() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w: u64 = both(7u8);
    out.line(&w)?;                 // expect: 7
    out.line(&(u16::zero() + 1))?; // expect: 1
    Result::Ok(())
}
