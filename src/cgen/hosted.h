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
#include <limits.h>
#include <poll.h>
#include <stdatomic.h>
#include <termios.h>
#include <time.h>
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/random.h>
#if defined(__linux__)
#include <sys/syscall.h>
#endif

extern char **environ;

/* The terminal as it was before `Stdin::raw` (docs/49-time-entropy-terminal.md),
 * and whether it's in raw mode now. There's one slot, as there's never
 * more than one `Raw`: it holds the one `Stdin` by `&mut` (lemma T3). The
 * flag is atomic, and lock-free, as a signal's handler and a panicking
 * thread read it. */
static struct termios t_raw_saved;
static atomic_int t_raw_on;

/* Puts the terminal back as it was, if it's in raw mode: when a `Raw` is
 * dropped, when the program halts, and when a signal stops it (lemma T4).
 * Not when `main` returns: every `Raw` was dropped before. Safe in a
 * signal's handler: `tcsetattr` is async-signal-safe. */
static void t_raw_restore(void) {
    if (atomic_exchange(&t_raw_on, 0)) (void)tcsetattr(0, TCSANOW, &t_raw_saved);
}

void t_platform_write(uint8_t byte) {
    /* What the program wrote first, as the interpreter shows it. */
    fflush(stdout);
    fputc(byte, stderr);
}

_Noreturn void t_platform_halt(void) {
    fflush(stderr);
    t_raw_restore();
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
    case ENOSYS: case ENOTSUP: case ENOTTY: kind = 6; break;
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

/* Now, by the clock that only goes forward, in nanoseconds since some
 * moment before the program began: what `Clock::now` tells. */
static uint64_t t_clock_now(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) t_bug("the clock couldn't be read");
    return (uint64_t)ts.tv_sec * 1000000000u + (uint64_t)ts.tv_nsec;
}

/* Waits until `t_clock_now()` is no less than `deadline`. A signal may
 * wake it early, so it checks again: it never returns before. */
static void t_clock_sleep_until(uint64_t deadline) {
    for (;;) {
        uint64_t now = t_clock_now();
        if (now >= deadline) return;
        uint64_t left = deadline - now;
        struct timespec ts;
        ts.tv_sec = (time_t)(left / 1000000000u);
        ts.tv_nsec = (long)(left % 1000000000u);
        (void)nanosleep(&ts, NULL);
    }
}

/* The nanoseconds since 1970 by the time of day, or `UINT64_MAX` if that's
 * before 1970 (`Clock::unix_time`); a time too far ahead to count in a
 * `u64` is one short of that, as far as one can go. */
static uint64_t t_unix_time(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_REALTIME, &ts) != 0) t_bug("the time of day couldn't be read");
    if (ts.tv_sec < 0) return UINT64_MAX;
    if ((uint64_t)ts.tv_sec > (UINT64_MAX - 1 - 999999999u) / 1000000000u) return UINT64_MAX - 1;
    return (uint64_t)ts.tv_sec * 1000000000u + (uint64_t)ts.tv_nsec;
}

/* Fills the `n` bytes at `buf` with entropy: 0, or the failure. */
static uint64_t t_entropy_fill(uint8_t *buf, uint64_t n) {
    while (n > 0) {
        size_t k = n > 256 ? 256 : (size_t)n;
        if (getentropy(buf, k) != 0) return t_io_fail(errno);
        buf += k;
        n -= k;
    }
    return 0;
}

/* A signal that stops the program: the terminal is put back first, then it
 * stops as it would have, by the signal (lemma T4). */
static void t_raw_signal(int sig) {
    t_raw_restore();
    signal(sig, SIG_DFL);
    raise(sig);
}

/* Puts the terminal in raw mode (`Stdin::raw`): keys come one at a time,
 * unseen, and Ctrl-C is a key, but what's written is as before, so `\n`
 * still starts a line. 0, or why not: `ENOTTY` if the input isn't a
 * terminal, which is `Unsupported`. */
static uint64_t t_raw_enter(void) {
    struct termios t;
    if (tcgetattr(0, &t) != 0) return t_io_fail(errno);
    t_raw_saved = t;
    t.c_iflag &= ~(tcflag_t)(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
    t.c_lflag &= ~(tcflag_t)(ECHO | ICANON | IEXTEN | ISIG);
    t.c_cflag = (t.c_cflag & ~(tcflag_t)CSIZE) | CS8;
    t.c_cc[VMIN] = 1;
    t.c_cc[VTIME] = 0;
    static atomic_int handled;
    if (!atomic_exchange(&handled, 1)) {
        int sigs[] = {SIGHUP, SIGINT, SIGQUIT, SIGTERM};
        for (size_t i = 0; i < sizeof sigs / sizeof *sigs; i++) {
            struct sigaction sa, old;
            memset(&sa, 0, sizeof sa);
            sa.sa_handler = t_raw_signal;
            sigemptyset(&sa.sa_mask);
            /* Not if it's ignored: then it doesn't stop the program. */
            if (sigaction(sigs[i], NULL, &old) == 0 && old.sa_handler == SIG_DFL)
                (void)sigaction(sigs[i], &sa, NULL);
        }
    }
    atomic_store(&t_raw_on, 1);
    if (tcsetattr(0, TCSANOW, &t) != 0) {
        int e = errno;
        atomic_store(&t_raw_on, 0);
        return t_io_fail(e);
    }
    return 0;
}

/* Puts the terminal back as `t_raw_enter` found it: a `Raw` dropped. */
static void t_raw_leave(void) { t_raw_restore(); }

/* Waits until there's input to read, or `t_clock_now()` is no less than
 * `deadline` (`Raw::wait`): 1, 0, or the failure. What's in the buffer
 * counts, and the output is written out first, as for a read. */
static uint64_t t_stdin_wait(uint64_t deadline) {
    if (fflush(stdout) != 0) return t_io_fail(errno);
    if (t_in_at < t_in_len) return 1;
    for (;;) {
        uint64_t now = t_clock_now();
        if (now >= deadline) return 0;
        uint64_t ms = (deadline - now + 999999u) / 1000000u;
        struct pollfd p = {.fd = 0, .events = POLLIN};
        int r = poll(&p, 1, ms > INT_MAX ? INT_MAX : (int)ms);
        if (r > 0) return 1;
        if (r < 0 && errno != EINTR) return t_io_fail(errno);
    }
}

/* The size of the terminal `fd` is: its columns, shifted up 16 bits, and
 * its rows, or the failure, `ENOTTY` if it isn't one. */
static uint64_t t_term_size(int fd) {
    struct winsize w;
    if (ioctl(fd, TIOCGWINSZ, &w) != 0) return t_io_fail(errno);
    return (uint64_t)w.ws_col << 16 | w.ws_row;
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

/* How many threads the machine runs at once, for the program: on Linux,
 * the processors it may run on, as `taskset` and containers limit them;
 * elsewhere, those online. 0 if it can't say. */
static uint64_t t_threads_available(void) {
#if defined(__linux__) && defined(SYS_sched_getaffinity)
    unsigned long mask[1024 / (8 * sizeof(unsigned long))] = {0};
    long n = syscall(SYS_sched_getaffinity, 0, sizeof mask, mask);
    if (n > 0) {
        uint64_t count = 0;
        for (size_t i = 0; i < (size_t)n / sizeof mask[0]; i++) {
            count += (uint64_t)__builtin_popcountl(mask[i]);
        }
        if (count > 0) return count;
    }
#endif
    long online = sysconf(_SC_NPROCESSORS_ONLN);
    return online > 0 ? (uint64_t)online : 0;
}

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
