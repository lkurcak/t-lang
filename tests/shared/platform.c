/* A platform for freestanding T programs, on a hosted machine, for the
 * tests (docs/39-freestanding.md): the platform's functions as the hosted
 * target has them, and a `main` that runs `t_main`. It also checks the
 * runtime keeps its side of the contract: every block is freed or
 * resized with the size and alignment it was given with. */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void t_main(void);

void t_platform_write(uint8_t byte) { fputc(byte, stderr); }

_Noreturn void t_platform_halt(void) {
    fflush(stderr);
    _Exit(1);
}

/* Each block has its size and alignment in front of it. */
typedef struct {
    size_t size, align;
    _Alignas(16) unsigned char data[];
} block;

static _Noreturn void broken(const char *what, size_t a, size_t b) {
    fprintf(stderr, "platform: %s (%zu, given %zu)\n", what, a, b);
    abort();
}

static void check(size_t size, size_t align) {
    if (size == 0) broken("a block of no bytes", size, 0);
    if (align == 0 || (align & (align - 1)) != 0 || align > 16)
        broken("an alignment that isn't a power of two up to 16", align, 0);
}

static block *head(void *p, size_t size, size_t align) {
    block *b = (block *)((unsigned char *)p - offsetof(block, data));
    if (b->size != size) broken("a size other than the block's", size, b->size);
    if (b->align != align) broken("an alignment other than the block's", align, b->align);
    return b;
}

void *t_platform_alloc(size_t size, size_t align) {
    check(size, align);
    block *b = malloc(sizeof(block) + size);
    if (!b) return NULL;
    b->size = size;
    b->align = align;
    return b->data;
}

void *t_platform_realloc(void *p, size_t size, size_t align, size_t new_size) {
    check(new_size, align);
    block *b = realloc(head(p, size, align), sizeof(block) + new_size);
    if (!b) return NULL;
    b->size = new_size;
    return b->data;
}

void t_platform_free(void *p, size_t size, size_t align) { free(head(p, size, align)); }

int main(void) {
    t_main();
    return 0;
}
