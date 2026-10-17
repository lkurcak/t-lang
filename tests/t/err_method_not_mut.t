struct C { n: int }
impl C {
    fn inc(&mut self) { self.n += 1 }
}
fn main() {
    let c = C { n: 0 };
    c.inc();
}
// error: cannot borrow `c` as mutable, as `c` isn't declared `mut`
