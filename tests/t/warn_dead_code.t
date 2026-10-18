// Code that can never run: functions `main` doesn't reach, structs it never
// builds, enums it never mentions and variants it never constructs. A name
// starting with `_` says that's on purpose.
#[derive(Discard)]
struct Used { a: int }
#[derive(Discard)]
struct Unbuilt { a: int }
#[derive(Discard)]
struct _Kept { a: int }
#[derive(Discard)]
enum Color { Red, Green, _Blue }
#[derive(Discard)]
enum Unused { A, B }
#[derive(Discard)]
enum Inner { X }

// Only mentioned by a struct that's built, so not dead.
#[derive(Discard)]
struct Holder { i: Inner }

fn helper() -> int { 1 }

// They call each other, but nothing live calls either.
fn ping(n: int) -> int { if n == 0 { 0 } else { pong(n - 1) } }
fn pong(n: int) -> int { ping(n) }

// Named but never called still counts as used.
fn passed(x: int) -> int { x }
fn apply(f: fn(int) -> int) -> int { f(2) }

fn _scratch() -> int { helper() }

// Takes an `Unbuilt`, which never exists, but `main` names it.
fn read(u: Unbuilt) -> int { u.a }

fn red(c: Color) -> bool {
    match c {
        Color::Red => true,
        _ => false,
    }
}

fn main() {
    let u = Used { a: helper() };
    print(u.a);
    print(apply(passed));
    let h = Holder { i: Inner::X };
    let Holder { i: Inner::X } = h;
    print_bool(red(Color::Red));
    let f = read;
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
