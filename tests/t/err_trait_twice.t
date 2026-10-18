trait Show { fn show(&self); }
#[derive(Discard)]
struct P {}
impl Show for P { fn show(&self) {} }
impl Show for P { fn show(&self) {} }
fn main() {}
// error: `P` implements `Show` twice: here and in `impl Show for P`
