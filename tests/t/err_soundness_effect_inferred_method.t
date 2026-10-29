// An inherent method that isn't exported is inferred, like a function.
mod m {
    #[derive(Discard)]
    pub struct S {}

    impl S {
        pub fn get(&self) -> i64 { panic("boom") }
    }
}

export fn f() -> i64 { m::S {}.get() }

fn main() { let _ = f(); }
// error: `get` may panic, but `f` doesn't say `with panic`
