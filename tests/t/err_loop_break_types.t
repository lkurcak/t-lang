fn main() {
    let mut i = 0;
    let x = loop {
        i += 1;
        if i == 3 { break 1; }
        if i == 4 { break true; }
    };
    print(x);
}
// error: expected `{integer}`, found `bool`
