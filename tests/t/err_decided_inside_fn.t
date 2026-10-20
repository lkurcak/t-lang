// A bound decides a parameter only through one that's in the impl's type
// outside function types: matching decides `A` here as a type that may hold
// a lifetime only `fn(A)` means, and `U` would have it too.
trait Tr {
    fn go(&self) -> int;
}

impl<U, A: Iterator<Item = U>> Tr for fn(A) {
    fn go(&self) -> int { 1 }
}

fn main() {}
// error: type parameter `U` isn't in `fn(A) -> ()`, nor decided by a bound
