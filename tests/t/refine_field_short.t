// A `&mut` lent to a call only while it runs: afterwards, what's known of
// the place is what's read from it again.
#[derive(Copy, Discard)]
struct Counter { n: { v: i64 | v >= 0 }, step: { s: i64 | s > 0 } }

impl Counter {
    fn reset(&mut self) { self.n = 0; }
}

fn per_step(c: &Counter, total: i64) -> i64 { total / c.step }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Counter { n: 5, step: 2 };
    c.reset();
    c.n = c.step;
    out.line(&per_step(&c, 10))?; // expect: 5
    out.line(&c.n)?; // expect: 2
    Result::Ok(())
}
