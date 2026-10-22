// A local type is named like a top-level one. This one shares its name
// with a top-level type and with a local one elsewhere: each fix renames
// only one of them.
#[derive(Discard)]
struct my_pair { a: i64, b: i64 }

fn main() {
    #[derive(Discard)]
    struct my_pair { a: bool }
    let p = my_pair { a: true };
    print_bool(p.a);
    print(other().b);
}

fn other() -> my_pair {
    #[derive(Discard)]
    enum my_pair { One }
    let my_pair::One = my_pair::One;
    top()
}

fn top() -> my_pair { my_pair { a: 1, b: 2 } }
// warning: type `my_pair` should have an upper camel case name: `MyPair`
// warning: type `my_pair` should have an upper camel case name: `MyPair`
// warning: type `my_pair` should have an upper camel case name: `MyPair`
// expect: true
// expect: 2
