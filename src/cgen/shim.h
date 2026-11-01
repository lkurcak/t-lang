/* The runtime of a T program compiled to C (docs/33-c-backend.md): what
 * its intrinsics need of a hosted C library, and arithmetic as T defines
 * it. Everything here is defined behaviour in ISO C11: integers are
 * computed in `int64_t`/`uint64_t` and checked before anything could
 * overflow, conversions to signed types go through `t_sx`, and shifts
 * never reach the width of their type. */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <signal.h>

_Static_assert(sizeof(void *) == 8, "T's C backend needs 64-bit pointers");
_Static_assert(sizeof(size_t) == 8, "T's C backend needs a 64-bit size_t");

/* What a value of a type that holds nothing is: ISO C has no empty
 * structs, so it's a byte nothing reads. */
typedef struct { uint8_t _; } t_unit;

/* Where every empty slice and buffer points: somewhere real, aligned for
 * any type, never read or written, never freed. */
static max_align_t t_nowhere;
#define T_NOWHERE ((void *)&t_nowhere)

/* The innermost call made from the program's own code, not a library's:
 * where a panic in a library is shown, as the interpreter shows it. */
static uint32_t t_site;
/* What a site's location looks like in a message, every line after the
 * first of the interpreter's. */
static const char *t_site_text(uint32_t site);

static _Noreturn void t_end(uint32_t site) {
    fputc('\n', stderr);
    fputs(t_site_text(site), stderr);
    fflush(stderr);
    /* A panic aborts: nothing is dropped, or freed, on the way out. */
    _Exit(1);
}

static void t_begin(void) {
    fflush(stdout);
    fputs("runtime error: ", stderr);
}

/* Stops the program with message `msg`, at `site`. */
static _Noreturn void t_fail(const char *msg, uint32_t site) {
    t_begin();
    fputs(msg, stderr);
    t_end(site);
}

/* A bug of the checker's or this backend's: what it proved can't happen
 * did. Not a panic: no T program can make it happen. */
static _Noreturn void t_bug(const char *what) {
    fflush(stdout);
    fprintf(stderr, "internal error: %s\n", what);
    abort();
}

static _Noreturn void t_unreachable(void) { t_bug("control got where it can't"); }

static void *t_alloc(uint64_t size, uint32_t site) {
    void *p = malloc(size ? size : 1);
    if (!p) t_fail("out of memory", site);
    return p;
}

static void t_free(void *p) { free(p); }

/* The low `w` bits of `u`, as a two's complement number. */
static inline int64_t t_sx(uint64_t u, unsigned w) {
    uint64_t m = w == 64 ? ~(uint64_t)0 : ((uint64_t)1 << w) - 1;
    u &= m;
    if (u >> (w - 1) & 1) return -(int64_t)(~u & m) - 1;
    return (int64_t)u;
}

#define T_SIGNED(N, T, W, MIN, MAX)                                                    \
    static inline bool t_add_ok_##N(int64_t a, int64_t b) {                            \
        return !((b > 0 && a > MAX - b) || (b < 0 && a < MIN - b));                    \
    }                                                                                  \
    static inline bool t_sub_ok_##N(int64_t a, int64_t b) {                            \
        return !((b < 0 && a > MAX + b) || (b > 0 && a < MIN + b));                    \
    }                                                                                  \
    static inline bool t_mul_ok_##N(int64_t a, int64_t b) {                            \
        if (a > 0) return b > 0 ? a <= MAX / b : b >= MIN / a;                         \
        if (b > 0) return a >= MIN / b;                                                \
        return a == 0 || b >= MAX / a;                                                 \
    }                                                                                  \
    static inline T t_add_##N(T a, T b, uint32_t s) {                                  \
        if (!t_add_ok_##N(a, b)) t_fail("integer overflow", s);                        \
        return (T)((int64_t)a + b);                                                    \
    }                                                                                  \
    static inline T t_sub_##N(T a, T b, uint32_t s) {                                  \
        if (!t_sub_ok_##N(a, b)) t_fail("integer overflow", s);                        \
        return (T)((int64_t)a - b);                                                    \
    }                                                                                  \
    static inline T t_mul_##N(T a, T b, uint32_t s) {                                  \
        if (!t_mul_ok_##N(a, b)) t_fail("integer overflow", s);                        \
        return (T)((int64_t)a * b);                                                    \
    }                                                                                  \
    static inline T t_div_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail("division by zero", s);                                     \
        if (a == MIN && b == -1) t_fail("integer overflow", s);                        \
        return (T)(a / b);                                                             \
    }                                                                                  \
    static inline T t_rem_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail("division by zero", s);                                     \
        if (a == MIN && b == -1) t_fail("integer overflow", s);                        \
        return (T)(a % b);                                                             \
    }                                                                                  \
    static inline T t_neg_##N(T a, uint32_t s) {                                       \
        if (a == MIN) t_fail("integer overflow", s);                                   \
        return (T)(-(int64_t)a);                                                       \
    }                                                                                  \
    static inline T t_shl_##N(T a, uint32_t r, uint32_t s) {                           \
        if (r >= W) {                                                                  \
            if (a != 0) t_fail("integer overflow", s);                                 \
            return 0;                                                                  \
        }                                                                              \
        int64_t lo = -(int64_t)(((uint64_t)1 << (W - 1)) >> r), hi = MAX >> r;         \
        if (a < lo || a > hi) t_fail("integer overflow", s);                           \
        return (T)t_sx((uint64_t)(int64_t)a << r, W);                                  \
    }                                                                                  \
    /* What the checker proved can't overflow: defined, whatever happens. */           \
    static inline T t_wadd_##N(T a, T b) {                                             \
        return (T)t_sx((uint64_t)(int64_t)a + (uint64_t)(int64_t)b, W);                \
    }                                                                                  \
    static inline T t_wsub_##N(T a, T b) {                                             \
        return (T)t_sx((uint64_t)(int64_t)a - (uint64_t)(int64_t)b, W);                \
    }                                                                                  \
    static inline T t_wmul_##N(T a, T b) {                                             \
        return (T)t_sx((uint64_t)(int64_t)a * (uint64_t)(int64_t)b, W);                \
    }                                                                                  \
    static inline T t_wdiv_##N(T a, T b) {                                             \
        return b == 0 ? 0 : (a == MIN && b == -1) ? a : (T)(a / b);                    \
    }                                                                                  \
    static inline T t_wrem_##N(T a, T b) { return b == 0 || b == -1 ? 0 : (T)(a % b); } \
    static inline T t_wneg_##N(T a) { return (T)t_sx(-(uint64_t)(int64_t)a, W); }      \
    static inline T t_wshl_##N(T a, uint32_t r) {                                      \
        return r >= W ? 0 : (T)t_sx((uint64_t)(int64_t)a << r, W);                     \
    }                                                                                  \
    static inline T t_shr_##N(T a, uint32_t r) {                                       \
        if (r >= W) return a < 0 ? -1 : 0;                                             \
        return a >= 0 ? (T)((int64_t)a >> r) : (T)~(~(int64_t)a >> r);                 \
    }                                                                                  \
    static inline T t_wrap_##N(uint64_t u) { return (T)t_sx(u, W); }

#define T_UNSIGNED(N, T, W, MAX)                                                       \
    static inline bool t_add_ok_##N(uint64_t a, uint64_t b) { return a <= MAX - b; }   \
    static inline bool t_sub_ok_##N(uint64_t a, uint64_t b) { return a >= b; }         \
    static inline bool t_mul_ok_##N(uint64_t a, uint64_t b) {                          \
        return b == 0 || a <= MAX / b;                                                 \
    }                                                                                  \
    static inline T t_add_##N(T a, T b, uint32_t s) {                                  \
        if (!t_add_ok_##N(a, b)) t_fail("integer overflow", s);                        \
        return (T)((uint64_t)a + b);                                                   \
    }                                                                                  \
    static inline T t_sub_##N(T a, T b, uint32_t s) {                                  \
        if (!t_sub_ok_##N(a, b)) t_fail("integer overflow", s);                        \
        return (T)((uint64_t)a - b);                                                   \
    }                                                                                  \
    static inline T t_mul_##N(T a, T b, uint32_t s) {                                  \
        if (!t_mul_ok_##N(a, b)) t_fail("integer overflow", s);                        \
        return (T)((uint64_t)a * b);                                                   \
    }                                                                                  \
    static inline T t_div_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail("division by zero", s);                                     \
        return (T)((uint64_t)a / b);                                                   \
    }                                                                                  \
    static inline T t_rem_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail("division by zero", s);                                     \
        return (T)((uint64_t)a % b);                                                   \
    }                                                                                  \
    static inline T t_neg_##N(T a, uint32_t s) {                                       \
        if (a != 0) t_fail("integer overflow", s);                                     \
        return 0;                                                                      \
    }                                                                                  \
    static inline T t_shl_##N(T a, uint32_t r, uint32_t s) {                           \
        if (r >= W) {                                                                  \
            if (a != 0) t_fail("integer overflow", s);                                 \
            return 0;                                                                  \
        }                                                                              \
        if ((uint64_t)a > (MAX >> r)) t_fail("integer overflow", s);                   \
        return (T)((uint64_t)a << r);                                                  \
    }                                                                                  \
    static inline T t_wadd_##N(T a, T b) { return (T)((uint64_t)a + b); }              \
    static inline T t_wsub_##N(T a, T b) { return (T)((uint64_t)a - b); }              \
    static inline T t_wmul_##N(T a, T b) { return (T)((uint64_t)a * b); }              \
    static inline T t_wdiv_##N(T a, T b) { return b == 0 ? 0 : (T)((uint64_t)a / b); }  \
    static inline T t_wrem_##N(T a, T b) { return b == 0 ? 0 : (T)((uint64_t)a % b); }  \
    static inline T t_wneg_##N(T a) { return (T)(-(uint64_t)a); }                      \
    static inline T t_wshl_##N(T a, uint32_t r) {                                      \
        return r >= W ? 0 : (T)((uint64_t)a << r);                                     \
    }                                                                                  \
    static inline T t_shr_##N(T a, uint32_t r) { return r >= W ? 0 : (T)((uint64_t)a >> r); } \
    static inline T t_wrap_##N(uint64_t u) { return (T)u; }

T_SIGNED(i8, int8_t, 8, INT8_MIN, INT8_MAX)
T_SIGNED(i16, int16_t, 16, INT16_MIN, INT16_MAX)
T_SIGNED(i32, int32_t, 32, INT32_MIN, INT32_MAX)
T_SIGNED(i64, int64_t, 64, INT64_MIN, INT64_MAX)
T_UNSIGNED(u8, uint8_t, 8, UINT8_MAX)
T_UNSIGNED(u16, uint16_t, 16, UINT16_MAX)
T_UNSIGNED(u32, uint32_t, 32, UINT32_MAX)
T_UNSIGNED(u64, uint64_t, 64, UINT64_MAX)

/* `n as T` that doesn't fit, `what` being "a `u8`" or the like. */
static _Noreturn void t_as_fail_i(int64_t n, const char *what, uint32_t site) {
    t_begin();
    fprintf(stderr, "`as` overflowed: %lld isn't %s", (long long)n, what);
    t_end(site);
}

static _Noreturn void t_as_fail_u(uint64_t n, const char *what, uint32_t site) {
    t_begin();
    fprintf(stderr, "`as` overflowed: %llu isn't %s", (unsigned long long)n, what);
    t_end(site);
}

static _Noreturn void t_char_fail_i(int64_t n, uint32_t site) {
    t_begin();
    fprintf(stderr, "`%lld` isn't a character", (long long)n);
    t_end(site);
}

static _Noreturn void t_char_fail_u(uint64_t n, uint32_t site) {
    t_begin();
    fprintf(stderr, "`%llu` isn't a character", (unsigned long long)n);
    t_end(site);
}

static inline bool t_is_char(uint64_t n) {
    return n <= 0x10FFFF && !(n >= 0xD800 && n <= 0xDFFF);
}

/* `panic(why)`, its bytes `ptr`, `len` of them. */
static _Noreturn void t_panic(const uint8_t *ptr, uint64_t len, uint32_t site) {
    t_begin();
    fputs("panicked: ", stderr);
    fwrite(ptr, 1, len, stderr);
    t_end(site);
}

static _Noreturn void t_oom_slots(uint64_t n, uint32_t site) {
    t_begin();
    fprintf(stderr, "out of memory for %llu slots", (unsigned long long)n);
    t_end(site);
}

/* `core::panic::Message`: written straight to where errors go, as
 * nothing can come between making one and aborting with it. */
static void t_msg_new(void) {
    t_begin();
    fputs("panicked: ", stderr);
}

static void t_msg_push(const uint8_t *ptr, uint64_t len) { fwrite(ptr, 1, len, stderr); }

static void t_msg_char(uint32_t c) {
    uint8_t b[4];
    int n;
    if (c < 0x80) {
        b[0] = (uint8_t)c;
        n = 1;
    } else if (c < 0x800) {
        b[0] = (uint8_t)(0xC0 | c >> 6);
        b[1] = (uint8_t)(0x80 | (c & 0x3F));
        n = 2;
    } else if (c < 0x10000) {
        b[0] = (uint8_t)(0xE0 | c >> 12);
        b[1] = (uint8_t)(0x80 | (c >> 6 & 0x3F));
        b[2] = (uint8_t)(0x80 | (c & 0x3F));
        n = 3;
    } else {
        b[0] = (uint8_t)(0xF0 | c >> 18);
        b[1] = (uint8_t)(0x80 | (c >> 12 & 0x3F));
        b[2] = (uint8_t)(0x80 | (c >> 6 & 0x3F));
        b[3] = (uint8_t)(0x80 | (c & 0x3F));
        n = 4;
    }
    fwrite(b, 1, (size_t)n, stderr);
}

/* What a stream's intrinsic gives back for an error: 257 for a broken
 * pipe, 258 for anything else, as `std::io::error` reads it. */
static uint32_t t_io_error(void) { return errno == EPIPE ? 257 : 258; }

static uint32_t t_read_byte(void) {
    int c = fgetc(stdin);
    if (c != EOF) return (uint32_t)c;
    return ferror(stdin) ? t_io_error() : 256;
}

static uint32_t t_write_byte(FILE *f, uint8_t b) {
    errno = 0;
    return fputc(b, f) == EOF ? t_io_error() : 0;
}

/* Grows `*items`, of `*cap` pointers, to hold one more than `len`. */
static void t_grow_ptrs(void ***items, uint64_t *cap, uint64_t len, uint32_t site) {
    if (len < *cap) return;
    uint64_t n = *cap ? *cap * 2 : 8;
    void **p = realloc(*items, n * sizeof(void *));
    if (!p) t_fail("out of memory", site);
    *items = p;
    *cap = n;
}

/* Ends the program after `main` returned: flushes its output, and fails
 * if `failed`, as `t run` does. */
static int t_exit(bool failed) {
    if (fflush(stdout) != 0) {
        fprintf(stderr, "runtime error: cannot flush stdout: %s\n", strerror(errno));
        return 1;
    }
    return failed ? 1 : 0;
}

static void t_start(void) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
}
