// Strings compare by their characters, as `str`s do.
use alloc::string::String;

fn main() with alloc {
    let a = String::from("abc");
    let b = String::from("abd");
    let mut c = String::from("ab");
    c.push('c');
    print_bool(a == c);   // expect: true
    print_bool(a == b);   // expect: false
    print_bool(a != b);   // expect: true
    print_bool(a.lt(&b)); // expect: true
    print_bool(b.lt(&a)); // expect: false
    print_bool(a.le(&c)); // expect: true
    a.free();
    b.free();
    c.free();
}
