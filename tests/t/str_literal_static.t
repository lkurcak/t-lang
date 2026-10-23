// A string literal lives as long as the program does: it can be returned
// from anywhere, and kept anywhere.
fn greeting(polite: bool) -> &'static str {
    if polite { "good morning" } else { "hi" }
}

#[derive(Discard)]
struct Named { name: &'static str }

fn main() {
    let n = Named { name: greeting(true) };
    print_str(n.name); // expect: good morning
    print_str(greeting(false)); // expect: hi
    print(n.name.len()); // expect: 12
}
