// An alias of a linear type is linear: it must be used up.
struct Handle { fd: i64 }
type H = Handle;

fn main() {
    let h: H = Handle { fd: 3 };
}
// warning: unused variable `h`: use it up, as its type, `Handle`, isn't `Discard`
// error: `h` is never used up, and its type, `Handle`, isn't `Discard`; to let it be thrown away, add `#[derive(Discard)]` to `Handle`
