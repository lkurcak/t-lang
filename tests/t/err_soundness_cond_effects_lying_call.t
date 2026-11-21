// A call's condition must follow from the caller's: `inner` may panic when
// `b > 3`, which `outer`'s `a > 3` doesn't rule out.
fn inner(b: u8) -> u8 with panic if b > 3 {
    if b > 3 {
        panic("big");
    }
    b
}

fn outer(a: u8, b: u8) -> u8 with panic if a > 3 { inner(b) + a * 0 }

fn main() with {} {
    let _v = outer(0, 9);
}
// error: `inner` may panic when `a > 3` doesn't hold, but `outer` says `with panic if a > 3`
