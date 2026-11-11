// The refinement of `R`'s argument is of the `Err`: an `Err(5)` doesn't
// have it. (Up to 0.80.0 it was of the `Ok`, which this would pass.)
type R<T> = Result<u8, T>;

fn f(_x: R<{ v: u8 | v < 3 }>) {}

fn main() {
    f(Err(5));
}
// error: can't prove `v < 3`
