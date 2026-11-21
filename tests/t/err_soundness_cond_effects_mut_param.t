// A condition is of the parameters as they're given: `x` is changed before
// the panic, so `x == 0` at the call doesn't say when it panics.
fn zero(mut x: u8) -> u8 with panic if x == 0 {
    x = 0;
    if x == 0 {
        panic("zero");
    }
    x
}

fn main() with {} {
    let _v = zero(1);
}
// error: `panic` may panic when `x == 0` doesn't hold, but `zero` says `with panic if x == 0`
