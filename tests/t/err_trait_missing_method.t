trait Show { fn show(&self); }
#[derive(Discard)]
struct P {}
impl Show for P {}
fn main() {}
// error: this impl of `Show` for `P` is missing a method: `show`
