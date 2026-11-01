// A trait's provided method can blame its caller.
trait Check {
    fn ok(&self) -> bool;

    #[blame_caller]
    fn check(&self) with panic { assert(self.ok()); }
}

#[derive(Discard)]
struct No {}

impl Check for No {
    fn ok(&self) -> bool { false }
}

fn main() { No {}.check(); }
// error: blame_caller_trait_default.t:16:13
