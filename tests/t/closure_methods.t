// Closures in methods: capturing `self`, a `&mut`, changes what it points
// to through it, which needs it unique, not `mut`, as a reborrow does.
#[derive(Discard)]
struct Counter { n: int }
impl Counter {
    fn bump_by(&mut self, k: int) -> int with panic {
        let mut add = |d: int| self.n = self.n + d;
        add(k);
        add(k);
        self.n
    }
}
trait Twice {
    fn get(&self) -> int;
    fn twice(&self) -> int with panic {
        let g = || self.get();
        g() + g()
    }
}
impl Twice for Counter {
    fn get(&self) -> int { self.n }
}
#[derive(Discard)]
struct Wrap<F> { f: F }
impl<f, F: Fn(int) -> int with f + Discard> Wrap<F> {
    fn run(&self, x: int) -> int with f { (self.f)(x) }
}
fn main() with panic {
    let mut c = Counter { n: 1 };
    print(c.bump_by(2)); // expect: 5
    print(c.twice());    // expect: 10
    let k = 7;
    let w = Wrap { f: |x| x + k };
    print(w.run(1)); // expect: 8
}
