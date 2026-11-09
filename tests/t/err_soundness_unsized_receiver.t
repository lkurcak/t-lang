// A method taking `self` by value, called on a reference to a `Copy`
// value, copies it: not a `D`, which may have no size.
trait Take { fn take(self) -> u8; }

fn f<unsized D: Take + Copy>(a: &D) -> u8 {
    a.take()
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
