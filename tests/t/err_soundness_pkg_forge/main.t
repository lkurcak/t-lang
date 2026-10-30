// A struct whose fields aren't exported can't be made outside its package.
fn main() {
    let _a = fd::close(fd::open());
    let f = fd::Fd { raw: 7 }; // error: field `raw` of `Fd` isn't exported by `fd`
    let _b = fd::close(f);
}
