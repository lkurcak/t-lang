// A struct a call returns: nothing's known of its fields but its
// refinements.
#[derive(Copy, Discard)]
struct P { x: int }

fn make() -> P { P { x: 0 } }

fn f() -> int { 10 / make().x }

fn main() with panic {
    print(f());
}
// error: `/` may panic, but `f` doesn't say `with panic`
