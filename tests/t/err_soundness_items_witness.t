// What every value an iterator gives has is said of each value on its own:
// two values it gives at two times aren't one, though what's said of each
// names the state it was given from.
#[derive(Discard)]
struct Count { at: u32 }

impl Count {
    fn new(at: u32) -> { c: Count | c.at() == at } { Count { at } }

    #[measure]
    fn at(&self) -> u32 { self.at }
}

impl Iterator for Count {
    type Item = u32;
    effect e = {};

    fn next(&mut self) -> Option<{ v: u32 | v == old(self.at()) }>
    ensures
        self.at() >= old(self.at()),
    {
        if self.at < 10 {
            let v = self.at;
            self.at = self.at + 1;
            Some(v)
        } else {
            None
        }
    }
}

fn main() with panic {
    let mut it = Count::new(0).skip(0);
    let x = it.next().unwrap();
    let y = it.next().unwrap();
    let c: { v: u32 | v == x } = y;
    let _ = c;
}
// error: can't prove `v == x` for `c`
