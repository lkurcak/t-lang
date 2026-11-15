// Two impls overlap when one's function type could be the other's: here
// `fn(u32) -> u32` implements `Tr`, so the blanket impl is for it too.
trait Tr {
    fn go(&self) -> u32;
}

trait B {
    type Out;
    fn make(&self) -> Self::Out;
}

impl Tr for fn(u32) -> u32 {
    fn go(&self) -> u32 { 1 }
}

impl<T: Tr> B for T {
    type Out = u32;
    fn make(&self) -> u32 { 7 }
}

impl B for fn(u32) -> u32 {
    type Out = bool;
    fn make(&self) -> bool { true }
}

fn main() {}
// error: this impl of `B` overlaps `impl B for T`: a type can be both `fn(u32) -> u32` and `T`, and then which impl it uses would be a choice
