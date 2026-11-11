// `Pair`'s parameter is both elements, so the argument's refinement is of
// both.
type Pair<T> = (T, T);

fn g(_p: Pair<{ v: u8 | v < 3 }>) {}

fn main() {
    g((1, 5));
}
// error: can't prove `v < 3`
