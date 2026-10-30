//! A library is checked on its own, with no `main`; what it exports is what
//! makes the rest live.
export fn api() -> i64 { helper() }
fn helper() -> i64 { 1 }
fn unused() -> i64 { 2 } // warning: function `unused` is never used
