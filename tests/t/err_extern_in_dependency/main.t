// Only a program can name C functions, not a library: a library
// couldn't be sure a C name is its own, and adding one would break builds.
fn main() {
    dev::f();
}
