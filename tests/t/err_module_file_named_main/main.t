// Its file would be `main.t`, this one: the program would contain itself.
mod main; // error: a module in a file of its own can't be called `main`
fn main() {}
