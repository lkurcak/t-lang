// And in a loop's invariant, where `old(..)` is the same entry value
// (docs/92-where-a-bound-holds.md).
fn f(a: &mut &mut &mut &mut &mut u8) with {} {
    *****a = 3;
    let mut i: u8 = 0;
    while i < 1 invariant *****a == old(*****a) {
        i = i + 1;
    }
}

fn main() with {} {
    let mut x: u8 = 1;
    let mut p1 = &mut x;
    let mut p2 = &mut p1;
    let mut p3 = &mut p2;
    let mut p4 = &mut p3;
    f(&mut p4);
}
// error: can't prove `*****a == old(*****a)` for the loop's invariant, as the loop starts
// error: can't prove `*****a == old(*****a)` for the loop's invariant, at the end of its body
