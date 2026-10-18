// Two local types of the same name are different types, and the message
// says so, since they look the same in it.
fn f() -> int {
    #[derive(Discard)]
    struct P { x: int }
    fn g(p: P) -> int { p.x }
    fn h() -> int {
        #[derive(Discard)]
        struct P { x: int }
        g(P { x: 1 })
    }
    h()
}

fn main() {
    print(f());
}
// error: expected `P`, found `P`; these are two different types called `P`, one declared in `h` and one in `f`
