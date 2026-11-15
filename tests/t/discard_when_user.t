// A type of a program's own can say when its values may be thrown away:
// a bag of linear values, when it holds none. Its `discard` is checked
// knowing that, and runs where one is dropped (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

struct Bag { items: List<Fd>, label: i64 }

impl Bag {
    #[measure]
    fn len(&self) -> usize { self.items.len() }

    fn take(&mut self) -> { r: Option<Fd> | r.is_some() == (old(self.len()) > 0) }
    ensures
        self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0),
    { self.items.pop() }
}

impl Discard for { b: Bag | b.len() == 0 } {
    fn discard(self) {
        let Bag { items, label: _ } = self;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut b = Bag { items: List::new(), label: 7 };
    b.items.push(Fd { n: 1 });
    b.items.push(Fd { n: 2 });
    let mut order = 0;
    while let Some(fd) = b.take() {
        let Fd { n } = fd;
        order = order * 10 + n;
    }
    out.line(&order)?; // expect: 21
    let e = Bag { items: List::new(), label: 8 };
    out.line(&e.label)?; // expect: 8
    Ok(())
}
