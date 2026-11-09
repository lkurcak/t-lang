// An `unsized` type parameter stands for any type, `str` and slices too.
// It's only ever behind a reference, so nothing generic needs its size.
use core::fmt::{Debug, Display};

/// Any `Display`, `str` too, without a `&&str`.
fn twice<unsized D: Display>(out: &mut std::io::Stdout, x: &D) -> Result<(), std::io::Error> {
    out.put(x)?;
    out.line(x)
}

/// Passing one through: the same reference back.
fn first<'a, unsized T>(a: &'a T, _b: &'a T) -> &'a T { a }

/// Through a mutable reference: at `[i64]`, a `&mut [i64]`.
fn again<'a, unsized T>(x: &'a mut T) -> &'a mut T { x }

/// A struct holding a reference to one: `Ref<'a, str>` is a `&'a str`.
#[derive(Copy, Discard, Debug)]
struct Ref<'a, unsized T> { r: &'a T }

/// An alias with one.
type Shared<'a, unsized T> = &'a T;

fn pick<'a, unsized T>(r: Ref<'a, T>) -> Shared<'a, T> { r.r }

/// A trait's method with one, and an impl's, of the same kind.
trait Show {
    fn show<unsized D: Display>(&self, out: &mut std::io::Stdout, x: &D) -> Result<(), std::io::Error>;
}

#[derive(Discard)]
struct Brackets {}

impl Show for Brackets {
    fn show<unsized D: Display>(&self, out: &mut std::io::Stdout, x: &D) -> Result<(), std::io::Error> {
        out.put("[")?;
        out.put(x)?;
        out.line("]")
    }
}

/// An impl for references to any `Display`, `&str` among them.
trait Loud {
    fn loud(&self, out: &mut std::io::Stdout) -> Result<(), std::io::Error>;
}

impl<'a, unsized T: Display> Loud for &'a T {
    fn loud(&self, out: &mut std::io::Stdout) -> Result<(), std::io::Error> {
        out.put(*self)?;
        out.line("!")
    }
}

/// A path to a trait's method, with `Self` the parameter.
fn path<unsized D: Debug>(out: &mut std::io::Stdout, x: &D) -> Result<(), std::io::Error> {
    Debug::fmt(x, out)?;
    out.line("")
}

/// A closure returning one, captured.
fn later<'a, unsized D: Display>(x: &'a D) -> impl Fn() -> &'a D + Discard { move || x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line("hello")?; // expect: hello
    twice(&mut out, "ab")?; // expect: abab
    twice(&mut out, &7u8)?; // expect: 77
    let s = "left";
    out.line(first(s, "right"))?; // expect: left
    let mut a = [3i64, 1, 2];
    let m = again(a.as_mut_slice());
    m.swap(0, 1);
    out.line(&f"{a:?}")?; // expect: [1, 3, 2]
    let r = Ref { r: "in a struct" };
    let r2 = r;
    out.line(pick(r))?; // expect: in a struct
    out.line(&f"{r2:?}")?; // expect: Ref { r: "in a struct" }
    let b = [1u8, 2];
    let xs: &[u8] = b.as_slice();
    let rs = Ref { r: xs };
    out.line(&f"{rs:?}")?; // expect: Ref { r: [1, 2] }
    Brackets {}.show(&mut out, "str")?; // expect: [str]
    Brackets {}.show(&mut out, &5u8)?; // expect: [5]
    Loud::loud(&s, &mut out)?; // expect: left!
    path(&mut out, "q")?; // expect: "q"
    path(&mut out, xs)?; // expect: [1, 2]
    let f = later("closure");
    out.line(f())?; // expect: closure
    Ok(())
}
