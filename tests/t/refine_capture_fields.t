// What's known of a field a closure captures is known in its body: by
// `&`, what it was when the closure was made, which nothing can change
// while the closure lives; by value, the value it took.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn positive(v: { v: u32 | v > 0 }) -> u32 { v }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = P { x: 5, y: 0 };
    let c = || positive(p.x);
    p.y = 7;
    let d = move || positive(p.y);
    p.y = 0;
    out.line(&f"{c()} {d()} {p.y}") // expect: 5 7 0
}
