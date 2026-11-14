// A closure captures the places its body names, not whole variables:
// `|| p.x` borrows `p.x`, so the rest of `p` can be borrowed, changed or
// moved while it lives (docs/76-widths-and-captures.md).
#[derive(Discard)]
struct P { x: i64, y: i64 }

#[derive(Discard)]
struct Machine { name: alloc::string::String, clock: u64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut out = os.stdout;
    // `p.y` changes while a closure reads `p.x`.
    let mut p = P { x: 1, y: 2 };
    let fx = || p.x;
    p.y = 20;
    out.line(&(fx() + p.y))?; // expect: 21
    // Two closures change two fields at once.
    let mut ix = || p.x = p.x + 1;
    let mut iy = || p.y = p.y + 1;
    ix();
    iy();
    ix();
    out.line(&f"{p.x} {p.y}")?; // expect: 3 21
    // A field `&mut` borrowed while a closure reads another.
    let y = &mut p.y;
    let gx = || p.x * 10;
    *y = gx();
    out.line(&p.y)?; // expect: 30
    // The field it names, of a variable partly moved: t-life's `os.clock`.
    let o = Machine { name: alloc::string::String::from("t"), clock: 7 };
    let name = o.name;
    let tick = || o.clock + 1;
    out.line(&f"{name} {tick()}") // expect: t 8
}
