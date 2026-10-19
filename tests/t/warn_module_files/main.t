// Warnings are in the file they're about, and fixes change every file that
// needs it: renaming `Helper` renames it in `util.t` and here.
mod util;

fn main() {
    print(util::Helper());   // expect: 2
}
