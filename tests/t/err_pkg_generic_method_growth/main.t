// `Y2 { x: X {} }.one(0)` calls `X`'s `two` with a `Wrap<u8>`, which calls
// `Y2<X>`'s `one` with it, which calls `two` with a `Wrap<Wrap<u8>>`, and
// so on: monomorphising it never ends. Neither `r1` nor `r2` has a cycle
// alone: `r1`'s `two` is on a type parameter, so it's whichever impl a
// caller's type has, and `r2`'s is what passes `K` back. So each package
// is checked as if any impl could be any package's: passing a bigger
// type to one is `r1`'s error, alone, and adding `r2` changes nothing
// (see docs/29-closed-and-sealed-traits.md).
use q::Tr1;

fn main() {
    r1::Y2 { x: r2::X {} }.one(0u8);
}
