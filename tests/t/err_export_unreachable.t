// What's exported is its package's interface, so other packages must be
// able to name it: `m` isn't exported, so nothing outside can reach `f`.
mod m {
    export fn f() -> i64 { 1 }
}

fn main() { let _x = m::f(); }
// error: `f` is exported, but other packages can't name it: module `m` isn't exported, and no `export use` names it
