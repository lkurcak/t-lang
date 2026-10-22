// An `#[allow(..)]` on `mod m;` turns its lints off in `m.t` and in the
// files of the modules in it.
#[allow(dead_code, unused_variables)]
mod quiet;
mod loud;

fn main() {
    print(1i64);   // expect: 1
}
