// A function that isn't exported and doesn't say `with` has the effects
// its body has: here `half` can panic, `boxed` can allocate, and `main`
// can do both, without any of them saying so (see docs/24-visibility.md).
use alloc::boxed::Box;

fn half(n: i64) -> i64 { n / 2 + 1 }

fn boxed(n: i64) -> Box<i64> { Box::new(half(n)) }

fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(x) }

mod m {
    #[derive(Discard)]
    pub struct Counter { n: i64 }

    impl Counter {
        pub fn new() -> Counter { Counter { n: 0 } }

        pub fn bump(&mut self) -> i64 {
            self.n += 1;
            self.n
        }

        pub fn done(self) -> i64 {
            let Counter { n } = self;
            n
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    // A nested function's effects are inferred too.
    fn twice(n: i64) -> i64 { n * 2 }
    out.line(&(*boxed(9)))?;            // expect: 5
    out.line(&apply(half, 4))?;         // expect: 3
    out.line(&apply(|x| twice(x), 4))?; // expect: 8
    let mut c = m::Counter::new();
    let _ = c.bump();
    out.line(&c.bump())?;               // expect: 2
    out.line(&c.done())?;               // expect: 2
    Result::Ok(())
}
