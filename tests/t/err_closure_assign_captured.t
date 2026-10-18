// Changing a captured variable needs it to be `mut`, as changing it
// anywhere else does.
fn main() {
    let n = 0;
    let mut set = || n = 1;
    set();
}
// error: this closure changes `n`, so it borrows it mutably, and `n` isn't declared `mut`
