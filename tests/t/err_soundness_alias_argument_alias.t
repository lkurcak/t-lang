// An alias given a refinement alias: each element of `Pair<Square>` is a
// square.
type Square = { s: u8 | s < 64 };
type Pair<T> = (T, T);

fn g(_p: Pair<Square>) {}

fn main() {
    g((1, 64));
}
// error: can't prove `s < 64`
