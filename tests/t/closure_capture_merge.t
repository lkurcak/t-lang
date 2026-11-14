// Where a closure names a place and a place in it, it captures the
// outer one, the most either needs: here `&mut p.x`, which `p.x.a = 1`
// changes and `&p.x` reads, leaving `p.y` to the function.
#[derive(Discard)]
struct Inner { a: u64, b: u64 }

#[derive(Discard)]
struct P { x: Inner, y: u64 }

fn sum(i: &Inner) -> u64 with panic { i.a + i.b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut p = P { x: Inner { a: 0, b: 2 }, y: 3 };
    let mut c = || {
        p.x.a = 1;
        sum(&p.x)
    };
    p.y = 4;
    let v = c();
    out.line(&f"{v} {p.x.a} {p.y}") // expect: 3 1 4
}
