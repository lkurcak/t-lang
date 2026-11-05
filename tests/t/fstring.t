// `f".."` is a value that's `Display`: its text, with what each hole names
// written in its place, `{x}` by `Display` and `{x:?}` by `Debug`. A hole
// names a variable or a field of one; `{{` and `}}` are braces. It needs
// no import, and goes wherever a `Display` does.
use alloc::string::String;

mod shapes {
    #[derive(Debug, Discard)]
    pub struct Point { pub x: i32, pub y: i32 }
}

use shapes::Point;

#[derive(Discard)]
struct Greeter<'a> { name: &'a str, p: Point }

impl<'a> Greeter<'a> {
    fn greet(&self) -> String with alloc { String::of(&f"hi {self.name}, at {self.p:?}") }
}

// A generic function taking any `Display`, as `f".."` is.
fn twice<D: core::fmt::Display>(d: &D) -> String with alloc {
    let mut s = String::new();
    s.push_str(String::of(d).as_str());
    s.push_str(String::of(d).as_str());
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let name = "T";
    let p = Point { x: 1, y: -2 };
    out.line(&f"hello {name}, p = {p:?}, x = {p.x}, y = {p.y:?}")?; // expect: hello T, p = Point { x: 1, y: -2 }, x = 1, y = -2
    out.line(&f"{{braces}} \"quotes\" \u{e9}")?; // expect: {braces} "quotes" é
    out.line(&f"{name}{name:?}")?; // expect: T"T"
    out.put(&f"")?;
    out.line(&f"{p.x}")?; // expect: 1
    // A value, kept and used more than once.
    let line = f"<{p.y}>";
    out.line(&line)?; // expect: <-2>
    out.line(&twice(&line))?; // expect: <-2><-2>
    // One inside another, as a hole.
    out.line(&f"[{line}]")?; // expect: [<-2>]
    let g = Greeter { name: "ann", p: Point { x: 3, y: 4 } };
    out.line(&g.greet())?; // expect: hi ann, at Point { x: 3, y: 4 }
    let s = String::of(&f"{g.p.x}-{g.p.y}");
    out.line(&f"{s} {s:?}")?; // expect: 3-4 "3-4"
    Ok(())
}

#[test]
fn panics_with_one() {
    let n = 3u8;
    core::panic::panic_with(&f"n is {n:?}");
}

// test: panics_with_one ... FAILED: n is 3
