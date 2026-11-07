// A `Raw` comes only from `Stdin::raw`, which put the terminal in raw
// mode: one made otherwise would put back a terminal never saved (lemma T3
// of docs/49-time-entropy-terminal.md).
fn main(os: std::Os) {
    let mut input = os.stdin;
    let r = std::io::Raw { stdin: &mut input }; // error: private
    r.discard();
}
