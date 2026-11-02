// A dependency's `t.toml` says what it uses, not its dependent's: the
// program listing alloc doesn't let `quiet` name it.
fn main(os: std::Os) {
    quiet::f();
}
