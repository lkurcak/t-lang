// Code that can never run: functions `main` doesn't reach, structs it never
// builds, enums it never mentions and variants it never constructs. A name
// starting with `_` says that's on purpose.
#[derive(Discard)]
struct Used { a: i64 }
#[derive(Discard)]
struct Unbuilt { a: i64 }
#[derive(Discard)]
struct _Kept { a: i64 }
#[derive(Discard)]
enum Color { Red, Green, _Blue }
#[derive(Discard)]
enum Unused { A, B }
#[derive(Discard)]
enum Inner { X }

// Only mentioned by a struct that's built, so not dead.
#[derive(Discard)]
struct Holder { i: Inner }

fn helper() -> i64 { 1 }

// They call each other, but nothing live calls either.
fn ping(n: i64) -> i64 with panic { if n == 0 { 0 } else { pong(n - 1) } }
fn pong(n: i64) -> i64 with panic { ping(n) }

// Named but never called still counts as used.
fn passed(x: i64) -> i64 { x }
fn apply(f: fn(i64) -> i64) -> i64 { f(2) }

fn _scratch() -> i64 { helper() }

// Takes an `Unbuilt`, which never exists, but `main` names it.
fn read(u: Unbuilt) -> i64 { u.a }

fn red(c: Color) -> bool {
    match c {
        Color::Red => true,
        _ => false,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let u = Used { a: helper() };
    out.line(&u.a)?;
    out.line(&apply(passed))?;
    let h = Holder { i: Inner::X };
    let Holder { i: Inner::X } = h;
    out.line(&red(Color::Red))?;
    let f = read;
    Result::Ok(())
}
// warning: struct `Unbuilt` is never constructed
// warning: variant `Color::Green` is never constructed
// warning: enum `Unused` is never used
// warning: function `ping` is never used
// warning: function `pong` is never used
// warning: unused variable `f`
// expect: 1
// expect: 2
// expect: true
