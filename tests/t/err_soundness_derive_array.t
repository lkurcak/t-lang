// A derived `Discard` of a struct holding `[T; 1]` needs `T: Discard`, as
// dropping it drops the `T`.
#[derive(Discard)]
struct One<T> { a: [T; 1] }

struct Fd { n: i64 }

fn main() {
    let _o = One { a: [Fd { n: 1 }] };
}
// error: `_o` is never used up, and its type, `One<Fd>`, isn't `Discard`
