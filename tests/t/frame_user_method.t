// A method whose result borrows `self` can say what it leaves behind as
// long as that can only be changed through the result by writing values
// of a type parameter, which the measures it names can't see (docs/59).
use alloc::list::List;

struct Bag<T> {
    items: List<T>,
    label: u8,
}

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    /// The `i`th item, to change; the size and label stay.
    fn slot(&mut self, i: { i: usize | i < self.size() }) -> &mut T
    ensures
        self.size() == old(self.size()) && self.label == old(self.label),
    {
        &mut self.items[i]
    }

    /// All of them: a slice, which writes only `T`s too.
    fn all(&mut self) -> { s: &mut [T] | s.len() == old(self.size()) }
    ensures
        self.size() == old(self.size()),
    {
        self.items.as_mut_slice()
    }
}

/// Two type parameters: the result writes only `A`s, so a measure of the
/// `B` part is kept, whatever its bounds.
struct Pair<A, B> {
    a: A,
    b: B,
}

impl<A, B: Int> Pair<A, B> {
    #[measure]
    fn second(&self) -> B { self.b }

    fn first_mut(&mut self) -> &mut A ensures self.second() == old(self.second()) { &mut self.a }
}

/// A struct whose field is refined by another's measure can lend it to
/// such a method: what it ensures proves the refinement again.
struct Table {
    width: usize,
    cells: { c: List<u32> | c.len() == width },
}

impl Table {
    fn put(&mut self, i: { i: usize | i < self.width }, v: u32) { self.cells[i] = v; }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = Bag { items: List::filled(2, 7u32), label: 1 };
    if b.size() == 2 && b.label == 1 {
        *b.slot(0) = 1;
        *b.slot(1) += 1;
        let s = b.all();
        s[1] += 1;
        let l: { l: u8 | l == 1 } = b.label;
        out.line(&l)?;            // expect: 1
        out.line(b.slot(1))?;     // expect: 9
    }
    let mut p = Pair { a: 1u8, b: 40u32 };
    if p.second() == 40 {
        *p.first_mut() = 2;
        let n: { n: u32 | n == 40 } = p.second();
        out.line(&n)?;            // expect: 40
    }
    let mut t = Table { width: 2, cells: List::filled(2, 0) };
    t.put(1, 5);
    out.line(&t.cells[1])?;       // expect: 5
    Ok(())
}
