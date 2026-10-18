// Changing through a captured `&mut`, by a method, a call or a reborrow,
// needs it unique, not `mut`, as anywhere else.
#[derive(Discard)]
struct V { n: int }
impl V {
    fn push(&mut self, k: int) {
        self.n = self.n + k;
    }
}
fn bump(v: &mut V) {
    v.push(1);
}
fn go<F: FnMut()>(f: &mut F) {
    let mut g = || f();
    g();
}
fn main() {
    let mut v = V { n: 0 };
    let r = &mut v;
    let mut a = || r.push(2);
    a();
    let r2 = &mut v;
    let mut b = || bump(r2);
    b();
    let mut t = || v.push(1);
    go(&mut t);
    print(v.n); // expect: 4
}
