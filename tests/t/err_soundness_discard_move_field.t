// Nor can a field be moved out of it, outside its module.
use alloc::string::String;

mod named {
    pub struct Named { pub name: alloc::string::String }

    impl Discard for Named {
        fn discard(self) {}
    }
}

fn main() with alloc {
    let n = named::Named { name: String::from("a") };
    let _s = n.name;
}
// error: can't take `Named` apart here: it has a `discard` of its own, which only `named` may skip
