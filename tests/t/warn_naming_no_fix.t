// No fix when the name it would get is a keyword, or would clash with
// another. The message only suggests a name that is one.
fn Match() -> i64 { 1 }
fn myFn() -> i64 { 2 }
fn my_fn() -> i64 { 3 }

fn main() with panic {
    let True = Match();
    print(True + myFn() + my_fn());
}
// warning: function `Match` should have a snake case name
// warning: function `myFn` should have a snake case name: `my_fn`
// warning: variable `True` should have a snake case name
// expect: 6
