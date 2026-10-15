// Functions are named in snake_case. Two nested ones share a name: each
// fix renames only one of them, and what uses it.
fn addOne(x: int) -> int { x + 1 }

fn main() {
    fn doIt() -> int { addOne(1) }
    print(doIt() + other());
}

fn other() -> int {
    fn doIt() -> int { 10 }
    doIt()
}
// warning: function `addOne` should have a snake case name: `add_one`
// warning: function `doIt` should have a snake case name: `do_it`
// warning: function `doIt` should have a snake case name: `do_it`
// expect: 12
