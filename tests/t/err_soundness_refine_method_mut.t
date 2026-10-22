// A method taking `&mut self` borrows `d` mutably, without a `&mut` written.
trait Clear {
    fn clear(&mut self);
}

impl Clear for i64 {
    fn clear(&mut self) { *self = 0; }
}

fn f() -> i64 {
    let mut d: i64 = 1;
    d.clear();
    10 / d
}

fn main() with panic {
    print(f());
}
// error: `/` may panic, but `f` doesn't say `with panic`
