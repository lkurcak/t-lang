trait Show { fn show(&self); }
fn main() {
    let f = |x| x.show();
}
// error: the type of this must be known to call its method `show`; add a type annotation
