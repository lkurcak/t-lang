#[derive(Discard)]
struct P { n: int }
impl P {
    fn take(self) -> int { self.n }
}
fn main() {
    let p = P { n: 1 };
    let r = &p;
    print(r.take());
}
// error: cannot move out of `*r`, which is behind a reference
