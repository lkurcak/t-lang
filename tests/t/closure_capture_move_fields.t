// A `move` closure takes the fields it names: a copy of one that's `Copy`,
// or the field itself, leaving the rest of the variable where it was.
#[derive(Discard)]
struct P { x: i64, name: alloc::string::String, tag: alloc::string::String }

fn take(s: alloc::string::String) -> usize { s.len() }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut p = P {
        x: 1,
        name: alloc::string::String::from("abc"),
        tag: alloc::string::String::from("de"),
    };
    // A copy of `p.x`: `p` is still whole.
    let get = move || p.x;
    p.x = 5;
    out.line(&f"{get()} {p.x}")?; // expect: 1 5
    // `p.name` moved in: `p.tag` and `p.x` stay usable.
    let len = move || take(p.name);
    out.line(&f"{p.tag.len()} {p.x}")?; // expect: 2 5
    out.line(&len())?; // expect: 3
    // A field read by a `move` closure is moved in, the rest kept.
    let show = move || p.tag.len();
    out.line(&f"{show()} {p.x}") // expect: 2 5
}
