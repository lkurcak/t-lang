trait Show { fn show(&self); }
impl Show for Self { fn show(&self) {} }
fn main() {}
// error: `Self` is the type an impl is for, so it can't be used to say what that is
