// An impl's method takes what the trait's does: where the trait's bound
// leaves what its calls do open, so must the impl's.
trait Each {
    fn each<F: FnMut(u8) + Discard>(&self, f: F) with F::e;
}

struct Three {}

impl Each for Three {
    fn each<G: FnMut(u8) with {} + Discard>(&self, mut g: G) { g(1); }
}

fn main() {}
// error: method `each` doesn't match its declaration in trait `Each`: an `Fn` bound in the trait's says no `with`
