// Through a parameter too: what's declared behind `r` is only its type,
// whose refinements relate `lo` and `hi`, not `log`.
#[derive(Discard)]
struct R { lo: u32, hi: { h: u32 | h >= lo }, log: u32 }

fn f(r: &mut R) with panic {
    let mut note = || r.log = r.log + 1;
    if r.hi < 1000 {
        r.hi = r.hi + 1;
    }
    note();
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut r = R { lo: 0, hi: 1, log: 0 };
    f(&mut r);
    out.line(&f"{r.hi} {r.log}") // expect: 2 1
}
