/* The runtime of a T program compiled to C (docs/33-c-backend.md): what
 * its intrinsics need, and arithmetic as T defines it. Everything here is
 * defined behaviour in ISO C11: integers are computed in
 * `int64_t`/`uint64_t` and checked before anything could overflow,
 * conversions to signed types go through `t_sx`, and shifts never reach
 * the width of their type.
 *
 * It needs no C library, only C11's freestanding headers: what it needs
 * of the machine it gets from the platform's functions below
 * (docs/39-freestanding.md). On the hosted target, `hosted.h` gives them,
 * with the C library; on the freestanding one, the program's platform. */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

_Static_assert(sizeof(void *) == 8, "T's C backend needs 64-bit pointers");
_Static_assert(sizeof(size_t) == 8, "T's C backend needs a 64-bit size_t");

/* The platform. Writes a byte of a panic's message to wherever errors
 * go; it can't fail, or it fails quietly. One byte at a time, so that a
 * program can write it in T (docs/40-extern.md). */
void t_platform_write(uint8_t byte);
/* Stops the program, after a panic: it never returns. */
_Noreturn void t_platform_halt(void);
/* A block of `size` bytes (never 0), aligned to `align` (a power of two,
 * at most `_Alignof(max_align_t)`), or `NULL` if there's no memory. */
void *t_platform_alloc(size_t size, size_t align);
/* Block `p`, of `size` bytes and alignment `align`, made `new_size` bytes,
 * its contents kept up to the smaller of the two; or `NULL` if there's no
 * memory, `p` then left as it was. */
void *t_platform_realloc(void *p, size_t size, size_t align, size_t new_size);
/* Gives back block `p`, given with `size` and `align`. */
void t_platform_free(void *p, size_t size, size_t align);

/* Waits a moment, in a loop that waits for another thread: lets another
 * run, on the hosted target; on the freestanding one, tells the
 * processor, if it can be. */
static void t_spin(void);

/* The symbol of C name `name`, a string literal, for an `__asm__` label:
 * with the prefix the machine's C names get, if any (`_` on macOS). */
#define T_STRING_(x) #x
#define T_STRING(x) T_STRING_(x)
#ifdef __USER_LABEL_PREFIX__
#define T_SYMBOL(name) T_STRING(__USER_LABEL_PREFIX__) name
#else
#define T_SYMBOL(name) name
#endif

/* Writes `len` bytes, from `bytes`, where errors go. */
static void t_write(const uint8_t *bytes, size_t len) {
    for (size_t i = 0; i < len; i++) t_platform_write(bytes[i]);
}

/* What a value of a type that holds nothing is: ISO C has no empty
 * structs, so it's a byte nothing reads. */
typedef struct { uint8_t _; } t_unit;

/* Where every empty slice and buffer points: somewhere real, aligned for
 * any type, never read or written, never freed. */
static max_align_t t_nowhere;
#define T_NOWHERE ((void *)&t_nowhere)

/* What a site's location looks like in a message, every line after the
 * first of the interpreter's. */
static const char *t_site_text(uint32_t site);

/* Writes C string `s` where errors go. */
static void t_say(const char *s) {
    size_t n = 0;
    while (s[n]) n++;
    t_write((const uint8_t *)s, n);
}

/* Writes `n` in decimal where errors go. */
static void t_say_u(uint64_t n) {
    uint8_t b[20];
    size_t i = sizeof b;
    do {
        b[--i] = (uint8_t)('0' + n % 10);
        n /= 10;
    } while (n);
    t_write(b + i, sizeof b - i);
}

static void t_say_i(int64_t n) {
    if (n < 0) {
        t_say("-");
        t_say_u(-(uint64_t)n);
    } else {
        t_say_u((uint64_t)n);
    }
}

static _Noreturn void t_end(uint32_t site) {
    t_say("\n");
    t_say(t_site_text(site));
    /* A panic aborts: nothing is dropped, or freed, on the way out. */
    t_platform_halt();
}

/* Whether a thread is saying why the program stops: only one does, and
 * any other that gets here waits for the end, so their messages aren't
 * mixed (docs/41-threads.md). */
static uint32_t t_stopping;

static void t_begin(void) {
    if (__atomic_exchange_n(&t_stopping, 1, __ATOMIC_SEQ_CST)) {
        for (;;) t_spin();
    }
    t_say("runtime error: ");
}

/* Whether the program stops because it panicked, rather than running out
 * of memory: the hosted platform then ends it with status 101, as the
 * interpreter does (docs/44-runs-and-holes.md). Only the thread that's saying why
 * the program stops sets it. */
static bool t_panicking;

/* `t_begin`, for a panic. */
static void t_begin_panic(void) {
    t_begin();
    t_panicking = true;
}

/* Stops the program with message `msg`, at `site`: for what isn't a panic,
 * running out of memory. */
static _Noreturn void t_fail(const char *msg, uint32_t site) {
    t_begin();
    t_say(msg);
    t_end(site);
}

/* Stops the program with message `msg`, at `site`, a panic. */
static _Noreturn void t_fail_panic(const char *msg, uint32_t site) {
    t_begin_panic();
    t_say(msg);
    t_end(site);
}

/* A bug of the checker's or this backend's: what it proved can't happen
 * did. Not a panic: no T program can make it happen. */
static _Noreturn void t_bug(const char *what) {
    t_say("internal error: ");
    t_say(what);
    t_say("\n");
    t_platform_halt();
}

static _Noreturn void t_unreachable(void) { t_bug("control got where it can't"); }

/* Are the `n` bytes at `a` the `m` at `b`? A string pattern's test, with
   no C library to call. */
static bool t_bytes_eq(const uint8_t *a, uint64_t n, const uint8_t *b, uint64_t m) {
    if (n != m) return false;
    for (uint64_t i = 0; i < n; i++) {
        if (a[i] != b[i]) return false;
    }
    return true;
}

/* A block for a value of `size` bytes, aligned to `align`: a type's
 * `sizeof` and `_Alignof`, as `t_free` is given them again. */
static void *t_alloc(size_t size, size_t align, uint32_t site) {
    void *p = t_platform_alloc(size ? size : 1, align);
    if (!p) t_fail("out of memory", site);
    return p;
}

static void t_free(void *p, size_t size, size_t align) {
    t_platform_free(p, size ? size : 1, align);
}

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
        if (!t_add_ok_##N(a, b)) t_fail_panic("integer overflow", s);                        \
        return (T)((int64_t)a + b);                                                    \
    }                                                                                  \
    static inline T t_sub_##N(T a, T b, uint32_t s) {                                  \
        if (!t_sub_ok_##N(a, b)) t_fail_panic("integer overflow", s);                        \
        return (T)((int64_t)a - b);                                                    \
    }                                                                                  \
    static inline T t_mul_##N(T a, T b, uint32_t s) {                                  \
        if (!t_mul_ok_##N(a, b)) t_fail_panic("integer overflow", s);                        \
        return (T)((int64_t)a * b);                                                    \
    }                                                                                  \
    static inline T t_div_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail_panic("division by zero", s);                                     \
        if (a == MIN && b == -1) t_fail_panic("integer overflow", s);                        \
        return (T)(a / b);                                                             \
    }                                                                                  \
    static inline T t_rem_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail_panic("division by zero", s);                                     \
        if (a == MIN && b == -1) t_fail_panic("integer overflow", s);                        \
        return (T)(a % b);                                                             \
    }                                                                                  \
    static inline T t_neg_##N(T a, uint32_t s) {                                       \
        if (a == MIN) t_fail_panic("integer overflow", s);                                   \
        return (T)(-(int64_t)a);                                                       \
    }                                                                                  \
    static inline T t_shl_##N(T a, uint32_t r, uint32_t s) {                           \
        if (r >= W) {                                                                  \
            if (a != 0) t_fail_panic("integer overflow", s);                                 \
            return 0;                                                                  \
        }                                                                              \
        int64_t lo = -(int64_t)(((uint64_t)1 << (W - 1)) >> r), hi = MAX >> r;         \
        if (a < lo || a > hi) t_fail_panic("integer overflow", s);                           \
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
    static inline bool t_add_ok_##N(uint64_t a, uint64_t b) { return a <= (uint64_t)MAX - b; }   \
    static inline bool t_sub_ok_##N(uint64_t a, uint64_t b) { return a >= b; }         \
    static inline bool t_mul_ok_##N(uint64_t a, uint64_t b) {                          \
        return b == 0 || a <= (uint64_t)MAX / b;                                       \
    }                                                                                  \
    static inline T t_add_##N(T a, T b, uint32_t s) {                                  \
        if (!t_add_ok_##N(a, b)) t_fail_panic("integer overflow", s);                        \
        return (T)((uint64_t)a + b);                                                   \
    }                                                                                  \
    static inline T t_sub_##N(T a, T b, uint32_t s) {                                  \
        if (!t_sub_ok_##N(a, b)) t_fail_panic("integer overflow", s);                        \
        return (T)((uint64_t)a - b);                                                   \
    }                                                                                  \
    static inline T t_mul_##N(T a, T b, uint32_t s) {                                  \
        if (!t_mul_ok_##N(a, b)) t_fail_panic("integer overflow", s);                        \
        return (T)((uint64_t)a * b);                                                   \
    }                                                                                  \
    static inline T t_div_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail_panic("division by zero", s);                                     \
        return (T)((uint64_t)a / b);                                                   \
    }                                                                                  \
    static inline T t_rem_##N(T a, T b, uint32_t s) {                                  \
        if (b == 0) t_fail_panic("division by zero", s);                                     \
        return (T)((uint64_t)a % b);                                                   \
    }                                                                                  \
    static inline T t_neg_##N(T a, uint32_t s) {                                       \
        if (a != 0) t_fail_panic("integer overflow", s);                                     \
        return 0;                                                                      \
    }                                                                                  \
    static inline T t_shl_##N(T a, uint32_t r, uint32_t s) {                           \
        if (r >= W) {                                                                  \
            if (a != 0) t_fail_panic("integer overflow", s);                                 \
            return 0;                                                                  \
        }                                                                              \
        if ((uint64_t)a > ((uint64_t)MAX >> r)) t_fail_panic("integer overflow", s);                   \
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
    t_begin_panic();
    t_say("`as` overflowed: ");
    t_say_i(n);
    t_say(" isn't ");
    t_say(what);
    t_end(site);
}

static _Noreturn void t_as_fail_u(uint64_t n, const char *what, uint32_t site) {
    t_begin_panic();
    t_say("`as` overflowed: ");
    t_say_u(n);
    t_say(" isn't ");
    t_say(what);
    t_end(site);
}

static _Noreturn void t_char_fail_i(int64_t n, uint32_t site) {
    t_begin_panic();
    t_say("`");
    t_say_i(n);
    t_say("` isn't a character");
    t_end(site);
}

static _Noreturn void t_char_fail_u(uint64_t n, uint32_t site) {
    t_begin_panic();
    t_say("`");
    t_say_u(n);
    t_say("` isn't a character");
    t_end(site);
}

static inline bool t_is_char(uint64_t n) {
    return n <= 0x10FFFF && !(n >= 0xD800 && n <= 0xDFFF);
}

/* `panic(why)`, its bytes `ptr`, `len` of them. */
static _Noreturn void t_panic(const uint8_t *ptr, uint64_t len, uint32_t site) {
    t_begin_panic();
    t_say("panicked: ");
    t_write(ptr, len);
    t_end(site);
}

static _Noreturn void t_oom_slots(uint64_t n, uint32_t site) {
    t_begin();
    t_say("out of memory for ");
    t_say_u(n);
    t_say(" slots");
    t_end(site);
}

/* `core::panic::Message`: written straight to where errors go, as
 * nothing can come between making one and aborting with it. */
static void t_msg_new(void) {
    t_begin_panic();
    t_say("panicked: ");
}

static void t_msg_push(const uint8_t *ptr, uint64_t len) { t_write(ptr, len); }

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
    t_write(b, (size_t)n);
}

/* Grows `*items`, of `*cap` pointers, to hold one more than `len`. */
static void t_grow_ptrs(void ***items, uint64_t *cap, uint64_t len, uint32_t site) {
    if (len < *cap) return;
    uint64_t n = *cap ? *cap * 2 : 8;
    void **p = *cap
        ? t_platform_realloc(*items, *cap * sizeof(void *), _Alignof(void *), n * sizeof(void *))
        : t_platform_alloc(n * sizeof(void *), _Alignof(void *));
    if (!p) t_fail("out of memory", site);
    *items = p;
    *cap = n;
}
