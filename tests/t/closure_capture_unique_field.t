// A closure that changes what a `&mut` field points to captures the field
// uniquely: `p` needn't be `mut`, and its other fields stay usable.
#[derive(Discard)]
struct P<'a> { r: &'a mut u32, n: u32 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x = 1;
    {
        let p = P { r: &mut x, n: 2 };
        let mut c = || *p.r = 5;
        let n = p.n;
        c();
        out.line(&n)?; // expect: 2
    }
    out.line(&x) // expect: 5
}
