// A closure that changes a field through a function's reference parameter
// captures that field, not the parameter (docs/77-method-captures.md):
// `self.total` while `self.items` is borrowed, as in Rust.
use alloc::list::List;

#[derive(Discard)]
struct S { items: List<u32>, total: u32, count: u32 }

impl S {
    fn sum(&mut self) with panic {
        let mut add = |x: u32| self.total = self.total + x;
        for x in &self.items {
            add(*x);
        }
    }

    // Two closures, each changing a field of its own; the function
    // changes a third.
    fn tally(&mut self) with {panic, alloc} {
        let mut add = |x: u32| self.total = self.total + x;
        let mut more = || self.count = self.count + 1;
        add(5);
        more();
        self.items.push(7);
        add(1);
        more();
    }
}

#[derive(Discard)]
struct In { a: u32, b: u32 }

#[derive(Discard)]
struct Out { inner: In, c: u32 }

// Through fields of what a parameter points to, and back out after.
fn deep(r: &mut Out) with panic {
    let mut set = || r.inner.a = r.inner.a + 10;
    r.inner.b = 2;
    r.c = 3;
    set();
    r.inner.a = r.inner.a * 2;
}

// Changing what a field that's a reference points to: the closure
// captures that field uniquely, through the parameter.
#[derive(Discard)]
struct Cursor<'a> { at: &'a mut u32, moved: u32 }

fn step(c: &mut Cursor) with panic {
    let mut go = || *c.at = *c.at + 1;
    c.moved = c.moved + 1;
    go();
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut out = os.stdout;
    let mut s = S { items: List::new(), total: 0, count: 0 };
    s.items.push(1);
    s.items.push(2);
    s.items.push(3);
    s.sum();
    out.line(&f"{s.total}")?; // expect: 6
    s.tally();
    out.line(&f"{s.total} {s.count} {s.items.len()}")?; // expect: 12 2 4
    let mut o = Out { inner: In { a: 1, b: 0 }, c: 0 };
    deep(&mut o);
    out.line(&f"{o.inner.a} {o.inner.b} {o.c}")?; // expect: 22 2 3
    let mut n: u32 = 7;
    let mut c = Cursor { at: &mut n, moved: 0 };
    step(&mut c);
    let moved = c.moved;
    out.line(&f"{n} {moved}") // expect: 8 1
}
