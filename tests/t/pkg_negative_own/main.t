// What isn't true of this program's own type is known: only this program,
// or core, which it can see, could make `Bag` an iterator. So `q`'s blanket
// impl isn't for `Bag`, and `Bag` can have its own.
use q::Size;

#[derive(Discard)]
struct Bag { n: i64 }

impl Size for Bag {
    fn size(&self) -> i64 { self.n }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(Bag { n: 3 }).size())?; // expect: 3
    out.line(&(0..5i64).size()) // expect: 0
}
