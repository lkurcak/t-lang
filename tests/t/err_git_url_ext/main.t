// git's `ext::` runs a command to reach a repository: fetching must run
// nothing, so only the transports that just fetch are allowed.
// error: `ext::sh -c touch% /tmp/pwned` isn't a URL T fetches from: it takes `https://`, `http://`, `ssh://`, `git://`, `file://` and `user@host:path`
fn main() {}
