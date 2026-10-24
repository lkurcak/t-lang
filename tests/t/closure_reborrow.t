// Changing through a captured `&mut`, by a method, a call or a reborrow,
// needs it unique, not `mut`, as anywhere else.
#[derive(Discard)]
struct V { n: i64 }
impl V {
    fn push(&mut self, k: i64) with panic {
        self.n = self.n + k;
    }
}
fn bump(v: &mut V) with panic {
    v.push(1);
}
fn go<effect e, F: FnMut() with e>(f: &mut F) with e {
    let mut g = || f();
    g();
}
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut v = V { n: 0 };
    let r = &mut v;
    let mut a = || r.push(2);
    a();
    let r2 = &mut v;
    let mut b = || bump(r2);
    b();
    let mut t = || v.push(1);
    go(&mut t);
    out.line(&v.n)?; // expect: 4
    Result::Ok(())
}
