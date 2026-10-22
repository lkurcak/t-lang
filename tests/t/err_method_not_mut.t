#[derive(Discard)]
struct C { n: i64 }
impl C {
    fn inc(&mut self) { self.n += 1 }
}
fn main() {
    let c = C { n: 0 };
    c.inc();
}
// error: cannot borrow `c` as mutable, as `c` isn't declared `mut`
