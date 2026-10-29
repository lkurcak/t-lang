// `export use` can't make more of an item than it is.
mod m {
    pub fn f() -> i64 { 1 }
}

export use m::f;

fn main() { let _x = f(); }
// error: `f` isn't exported, so `export use` can't export it
