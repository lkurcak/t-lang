// A reference returned early has to live as long as the result's lifetime
// says, as one returned at the end does.
fn pick<'a>(a: &'a int, c: bool) -> &'a int {
    let local = 5;
    if c { return &local; }
    a
}
fn main() {
    let x = 1;
    print(*pick(&x, false));
}
// error: `local` doesn't live long enough: it goes away when the function returns
