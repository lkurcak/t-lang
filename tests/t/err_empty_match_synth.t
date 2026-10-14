// With no arms and nothing that uses it, nothing says what type this `match`
// has. `empty_enum.t` has one that's used, so it's inferred.
enum Never {}
fn f(n: Never) {
    let _x = match n {};
}
fn main() {}
// error: cannot infer the type of this `match`; it has no arms to tell
