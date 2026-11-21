// Beside an associated effect, or an effect parameter, a condition can't
// say when the call has an effect: `I::e` is `panic` at this call, and
// ruling `x > 3` out there wouldn't rule out `Boom`'s panic. So there's
// none.
#[derive(Discard)]
struct Boom {}

impl Iterator for Boom {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic { panic("boom") }
}

fn step<I: Iterator>(it: &mut I, x: u8) -> Option<I::Item> with {I::e, panic if x > 3} {
    if x > 3 {
        panic("big");
    }
    it.next()
}

fn main() with {} {
    let mut b = Boom {};
    let _v = step(&mut b, 0);
}
// error: an effect with a condition can't be beside an effect parameter or an associated effect, which may be it at a call
