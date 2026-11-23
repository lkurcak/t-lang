// What a parameter points to is known as it was at the call only so many
// references deep (four); below that, `old(..)` is unknown, not what's
// there when it's read (a hole since field paths, 0.71.0: it was the
// value written since). See docs/30-postconditions.md.
fn f(a: &mut &mut &mut &mut &mut u8) with {} ensures *****a == old(*****a) {
    *****a = 3;
}

fn main() with {} {
    let mut x: u8 = 1;
    let mut p1 = &mut x;
    let mut p2 = &mut p1;
    let mut p3 = &mut p2;
    let mut p4 = &mut p3;
    f(&mut p4);
}
// error: can't prove `*****a == old(*****a)`
