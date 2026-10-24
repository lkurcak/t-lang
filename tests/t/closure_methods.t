// Closures in methods: capturing `self`, a `&mut`, changes what it points
// to through it, which needs it unique, not `mut`, as a reborrow does.
#[derive(Discard)]
struct Counter { n: i64 }
impl Counter {
    fn bump_by(&mut self, k: i64) -> i64 with panic {
        let mut add = |d: i64| self.n = self.n + d;
        add(k);
        add(k);
        self.n
    }
}
trait Twice {
    fn get(&self) -> i64;
    fn twice(&self) -> i64 with panic {
        let g = || self.get();
        g() + g()
    }
}
impl Twice for Counter {
    fn get(&self) -> i64 { self.n }
}
#[derive(Discard)]
struct Wrap<F> { f: F }
impl<effect f, F: Fn(i64) -> i64 with f + Discard> Wrap<F> {
    fn run(&self, x: i64) -> i64 with f { (self.f)(x) }
}
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut c = Counter { n: 1 };
    out.line(&c.bump_by(2))?; // expect: 5
    out.line(&c.twice())?;    // expect: 10
    let k = 7;
    let w = Wrap { f: |x| x + k };
    out.line(&w.run(1))?; // expect: 8
    Result::Ok(())
}
