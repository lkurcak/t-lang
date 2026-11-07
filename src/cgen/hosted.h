/* The hosted target (docs/39-freestanding.md): the platform's functions
 * from the C library, std's streams, and what's around `main`. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <signal.h>
#include <pthread.h>
#include <sched.h>
#include <unistd.h>
#include <fcntl.h>

extern char **environ;

void t_platform_write(uint8_t byte) {
    /* What the program wrote first, as the interpreter shows it. */
    fflush(stdout);
    fputc(byte, stderr);
}

_Noreturn void t_platform_halt(void) {
    fflush(stderr);
    _Exit(t_panicking ? 101 : 1);
}

/* `malloc` aligns for any type, which is all a T type needs: `align` is
 * at most `_Alignof(max_align_t)`. */
void *t_platform_alloc(size_t size, size_t align) {
    (void)align;
    return malloc(size);
}

void *t_platform_realloc(void *p, size_t size, size_t align, size_t new_size) {
    (void)size;
    (void)align;
    return realloc(p, new_size);
}

void t_platform_free(void *p, size_t size, size_t align) {
    (void)size;
    (void)align;
    free(p);
}

/* What a stream's intrinsic gives back when it fails with `errno` `e`:
 * bit 63, the `core::io::ErrorKind` by its place in the enum in bits 32
 * to 39, and `e` in the low 32 bits, as `std::io::failure` reads it. */
static uint64_t t_io_fail(int e) {
    uint64_t kind;
    switch (e) {
    case ENOENT: kind = 0; break;
    case EACCES: case EPERM: kind = 1; break;
    case EEXIST: kind = 2; break;
    case EPIPE: kind = 3; break;
    case ENOSYS: case ENOTSUP: kind = 6; break;
    default: kind = 8; break;
    }
    return ((uint64_t)1 << 63) | (kind << 32) | (uint32_t)e;
}

/* The input's buffer: `Stdin` reads through it, so a byte at a time is
 * quick. Only the one `Stdin` reaches it, by `&mut` (lemma C2 of
 * docs/19-std-io.md), so no two threads do at once. */
static uint8_t t_in_buf[4096];
static size_t t_in_at, t_in_len;

/* Reads into `buf`, `n` bytes long, as `Read::read` does: what's in the
 * buffer, or else what one `read` gives. The output is written out first,
 * so that a prompt is seen before it's answered. */
static uint64_t t_stdin_read(uint8_t *buf, uint64_t n) {
    if (n == 0) return 0;
    if (fflush(stdout) != 0) return t_io_fail(errno);
    if (t_in_at == t_in_len) {
        uint8_t *into = n >= sizeof t_in_buf ? buf : t_in_buf;
        size_t len = n >= sizeof t_in_buf ? (size_t)n : sizeof t_in_buf;
        ssize_t r;
        do r = read(0, into, len); while (r < 0 && errno == EINTR);
        if (r < 0) return t_io_fail(errno);
        if (into == buf) return (uint64_t)r;
        t_in_at = 0;
        t_in_len = (size_t)r;
        if (r == 0) return 0;
    }
    size_t k = t_in_len - t_in_at;
    if (k > n) k = (size_t)n;
    memcpy(buf, t_in_buf + t_in_at, k);
    t_in_at += k;
    return k;
}

/* Writes all `n` bytes at `p` to `f`: 0, or why it couldn't. Before
 * anything goes where errors go, the output is written out. */
static uint64_t t_out_write(FILE *f, const uint8_t *p, uint64_t n) {
    if (f == stderr) (void)fflush(stdout);
    if (n == 0) return 0;
    errno = 0;
    if (fwrite(p, 1, n, f) == n) return 0;
    return t_io_fail(errno ? errno : EIO);
}

static uint64_t t_out_flush(FILE *f) {
    errno = 0;
    return fflush(f) == 0 ? 0 : t_io_fail(errno ? errno : EIO);
}

/* A file open to be read (docs/48-files.md): where a `ReadFile` points.
 * Only the one `ReadFile` reaches it, by `&mut` to read, or by value to
 * close, so no two threads do at once. */
struct t_file {
    int fd;
    size_t at, len;
    uint8_t buf[4096];
};

/* Copies path `p`, `n` bytes with no `\0` in them, into `into` with a `\0`
 * after: 0, or `ENAMETOOLONG` if it doesn't fit, which no path that long
 * would anyway, as the operating system's are shorter. */
static int t_path(char into[4096], const uint8_t *p, uint64_t n) {
    if (n >= 4096) return ENAMETOOLONG;
    memcpy(into, p, n);
    into[n] = 0;
    return 0;
}

/* Opens the file at path `p`, `n` bytes long, to be read, into `*f`: 0, or
 * the failure, as `t_io_fail` gives it. Its buffer is memory: if there's
 * none, the program stops, at `site`. */
static uint64_t t_file_open(const uint8_t *p, uint64_t n, struct t_file **f, uint32_t site) {
    char path[4096];
    int e = t_path(path, p, n);
    if (e != 0) return t_io_fail(e);
    int fd;
    do fd = open(path, O_RDONLY | O_CLOEXEC); while (fd < 0 && errno == EINTR);
    if (fd < 0) return t_io_fail(errno);
    *f = t_alloc(sizeof **f, _Alignof(struct t_file), site);
    (*f)->fd = fd;
    (*f)->at = (*f)->len = 0;
    return 0;
}

/* Reads into `buf`, `n` bytes long, from `f`, as `t_stdin_read` does from
 * the input. */
static uint64_t t_file_read(struct t_file *f, uint8_t *buf, uint64_t n) {
    if (n == 0) return 0;
    if (f->at == f->len) {
        uint8_t *into = n >= sizeof f->buf ? buf : f->buf;
        size_t len = n >= sizeof f->buf ? (size_t)n : sizeof f->buf;
        ssize_t r;
        do r = read(f->fd, into, len); while (r < 0 && errno == EINTR);
        if (r < 0) return t_io_fail(errno);
        if (into == buf) return (uint64_t)r;
        f->at = 0;
        f->len = (size_t)r;
        if (r == 0) return 0;
    }
    size_t k = f->len - f->at;
    if (k > n) k = (size_t)n;
    memcpy(buf, f->buf + f->at, k);
    f->at += k;
    return k;
}

/* Closes `f`, and frees it. A file only read loses nothing if `close`
 * fails, and its descriptor is gone either way: it's never closed again. */
static void t_file_close(struct t_file *f) {
    (void)close(f->fd);
    t_free(f, sizeof *f, _Alignof(struct t_file));
}

/* Writes all `n` bytes at `b` to the file at path `p`, `pn` bytes long,
 * made if there's none, after what it holds if `append`, or else instead
 * of it: 0, or the first failure, closing it included. */
static uint64_t t_file_write(const uint8_t *p, uint64_t pn, const uint8_t *b, uint64_t n,
                             bool append) {
    char path[4096];
    int e = t_path(path, p, pn);
    if (e != 0) return t_io_fail(e);
    int flags = O_WRONLY | O_CREAT | O_CLOEXEC | (append ? O_APPEND : O_TRUNC);
    int fd;
    do fd = open(path, flags, 0666); while (fd < 0 && errno == EINTR);
    if (fd < 0) return t_io_fail(errno);
    uint64_t failed = 0;
    while (n > 0) {
        ssize_t r = write(fd, b, n);
        if (r < 0 && errno == EINTR) continue;
        if (r < 0) {
            failed = t_io_fail(errno);
            break;
        }
        b += r;
        n -= (uint64_t)r;
    }
    if (close(fd) != 0 && failed == 0 && errno != EINTR) failed = t_io_fail(errno);
    return failed;
}

/* Removes the file at path `p`, `n` bytes long: 0, or the failure. */
static uint64_t t_file_remove(const uint8_t *p, uint64_t n) {
    char path[4096];
    int e = t_path(path, p, n);
    if (e != 0) return t_io_fail(e);
    return unlink(path) == 0 ? 0 : t_io_fail(errno);
}

/* What the program was started with (docs/47-io.md): its name, then its
 * arguments, and its environment's variables, each `NAME=value`, with
 * where its `=` is. Kept as they were when it started: nothing in T
 * changes them, so a `&'static str` of one lives as long as the program. */
static char *t_no_args[] = {"", NULL};
static char **t_argv = t_no_args;
static uint64_t t_argc = 1;
static char **t_envp;
static size_t *t_env_eq;
static uint64_t t_envc;

/* Are the `n` bytes at `s` UTF-8, as `core::str::from_utf8` says? */
static bool t_utf8(const uint8_t *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        uint8_t b = s[i];
        size_t k;
        uint32_t c, least;
        if (b < 0x80) { i++; continue; }
        if ((b & 0xE0) == 0xC0) { k = 1; c = b & 0x1F; least = 0x80; }
        else if ((b & 0xF0) == 0xE0) { k = 2; c = b & 0x0F; least = 0x800; }
        else if ((b & 0xF8) == 0xF0) { k = 3; c = b & 0x07; least = 0x10000; }
        else return false;
        if (n - i <= k) return false;
        for (size_t j = 1; j <= k; j++) {
            if ((s[i + j] & 0xC0) != 0x80) return false;
            c = c << 6 | (s[i + j] & 0x3F);
        }
        if (c < least || c > 0x10FFFF || (c >= 0xD800 && c < 0xE000)) return false;
        i += k + 1;
    }
    return true;
}

/* Keeps what the program was started with, for a `main` that takes the
 * `Os`: if something of it isn't UTF-8, the program says so, at `site`,
 * where `main` is, and stops, before `main` (lemma A1 of docs/47-io.md). */
static void t_start_os(int argc, char **argv, uint32_t site) {
    if (argc > 0) {
        t_argc = (uint64_t)argc;
        t_argv = argv;
    }
    for (uint64_t i = 0; i < t_argc; i++) {
        if (t_utf8((const uint8_t *)t_argv[i], strlen(t_argv[i]))) continue;
        t_begin();
        if (i == 0) {
            t_say("the program's name isn't UTF-8");
        } else {
            t_say("argument ");
            t_say_i((int64_t)(i - 1));
            t_say(" isn't UTF-8");
        }
        t_end(site);
    }
    size_t n = 0;
    while (environ != NULL && environ[n] != NULL) n++;
    t_envp = malloc((n + 1) * sizeof *t_envp);
    t_env_eq = malloc((n + 1) * sizeof *t_env_eq);
    if (t_envp == NULL || t_env_eq == NULL) t_fail("out of memory", site);
    for (size_t i = 0; i < n; i++) {
        char *v = environ[i];
        char *eq = strchr(v, '=');
        /* Not a variable: the C library would skip it too. */
        if (eq == NULL) continue;
        if (!t_utf8((const uint8_t *)v, strlen(v))) {
            t_begin();
            t_say("environment variable `");
            for (char *c = v; c < eq; c++) t_platform_write((uint8_t)*c);
            t_say("` isn't UTF-8");
            t_end(site);
        }
        t_envp[t_envc] = v;
        t_env_eq[t_envc] = (size_t)(eq - v);
        t_envc++;
    }
}

/* Ends the program after `main` returned: flushes its output, and fails
 * if `failed`, as `t run` does; a failed flush is shown at `site`, where
 * `main` is. */
static int t_exit(bool failed, uint32_t site) {
    errno = 0;
    if (fflush(stdout) != 0) {
        int e = errno;
        t_begin();
        t_say("cannot flush stdout: ");
        t_say(strerror(e));
        t_say(" (os error ");
        t_say_i(e);
        t_say(")");
        t_end(site);
    }
    return failed ? 1 : 0;
}

static void t_start(void) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
}

static void t_spin(void) { sched_yield(); }

/* Starts a thread, `th`, running `go(k)`, with a stack as big as `main`'s
 * usually is. If it can't, the program stops, as when there's no memory:
 * a stack is memory (docs/41-threads.md). */
static void t_thread_start(pthread_t *th, void *(*go)(void *), void *k, uint32_t site) {
    pthread_attr_t attr;
    int failed = pthread_attr_init(&attr) != 0;
    if (!failed) {
        (void)pthread_attr_setstacksize(&attr, (size_t)8 << 20);
        failed = pthread_create(th, &attr, go, k) != 0;
        (void)pthread_attr_destroy(&attr);
    }
    if (failed) t_fail("out of memory: no thread could be started", site);
}

/* Waits for thread `th` to end. */
static void t_thread_join(pthread_t th) {
    if (pthread_join(th, NULL) != 0) t_bug("a thread couldn't be joined");
}
