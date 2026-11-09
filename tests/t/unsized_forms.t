// More of what an `unsized` parameter may be part of: a `where` clause, a
// type argument written out or left to infer, impls for a struct holding
// one, a derived enum, a function's parameter type, an opaque type, and
// what `==` and `<` compare.
use core::fmt::{Debug, Display};

fn show<unsized D>(out: &mut std::io::Stdout, x: &D) -> Result<(), std::io::Error> where D: Display {
    out.line(x)
}

#[derive(Copy, Discard)]
struct Ref<'a, unsized T> { r: &'a T }

trait Show { fn show(&self, out: &mut std::io::Stdout) -> Result<(), std::io::Error>; }

impl<'a, unsized T: Debug> Show for Ref<'a, T> {
    fn show(&self, out: &mut std::io::Stdout) -> Result<(), std::io::Error> { out.line(&f"{self.r:?}") }
}

impl<'a, unsized T> Ref<'a, T> {
    fn get(&self) -> &'a T { self.r }
}

#[derive(Copy, Discard, Eq, Debug)]
enum E<'a, unsized T> { A(&'a T), B }

fn same<unsized D: Eq + Ord>(a: &D, b: &D) -> bool { a == b && !(a < b) }

fn apply<unsized D, F: Fn(&D) -> usize + Discard>(f: F, x: &D) -> usize { f(x) }

fn shown<'a, unsized D: Display>(x: &'a D) -> impl Display + Discard { x }

fn first<unsized I: Iterator>(it: &mut I) -> Option<I::Item> { it.next() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    show::<str>(&mut out, "turbofish")?; // expect: turbofish
    show::<_>(&mut out, "hole")?; // expect: hole
    let o: Option<Ref<'static, str>> = Some(Ref { r: "opt" });
    if let Some(r) = o {
        r.show(&mut out)?; // expect: "opt"
        out.line(r.get())?; // expect: opt
    }
    out.line(&same("a", "a"))?; // expect: true
    out.line(&same("a", "b"))?; // expect: false
    let xs = [1u8, 2];
    out.line(&same(xs.as_slice(), xs.as_slice()))?; // expect: true
    let a: E<'static, str> = E::A("s");
    let b: E<'static, str> = E::B;
    out.line(&f"{a:?} {b:?}")?; // expect: A("s") B
    out.line(&(a == a))?; // expect: true
    out.line(&(a == b))?; // expect: false
    out.line(&apply(|s: &str| s.len(), "four"))?; // expect: 4
    out.line(&shown("opaque"))?; // expect: opaque
    let mut it = xs.as_slice().iter();
    out.line(&f"{first(&mut it):?}")?; // expect: Some(1)
    Ok(())
}
