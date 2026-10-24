// What an impl needs, it needs where it's used, through a generic function
// too.
struct Fd { n: i64 }

trait Tr {
    fn go(self) -> i64;
}

struct W<T> { x: T }

impl<T: Discard> Tr for W<T> {
    fn go(self) -> i64 {
        let W { x: _ } = self;
        1
    }
}

fn call<U: Tr>(u: U) -> i64 { u.go() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&call(W { x: Fd { n: 1 } }))?;
    Result::Ok(())
}
// error: `call` needs its `U` to implement `Tr`, and `W<Fd>` doesn't; that needs `Fd: Discard`
