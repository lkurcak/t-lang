// `Self::Out` in a blanket impl is what the impl says, `u64`, and nothing
// else.
trait Widen {
    type Out;
    fn widen(self) -> Self::Out;
}

impl<T: Int> Widen for T {
    type Out = u64;
    fn widen(self) -> u32 { core::num::wrap(self) }
}

fn main() {}
// error: method `widen` doesn't match its declaration in trait `Widen`: it should return `u64`, not `u32`
