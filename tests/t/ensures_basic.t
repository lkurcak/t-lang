// `ensures` says what a function leaves behind a `&mut` parameter: there,
// the parameter's name means its value when the function returns, and
// `old(..)` its value at the call. The caller learns it of what it lent.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    /// Counts one more, below `limit`.
    fn bump(&mut self, _limit: { l: usize | l > self.get() + 1 })
    ensures
        self.get() == old(self.get()) + 1,
    {
        self.n = self.n + 1;
    }

    /// Leaves it at most `limit`, which only `ensures` names: that's a use.
    fn clear(&mut self, limit: usize) ensures self.get() <= limit {
        self.n = 0;
    }

    /// Leaves it as it was.
    fn look(&mut self) -> usize ensures self.get() == old(self.get()) { self.n }
}

fn below(n: { n: usize | n < 3 }) -> usize { n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Counter { n: 0 };
    c.bump(5);
    c.bump(5);
    c.look();
    out.line(&below(c.get()))?; // expect: 2
    c.clear(1);
    out.line(&below(c.get()))?; // expect: 0
    Result::Ok(())
}
