// A path to a trait's method may have `Self` unsized, as `s.fmt(w)` may;
// but a value of it is still none to pass.
trait Take { fn take(self) -> u8; }

fn f<unsized D: Take + Copy>(a: &D) -> u8 {
    Take::take(*a)
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
