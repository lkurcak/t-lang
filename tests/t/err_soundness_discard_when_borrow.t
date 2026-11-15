// A refined impl's `discard` runs where its value is dropped, and may
// read what the value borrows: so the value is used there, and what it
// borrows must still be alive (lemma W6 of docs/80).
use alloc::list::List;

struct Peek<'a> { r: &'a List<i64>, n: i64 }

impl<'a> Peek<'a> {
    #[measure]
    fn n(&self) -> i64 { self.n }
}

impl<'a> Discard for { p: Peek<'a> | p.n() == 0 } {
    fn discard(self) {
        let Peek { r, n: _ } = self;
        let _ = r.len();
    }
}

fn main() with alloc {
    let p;
    {
        let mut l: List<i64> = List::new();
        l.push(1);
        p = Peek { r: &l, n: 0 };
    }
}
// error: `l` doesn't live long enough: it goes out of scope here while still borrowed
