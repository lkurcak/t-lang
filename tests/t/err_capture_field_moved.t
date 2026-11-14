// A field a `move` closure took is moved: it can't be used after.
#[derive(Discard)]
struct P { name: alloc::string::String, n: u64 }

fn main() with alloc {
    let p = P { name: alloc::string::String::from("a"), n: 1 };
    let c = move || p.name.len();
    let _l = p.name.len(); // error: use of moved value `p.name`
    let _ = c();
}
