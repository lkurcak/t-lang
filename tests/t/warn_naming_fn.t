// Functions are named in snake_case. Two nested ones share a name: each
// fix renames only one of them, and what uses it.
fn addOne(x: i64) -> i64 with panic { x + 1 }

fn main() with panic {
    fn doIt() -> i64 with panic { addOne(1) }
    print(doIt() + other());
}

fn other() -> i64 {
    fn doIt() -> i64 { 10 }
    doIt()
}
// warning: function `addOne` should have a snake case name: `add_one`
// warning: function `doIt` should have a snake case name: `do_it`
// warning: function `doIt` should have a snake case name: `do_it`
// expect: 12
