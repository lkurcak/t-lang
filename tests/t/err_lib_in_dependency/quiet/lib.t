// error: this package's `t.toml` doesn't list the `alloc` library in its `libraries`
export fn f() with alloc { let b = alloc::boxed::Box::new(1_i64); }
