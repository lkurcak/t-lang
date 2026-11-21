// An argument is its variable's value where it's written, not after a
// later argument has changed it: `a` is 5 here and `b` 7, so `pair`'s
// condition isn't ruled out (docs/91-evaluation-order.md).
fn pair(a: u8, b: u8) -> u8 with panic if a != b {
    if a != b {
        panic("not the same");
    }
    a
}

fn safe(c: &mut u8) -> u8 with {} {
    match *c == 5 {
        true => pair(*c, { *c = 7; 7 }),
        false => 0,
    }
}

fn main() with {} {
    let mut c = 5u8;
    let _v = safe(&mut c);
}
// error: `pair` may panic, but `safe` doesn't say `with panic`
