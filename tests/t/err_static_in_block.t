// A static is declared at a module's top level.
fn main() {
    static A: u32 = 1;
// error: a static is an item of a module, not of a block
}
