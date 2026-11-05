// What a `let .. else` binds is only there after it, in its block.
fn f(o: Option<i64>) -> i64 {
    {
        let Some(n) = o else { return 0 };
    }
    n
}

fn main() {
    let _ = f(None);
}
// warning: unused variable `n`
// error: unknown variable `n`
