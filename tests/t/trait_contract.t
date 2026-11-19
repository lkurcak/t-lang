// A trait's method can say what it needs and what it gives, which every
// impl's has too: a call through a bound proves the one and learns the
// other (docs/86-trait-contracts.md).

trait Table {
    /// The value at `i`, which is less than `n`, and less than 100.
    fn at(&self, i: { i: usize | i < 4 }) -> { v: u32 | v < 100 };

    /// Puts `v` at `i`, giving back what was there, which is less than 100
    /// too.
    fn put(&mut self, i: { i: usize | i < 4 }, v: { v: u32 | v < 100 }) -> { w: u32 | w < 100 };
}

#[derive(Discard)]
struct Four { cells: [{ v: u32 | v < 100 }; 4] }

// It needn't say it again: its `at` may index with `i`, and must give back
// less than 100.
impl Table for Four {
    fn at(&self, i: usize) -> u32 { self.cells[i] }

    fn put(&mut self, i: usize, v: u32) -> u32 {
        let w = self.cells[i];
        self.cells[i] = v;
        w
    }
}

// Generic code knows what the trait says, and nothing more.
fn sum<T: Table>(t: &T) -> { s: u32 | s < 400 } {
    t.at(0) + t.at(1) + t.at(2) + t.at(3)
}

fn bump<T: Table>(t: &mut T, i: { i: usize | i < 4 }) -> u32 {
    let old = t.put(i, 99);
    old + 1
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut f = Four { cells: [1, 2, 3, 4] };
    out.line(&sum(&f))?;      // expect: 10
    out.line(&bump(&mut f, 2))?; // expect: 4
    out.line(&f.at(2))?;      // expect: 99
    // A call of `Four`'s own knows it too.
    let v: { v: u32 | v < 100 } = f.at(3);
    out.line(&v)?;            // expect: 4
    Ok(())
}
