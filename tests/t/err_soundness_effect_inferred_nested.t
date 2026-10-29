// So is a nested function.
export fn f() -> i64 {
    fn inner() -> i64 { panic("boom") }
    inner()
}

fn main() with panic { let _ = f(); }
// error: `inner` may panic, but `f` doesn't say `with panic`
