// A field a refinement is about, lent to a call that ensures something:
// reading the field's new value for what the call ensures used to assume
// the struct's refinements of it, so proving them again after the call
// proved them from themselves. `touch` breaks `bag`'s, and the
// interpreter caught it (t 0.43.0 to 0.74.0).
#[derive(Copy, Discard)]
struct Bag {
    extra: usize,
}

impl Bag {
    #[measure]
    fn more(&self) -> usize { self.extra }

    fn touch(&mut self) ensures self.extra >= 0 { self.extra = 7; }
}

#[derive(Copy, Discard)]
struct Holder {
    bag: { b: Bag | b.more() == 3 },
}

fn first(h: &mut Holder) { h.bag.touch(); }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut h = Holder { bag: Bag { extra: 3 } };
    first(&mut h);
    let mut out = os.stdout;
    out.line(&h.bag.more())?;
    Ok(())
}
// error: can't prove `b.more() == 3` for field `bag` of `Holder`
