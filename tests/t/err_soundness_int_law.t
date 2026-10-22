// The greatest value of an integer type is at least -1 minus its least,
// and no more is known: in a signed type, their sum is -1, so this
// branch runs, and `-` panics.
fn f<T: Int>() -> T {
    if core::num::greatest::<T>() + core::num::least::<T>() < 0 {
        core::num::least::<T>() - 1
    } else {
        0
    }
}

fn main() {
    print(f::<i8>());
}
// error: `-` may panic, but `f` doesn't say `with panic`
