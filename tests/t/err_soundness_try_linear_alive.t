// A `?` may return, and `s` would be thrown away there.
struct S { v: i64 }

fn get() -> Option<i64> { Option::None }

fn f() -> Option<i64> with panic {
    let s = S { v: 1 };
    let x = get()?;
    let S { v } = s;
    Option::Some(x + v)
}

fn main() with panic { let _ = f(); }
// error: `s` goes out of scope here, if this `?` returns early
