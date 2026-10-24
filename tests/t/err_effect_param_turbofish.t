// A type argument for an effect parameter isn't a type: only `_` can be
// written for one yet.
fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }

fn main() {
    print(apply::<i64, _>(|| 1));
}
// error: `e` is an effect parameter, which can't be written here yet
