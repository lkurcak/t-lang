//! Fetching packages from git (see docs/27-git-dependencies.md): a commit
//! of a repository, checked out once into a cache, by the `git` command,
//! so the user's credentials and keys are what reach the repository.
//!
//! A commit's hash is the hash of its files, so what's checked out for a
//! `rev` is the same wherever and whenever it's fetched: the cache is never
//! refreshed, and a package once fetched needs no network. A checkout is
//! the commit's files and nothing else: no `.git`, and no links, which git
//! writes as files holding their targets, so nothing in it can be read
//! from outside it. Fetching runs no hooks, the repository's or the
//! user's, and reaches repositories only by transports that just fetch.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The transports git may use: not `ext::`, which runs a command.
const PROTOCOLS: &str = "https:http:ssh:git:file";

/// The URL prefixes T fetches from, as messages list them.
pub(crate) const URLS: &str = "`https://`, `http://`, `ssh://`, `git://`, `file://` and \
                               `user@host:path`";

/// Where T keeps what it fetches: `$T_LANG_CACHE`, or the system's place
/// for caches, in `t-lang`.
pub fn cache_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    if let Some(dir) = var("T_LANG_CACHE") {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        return var("LOCALAPPDATA").map(|d| Path::new(&d).join("t-lang").join("cache"));
    }
    if let Some(dir) = var("XDG_CACHE_HOME") {
        return Some(Path::new(&dir).join("t-lang"));
    }
    let home = PathBuf::from(var("HOME")?);
    Some(match cfg!(target_os = "macos") {
        true => home.join("Library").join("Caches").join("t-lang"),
        false => home.join(".cache").join("t-lang"),
    })
}

/// Is `url` one T fetches from? Only by a transport that just fetches,
/// and nothing git could take for an option.
pub(crate) fn fetchable(url: &str) -> bool {
    let schemes = ["https://", "http://", "ssh://", "git://", "file://"];
    if (schemes.iter()).any(|s| url.len() > s.len() && url.starts_with(s)) {
        return true;
    }
    // `user@host:path`, as scp has it.
    match url.split_once(':') {
        Some((head, path)) => {
            head.contains('@')
                && !head.starts_with(['-', '@'])
                && !head.contains('/')
                && !path.is_empty()
                && !url.contains("::")
        }
        None => false,
    }
}

/// Is `rev` a commit's full hash, as git spells it: SHA-1's 40 hex digits,
/// or SHA-256's 64, in lower case?
pub(crate) fn is_rev(rev: &str) -> bool {
    matches!(rev.len(), 40 | 64) && rev.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The directory `url` at `rev` is checked out to, in `cache`.
pub(crate) fn checkout(cache: &Path, url: &str, rev: &str) -> PathBuf {
    // A name to know it by, and a hash to tell it from others of that
    // name: FNV-1a, which is the same in every build of T.
    let last = url.trim_end_matches(".git").rsplit(['/', ':']).next();
    let name: String = (last.unwrap_or("").chars())
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(32)
        .collect();
    let hash = url.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    cache
        .join("git")
        .join(format!("{name}-{hash:016x}"))
        .join(rev)
}

/// Checks out `url` at `rev` into the cache, unless it's there, and
/// returns where; or says why it couldn't.
pub(crate) fn fetch(cache: &Path, url: &str, rev: &str) -> Result<PathBuf, String> {
    let done = checkout(cache, url, rev);
    if done.is_dir() {
        return Ok(done);
    }
    let slot = done.parent().expect("a checkout is in a directory");
    std::fs::create_dir_all(slot).map_err(|e| format!("can't make `{}`: {e}", slot.display()))?;
    // Fetched beside where it goes, then moved there whole: whoever looks
    // finds all of it or nothing, even with other `t`s fetching it too.
    static FETCHES: AtomicUsize = AtomicUsize::new(0);
    let n = FETCHES.fetch_add(1, Ordering::Relaxed);
    let tmp = slot.join(format!(".{rev}.{}.{n}", std::process::id()));
    let result = fetch_into(&tmp, url, rev).and_then(|()| {
        std::fs::remove_dir_all(tmp.join(".git"))
            .map_err(|e| format!("can't tidy `{}`: {e}", tmp.display()))?;
        match std::fs::rename(&tmp, &done) {
            Ok(()) => Ok(()),
            // Another `t` got there first, with the same files.
            Err(_) if done.is_dir() => Ok(()),
            Err(e) => Err(format!(
                "can't move the checkout to `{}`: {e}",
                done.display()
            )),
        }
    });
    let _ = std::fs::remove_dir_all(&tmp);
    result.map(|()| done)
}

/// Makes `dir` a checkout of `url` at `rev`, with `.git` still in it.
fn fetch_into(dir: &Path, url: &str, rev: &str) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("can't make `{}`: {e}", dir.display()))?;
    git(dir, &["init", "-q"])?;
    // Just the commit, if the server lets a client ask for one by hash, as
    // GitHub's does; else everything, which has it if anything does.
    let shallow = ["fetch", "-q", "--depth", "1", "--no-tags", "--", url, rev];
    if git(dir, &shallow).is_err() {
        let refs = "+refs/*:refs/fetched/*";
        git(dir, &["fetch", "-q", "--no-tags", "--", url, refs])?;
    }
    git(dir, &["checkout", "-q", "--detach", rev])?;
    let head = git(dir, &["rev-parse", "HEAD"])?;
    if head != rev {
        return Err(format!("it checked out `{head}` rather than `{rev}`"));
    }
    Ok(())
}

/// Runs git in `dir`, and returns what it printed, or why it failed.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    // Hooks from a directory that isn't there: none run. In `.git`, which
    // no checkout writes into.
    let hooks = format!(
        "core.hooksPath={}",
        dir.join(".git").join("no-hooks").display()
    );
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_ALLOW_PROTOCOL", PROTOCOLS)
        .args(["-c", &hooks])
        .args(["-c", "core.symlinks=false"])
        .args(["-c", "core.autocrlf=false"])
        .args(["-c", "core.fsmonitor=false"])
        .args(["-c", "submodule.recurse=false"])
        .args(["-c", "advice.detachedHead=false"])
        .args(args)
        .output()
        .map_err(|e| format!("can't run `git`, which git dependencies need: {e}"))?;
    match out.status.success() {
        true => Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()),
        false => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let why = (stderr.lines())
                .map(|l| l.trim_start_matches("fatal: ").trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            Err(match why.is_empty() {
                true => format!("`git {}` failed", args[0]),
                false => why,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_urls_that_just_fetch() {
        for url in [
            "https://github.com/a/b",
            "http://example.com/b.git",
            "ssh://git@github.com/a/b",
            "git://example.com/b",
            "file:///tmp/b",
            "git@github.com:a/b.git",
        ] {
            assert!(fetchable(url), "{url}");
        }
        for url in [
            "",
            "https://",
            "ext::sh -c touch% /tmp/x",
            "fd::3",
            "-uupload-pack=touch /tmp/x",
            "../b",
            "/tmp/b",
            "b",
            "git@github.com:",
            "-x@host:path",
            "a@b::c",
            "a/b@c:d",
            "C:\\repos\\b",
        ] {
            assert!(!fetchable(url), "{url}");
        }
    }

    #[test]
    fn revs_are_full_hashes() {
        assert!(is_rev(&"a".repeat(40)));
        assert!(is_rev(&"0".repeat(64)));
        for rev in [
            "",
            "abc1234",
            &"a".repeat(39),
            &"A".repeat(40),
            &"g".repeat(40),
        ] {
            assert!(!is_rev(rev), "{rev}");
        }
    }

    #[test]
    fn checkouts_are_by_url_and_rev() {
        let cache = Path::new("c");
        let a = checkout(cache, "https://github.com/x/hashmap.git", "r");
        let shown = |p: PathBuf| p.to_string_lossy().replace('\\', "/");
        assert!(
            shown(a.clone()).starts_with("c/git/hashmap-"),
            "{}",
            a.display()
        );
        assert!(a.ends_with("r"));
        assert_ne!(a, checkout(cache, "https://github.com/y/hashmap.git", "r"));
        assert_ne!(a, checkout(cache, "https://github.com/x/hashmap.git", "s"));
        // FNV-1a of the empty string, so the hash can't change between
        // builds without this test seeing it.
        assert_eq!(shown(checkout(cache, "", "r")), "c/git/-cbf29ce484222325/r");
    }
}
