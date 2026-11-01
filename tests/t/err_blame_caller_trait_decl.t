// A trait's method without a body: blame is a property of a body, so each
// impl says it of its own.
trait Check {
    #[blame_caller]
    fn check(&self) with panic;
}

fn main() {}
// error: `#[blame_caller]` can only be on a function with a body
