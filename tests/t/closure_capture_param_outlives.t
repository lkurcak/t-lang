// A closure changing a field through a parameter borrows the field, for as
// long as what the parameter points to is lent, not just while the
// parameter is in scope: so it can be given back (docs/77-method-captures.md).
#[derive(Discard)]
struct S { total: u32, n: u32 }
fn adder(r: &mut S) -> impl FnMut(u32) with panic + Discard { |x: u32| r.total += x }
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut s = S { total: 1, n: 0 };
    {
        let mut add = adder(&mut s);
        add(2);
        add(3);
    }
    out.line(&f"{s.total} {s.n}") // expect: 6 0
}
