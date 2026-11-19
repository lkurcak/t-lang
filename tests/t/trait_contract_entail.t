// An impl may ask less of a parameter than its trait, if what the trait
// asks is enough for it, and may promise more: a call known to be of it
// gets both (docs/86-trait-contracts.md).
trait Pick {
    fn pick(&self, i: { i: usize | i < 3 }) -> { v: u8 | v < 10 };
}

#[derive(Discard)]
struct Three { a: [{ v: u8 | v < 10 }; 3] }

impl Pick for Three {
    // Anything up to 3 is fine here, and 3 itself gives 0.
    fn pick(&self, i: { i: usize | i <= 3 }) -> { v: u8 | v < 10 && (i < 3 || v == 0) } {
        if i < 3 { self.a[i] } else { 0 }
    }
}

#[derive(Discard)]
struct Zero {}

impl Pick for Zero {
    // Anything at all.
    fn pick(&self, _i: { i: usize | true }) -> { v: u8 | v == 0 } { 0 }
}

fn through<P: Pick>(p: &P) -> { v: u8 | v < 10 } { p.pick(2) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = Three { a: [4, 5, 6] };
    let z: { v: u8 | v == 0 } = t.pick(3);
    out.line(&z)?;              // expect: 0
    out.line(&through(&t))?;    // expect: 6
    let w: { v: u8 | v == 0 } = Zero {}.pick(1000);
    out.line(&w)?;              // expect: 0
    out.line(&through(&Zero {}))?; // expect: 0
    Ok(())
}
