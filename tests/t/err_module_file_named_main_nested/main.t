// Even inside another module: every `main.t` is where a program starts.
mod a {
    mod main; // error: can't be called `main`
}
fn main() {}
