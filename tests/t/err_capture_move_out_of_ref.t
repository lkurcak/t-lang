// By value, a closure never captures through a reference: `move` takes
// the reference itself, and the body's moving a field out from behind it
// is refused, as anywhere.
#[derive(Discard)]
struct P { name: alloc::string::String }

fn take(s: alloc::string::String) -> usize { s.len() }

fn f(r: &mut P) -> usize {
    let c = move || take(r.name); // error: cannot move out of
    c()
}

fn main() with alloc {
    let mut p = P { name: alloc::string::String::from("ab") };
    let _ = f(&mut p);
}
