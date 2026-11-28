// And where the trait's bound says what its calls may do, the impl's says
// it too: an open one would let it do what the trait's callers don't
// count on.
trait Each {
    fn each<F: FnMut(u8) with {} + Discard>(&self, f: F);
}

struct Three {}

impl Each for Three {
    fn each<G: FnMut(u8) + Discard>(&self, mut g: G) with G::e {
        g(1);
    }
}

fn main() {}
// error: method `each` doesn't match its declaration in trait `Each`: an `Fn` bound in this one says no `with`
