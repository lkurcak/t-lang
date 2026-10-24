// `Copy` too. The fix adds it to the derive that's there.
#[derive(Discard)]
struct Celsius { degrees: i64 }

impl Copy for Celsius {}

fn main() {
    let t = Celsius { degrees: 20 };
    let u = t;
    print(t.degrees + u.degrees);
}
// error: `Copy` can only be derived, with `#[derive(Copy)]` on a struct or enum, so that whether a type is `Copy` follows from its fields alone
