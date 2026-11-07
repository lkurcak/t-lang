// An environment's variables are read by name or in order: its `index`
// and `len`, which the runtime trusts, are private.
fn main(os: std::Os) {
    let _ = os.env.len(); // error: private
}
