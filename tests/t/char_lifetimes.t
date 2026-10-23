// `'a'` is a character, `'a` a lifetime.
fn first<'a>(c: &'a char, d: &'a char) -> &'a char { if *c == 'a' { c } else { d } }

fn main() {
    let x = 'a';
    let y = 'b';
    print(*first(&x, &y) as u32); // expect: 97
    print(*first(&y, &x) as u32); // expect: 97
}
