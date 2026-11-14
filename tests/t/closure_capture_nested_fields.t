// A closure inside another captures what it names from the variables
// around both, through the outer one, which captures at least that.
#[derive(Discard)]
struct P { x: i64, y: (i64, i64) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut p = P { x: 1, y: (2, 3) };
    let r = &mut p.x;
    let outer = || {
        let inner = || p.y.0 + p.y.1;
        inner() * 2
    };
    *r = outer();
    out.line(&p.x)?; // expect: 10
    let mut bump = || {
        let mut inner = || p.y.1 = p.y.1 + 1;
        inner();
        inner();
    };
    let x = &p.x;
    bump();
    out.line(&f"{x} {p.y.1}") // expect: 10 5
}
