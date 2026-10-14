/* The hook rig: logs each call of chosen client functions, with its arguments and its result,
 * into calls.csv.
 *
 * BENILLA_CAPTURE_HOOKS lists the functions as address:args pairs, comma-separated, for example
 * "0x636100:4"; args is how many stack dwords to log (0 to 8). A hooked function must begin with
 * push ebp (0x55), the one instruction the rig steps over itself.
 *
 * How a call is caught. The rig writes int3 (0xcc) over the function's first byte and installs a
 * vectored exception handler. At the breakpoint the handler records ecx, the return address and
 * the stack arguments, replaces the return address with the address of the return stub, performs
 * the push ebp the breakpoint replaced (esp -= 4, [esp] = ebp, eip = entry + 1), and continues.
 * The breakpoint stays in place for the next call; nothing is relocated and no trap flag is used.
 *
 * The return takes no exception. Some translation layers lose the x87 registers across an
 * exception (hooktest shows it), and a function's x87 result is live when it returns, while at
 * its entry the x87 stack is empty by the calling convention. So the function returns into the
 * stub below, which copies st0 when it is in use, hands eax, edx and that copy to on_return(),
 * and jumps to the real return address on_return() gives back, with every register, the flags and
 * the x87 environment as the function left them. Each thread keeps its own stack of open calls,
 * so nested and recursive calls pair correctly.
 *
 * A call can also leave without returning: Lua 5.0 raises an error by longjmp, and an SEH unwind
 * skips frames the same way. Its entry then stays on the stack, and the next return must not pop
 * it. So each entry keeps esp at entry, which points at the return address. A live frame's entry
 * esp is above every call it makes; a frame that has left, by return or by unwind, has an entry esp
 * below the current esp. So the handler first drops every open entry whose entry esp is at or below
 * the new call's, and on_return() takes the outermost entry below the esp it returned with and
 * drops the ones above it, deeper frames an unwind abandoned. A dropped entry is written out as a
 * row with unwound = 1 and no result.
 *
 * Neither the handler nor on_return() uses the x87 or writes the file: a finished call goes into
 * a lock-free ring the rig's own thread writes out, so a thread killed at exit can leave no lock
 * held.
 *
 * BENILLA_CAPTURE_FILTER=mover keeps only calls whose ecx is the active player's CMovement, and
 * for those logs its position and movement flags before and after the call. The handler checks
 * that at each call by looking the player up itself (rig_find_player). */
#include "rig.h"

#include <stdlib.h>
#include <string.h>

const char rig_name[] = "hook";

#define MAX_HOOKS 16
#define MAX_ARGS 8
#define MAX_DEPTH 64
#define RING 8192

/* CMovement fields (symbols/fields.tsv): the position at +0x10, the movement flags at +0x40. */
#define MV_POSITION 0x10
#define MV_FLAGS 0x40

typedef struct {
    UINT_PTR address;
    unsigned args;
} Hook;

typedef struct {
    LONG seq;
    DWORD thread;
    LONGLONG t_in, t_out;
    const Hook *hook;
    DWORD entry_esp; /* esp at entry: the slot holding the return address */
    int unwound;     /* left by an unwind, not a return: no t_out and no result */
    DWORD ecx, return_to, eax, edx;
    DWORD args[MAX_ARGS];
    int has_st0;
    unsigned char st0[10];
    int mover; /* ecx is the active player's CMovement */
    float pre[3], post[3];
    DWORD pre_flags, post_flags;
} Call;

typedef struct {
    int depth;
    Call calls[MAX_DEPTH];
} Stack;

static Hook g_hooks[MAX_HOOKS];
static int g_hook_count;
static int g_only_mover;
static DWORD g_tls = TLS_OUT_OF_INDEXES;
static volatile LONG g_seq;

/* Finished calls, written by the game's threads and drained by the rig's: a bounded queue with a
 * sequence number per slot (Vyukov's), so no writer ever waits on a lock. A writer claims slot
 * pos by moving g_head from pos to pos + 1 while the slot's seq is pos, fills it, and sets seq to
 * pos + 1; the reader takes the slot at g_tail when its seq is g_tail + 1 and sets it to
 * g_tail + RING, which frees it for the next round. */
typedef struct {
    volatile unsigned seq;
    Call call;
} Slot;
static Slot g_ring[RING];
static volatile unsigned g_head;
static unsigned g_tail;
static volatile LONG g_dropped;

void return_stub(void);
double x87_probe(void);
extern const unsigned char x87_probe_int3[];
DWORD __cdecl on_return(const DWORD *regs, const unsigned char *st0, int has_st0);

/* The stub's frame, from esp up: 28 bytes of fnstenv, a 10-byte copy of st0 and 2 of padding
 * (40); pushad's eight registers, eax last at +28 of them (32); the flags (4); then the slot the
 * real return address goes in, which the final ret pops. st0 is copied only when its register is
 * in use and the one below it is free, so the fld cannot overflow the stack; fldenv puts the
 * status, control and tag words back after the copy and after on_return(). */
__asm__(".intel_syntax noprefix\n"
        ".text\n"
        ".globl _return_stub\n"
        "_return_stub:\n"
        "    sub esp, 4\n"
        "    pushfd\n"
        "    pushad\n"
        "    sub esp, 40\n"
        "    fnstenv [esp]\n"
        "    fldenv [esp]\n"
        "    xor ebx, ebx\n"
        "    mov ecx, [esp+4]\n"   /* status word */
        "    shr ecx, 11\n"
        "    and ecx, 7\n"         /* TOP: the physical register st0 is */
        "    mov esi, [esp+8]\n"   /* tag word, two bits per physical register */
        "    mov edx, esi\n"
        "    shr edx, cl\n"
        "    shr edx, cl\n"
        "    and edx, 3\n"
        "    cmp edx, 3\n"         /* st0 empty: nothing to copy */
        "    je 1f\n"
        "    dec ecx\n"
        "    and ecx, 7\n"         /* the register a push would use */
        "    shr esi, cl\n"
        "    shr esi, cl\n"
        "    and esi, 3\n"
        "    cmp esi, 3\n"         /* in use: the stack is full, do not push */
        "    jne 1f\n"
        "    fld st(0)\n"
        "    fstp tbyte ptr [esp+28]\n"
        "    mov ebx, 1\n"
        "1:\n"
        "    lea eax, [esp+28]\n"
        "    lea ecx, [esp+40]\n"
        "    push ebx\n"
        "    push eax\n"
        "    push ecx\n"
        "    call _on_return\n"
        "    add esp, 12\n"
        "    mov [esp+76], eax\n"
        "    fldenv [esp]\n"
        "    add esp, 40\n"
        "    popad\n"
        "    popfd\n"
        "    ret\n"
        ".att_syntax prefix\n");

/* x87_probe() returns 1 + 1 computed before an int3 the handler steps over: 2 where the system
 * carries the x87 registers through an exception, something else where it does not. */
__asm__(".intel_syntax noprefix\n"
        ".text\n"
        ".globl _x87_probe\n"
        ".globl _x87_probe_int3\n"
        "_x87_probe:\n"
        "    fld1\n"
        "    fld1\n"
        "    faddp st(1), st\n"
        "_x87_probe_int3:\n"
        "    int3\n"
        "    ret\n"
        ".att_syntax prefix\n");

static const Hook *hook_at(UINT_PTR address) {
    for (int i = 0; i < g_hook_count; i++)
        if (g_hooks[i].address == address) return &g_hooks[i];
    return NULL;
}

static Stack *thread_stack(void) {
    Stack *s = (Stack *)TlsGetValue(g_tls);
    if (!s) {
        s = (Stack *)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, sizeof *s);
        if (s) TlsSetValue(g_tls, s);
    }
    return s;
}

/* general-regs-only: the compiler may use no x87 or SSE register here, nor in the handler. */
__attribute__((target("general-regs-only"))) static void ring_put(const Call *k) {
    unsigned pos = __atomic_load_n(&g_head, __ATOMIC_RELAXED);
    for (;;) {
        Slot *slot = &g_ring[pos % RING];
        int diff = (int)(__atomic_load_n(&slot->seq, __ATOMIC_ACQUIRE) - pos);
        if (diff == 0) {
            if (__atomic_compare_exchange_n(&g_head, &pos, pos + 1, 1, __ATOMIC_RELAXED,
                                            __ATOMIC_RELAXED)) {
                slot->call = *k;
                __atomic_store_n(&slot->seq, pos + 1, __ATOMIC_RELEASE);
                return;
            }
        } else if (diff < 0) { /* full */
            __atomic_add_fetch(&g_dropped, 1, __ATOMIC_SEQ_CST);
            return;
        } else {
            pos = __atomic_load_n(&g_head, __ATOMIC_RELAXED);
        }
    }
}

static int ring_take(Call *k) {
    Slot *slot = &g_ring[g_tail % RING];
    if (__atomic_load_n(&slot->seq, __ATOMIC_ACQUIRE) != g_tail + 1) return 0;
    *k = slot->call;
    __atomic_store_n(&slot->seq, g_tail + RING, __ATOMIC_RELEASE);
    g_tail++;
    return 1;
}

/* Writes out the open entries from index `from` up as unwound, and drops them. */
__attribute__((target("general-regs-only"))) static void drop_unwound(Stack *s, int from) {
    while (s->depth > from) {
        Call *k = &s->calls[--s->depth];
        k->unwound = 1;
        ring_put(k);
    }
}

__attribute__((target("general-regs-only"))) DWORD __cdecl on_return(const DWORD *regs,
                                                                      const unsigned char *st0,
                                                                      int has_st0) {
    Stack *s = (Stack *)TlsGetValue(g_tls);
    /* regs is pushad's block; its esp, regs[3], is the stub's esp at entry less the flags and the
     * reserved slot: the esp the function returned with is regs[3] + 8. */
    DWORD esp = regs[3] + 8;
    /* The returning call is the outermost open entry below esp; any above it were left by an
     * unwind. Only a redirected return reaches the stub, so there is one; if there is not, the
     * real return address is lost and returning to 0 stops the client where the fault is plain. */
    int i = s ? s->depth : 0;
    while (i > 0 && s->calls[i - 1].entry_esp < esp) i--;
    if (!s || i == s->depth) return 0;
    drop_unwound(s, i + 1);
    Call *k = &s->calls[--s->depth];
    k->t_out = rig_ticks();
    k->eax = regs[7];
    k->edx = regs[5];
    k->has_st0 = has_st0;
    if (has_st0) memcpy(k->st0, st0, 10);
    if (k->mover) {
        rig_read(k->ecx + MV_POSITION, k->post, sizeof k->post);
        k->post_flags = rig_dword(k->ecx + MV_FLAGS);
    }
    ring_put(k);
    return k->return_to;
}

/* 1 when ecx is the active player's CMovement now. A cheap test of the shape first (the unit
 * that would embed it points back at it), then the live-table lookup, so a released object, a
 * reused one or a new player is told apart at this call, not on the rig thread's next look. */
__attribute__((target("general-regs-only"))) static int is_player_movement(UINT_PTR ecx) {
    if (ecx < RIG_UNIT_MOVEMENT || rig_dword(ecx - RIG_UNIT_MOVEMENT + RIG_UNIT_MOVEMENT_PTR) != ecx)
        return 0;
    RigPlayer p;
    return rig_find_player(&p) && p.movement == ecx;
}

__attribute__((target("general-regs-only"))) static LONG CALLBACK
on_exception(PEXCEPTION_POINTERS ep) {
    if (ep->ExceptionRecord->ExceptionCode != EXCEPTION_BREAKPOINT)
        return EXCEPTION_CONTINUE_SEARCH;
    UINT_PTR at = (UINT_PTR)ep->ExceptionRecord->ExceptionAddress;
    CONTEXT *c = ep->ContextRecord;
    if (at == (UINT_PTR)x87_probe_int3) {
        c->Eip = at + 1;
        return EXCEPTION_CONTINUE_EXECUTION;
    }
    const Hook *h = hook_at(at);
    if (!h) return EXCEPTION_CONTINUE_SEARCH;
    UINT_PTR esp = c->Esp; /* [esp] is the return address, the arguments follow */
    int mover = g_only_mover && is_player_movement(c->Ecx);
    Stack *s = (!g_only_mover || mover) ? thread_stack() : NULL;
    if (s) {
        /* An open entry at or below this call's esp is a frame that has left by an unwind. */
        int i = s->depth;
        while (i > 0 && s->calls[i - 1].entry_esp <= esp) i--;
        drop_unwound(s, i);
    }
    if (s && s->depth < MAX_DEPTH) {
        Call *k = &s->calls[s->depth++];
        memset(k, 0, sizeof *k);
        k->entry_esp = (DWORD)esp;
        k->seq = __atomic_add_fetch(&g_seq, 1, __ATOMIC_SEQ_CST);
        k->thread = GetCurrentThreadId();
        k->t_in = rig_ticks();
        k->hook = h;
        k->ecx = c->Ecx;
        k->return_to = *(DWORD *)esp;
        for (unsigned i = 0; i < h->args; i++) k->args[i] = *(DWORD *)(esp + 4 + 4 * i);
        k->mover = mover;
        if (mover) {
            rig_read(c->Ecx + MV_POSITION, k->pre, sizeof k->pre);
            k->pre_flags = rig_dword(c->Ecx + MV_FLAGS);
        }
        *(DWORD *)esp = (DWORD)(UINT_PTR)return_stub;
    }
    /* The push ebp the breakpoint replaced. */
    c->Esp -= 4;
    *(DWORD *)(UINT_PTR)c->Esp = c->Ebp;
    c->Eip = at + 1;
    return EXCEPTION_CONTINUE_EXECUTION;
}

static void write_call(FILE *csv, const Call *k) {
    char st0[40] = "", t_out[32] = "", eax[16] = "", edx[16] = "";
    if (!k->unwound) {
        snprintf(t_out, sizeof t_out, "%.3f", rig_ticks_ms(k->t_out));
        snprintf(eax, sizeof eax, "%08lx", k->eax);
        snprintf(edx, sizeof edx, "%08lx", k->edx);
    }
    if (k->has_st0) {
        long double v = 0;
        memcpy(&v, k->st0, 10);
        snprintf(st0, sizeof st0, "%.17g", (double)v);
    }
    fprintf(csv, "%ld,%.3f,%s,%lu,%06lx,%08lx,%08lx,%s,%s,%s", k->seq, rig_ticks_ms(k->t_in),
            t_out, k->thread, (unsigned long)k->hook->address, k->ecx, k->return_to, eax, edx,
            st0);
    for (unsigned i = 0; i < MAX_ARGS; i++)
        i < k->hook->args ? fprintf(csv, ",%08lx", k->args[i]) : fputc(',', csv);
    if (k->mover)
        fprintf(csv, ",%.4f,%.4f,%.4f,%08lx", k->pre[0], k->pre[1], k->pre[2], k->pre_flags);
    else
        fputs(",,,,", csv);
    if (k->mover && !k->unwound)
        fprintf(csv, ",%.4f,%.4f,%.4f,%08lx", k->post[0], k->post[1], k->post[2], k->post_flags);
    else
        fputs(",,,,", csv);
    fprintf(csv, ",%d\n", k->unwound);
}

/* Parses BENILLA_CAPTURE_HOOKS and checks each address before anything is written. */
static int read_hooks(void) {
    char spec[1024];
    DWORD n = GetEnvironmentVariableA("BENILLA_CAPTURE_HOOKS", spec, sizeof spec);
    if (n == 0 || n >= sizeof spec) {
        rig_log("BENILLA_CAPTURE_HOOKS is not set: nothing to hook");
        return 0;
    }
    UINT_PTR text_start = 0, text_end = 0;
    if (!rig_text_bounds(&text_start, &text_end)) {
        rig_log("cannot read the .text bounds from the image header");
        return 0;
    }
    for (char *item = strtok(spec, ","); item; item = strtok(NULL, ",")) {
        char *end = NULL;
        unsigned long address = strtoul(item, &end, 16);
        unsigned long args = 0;
        if (end && *end == ':') args = strtoul(end + 1, &end, 10);
        if (!end || *end != 0 || args > MAX_ARGS || g_hook_count == MAX_HOOKS) {
            rig_log("cannot read the hook '%s' (address:args, args 0 to %d, at most %d hooks)",
                    item, MAX_ARGS, MAX_HOOKS);
            return 0;
        }
        unsigned char first = 0;
        if (address < text_start || address >= text_end) {
            rig_log("0x%06lx is outside .text [0x%06lx, 0x%06lx)", address,
                    (unsigned long)text_start, (unsigned long)text_end);
            return 0;
        }
        if (!rig_read(address, &first, 1) || first != 0x55) {
            rig_log("0x%06lx begins with 0x%02x, not push ebp (0x55)", address, first);
            return 0;
        }
        if (hook_at(address)) {
            rig_log("0x%06lx is listed twice", address);
            return 0;
        }
        g_hooks[g_hook_count].address = address;
        g_hooks[g_hook_count].args = (unsigned)args;
        g_hook_count++;
    }
    return g_hook_count > 0;
}

static int plant(UINT_PTR address) {
    DWORD old;
    if (!VirtualProtect((void *)address, 1, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *(volatile unsigned char *)address = 0xcc;
    VirtualProtect((void *)address, 1, old, &old);
    FlushInstructionCache(GetCurrentProcess(), (void *)address, 1);
    return 1;
}

DWORD rig_main(void) {
    char filter[16] = "";
    GetEnvironmentVariableA("BENILLA_CAPTURE_FILTER", filter, sizeof filter);
    g_only_mover = strcmp(filter, "mover") == 0;
    if (!read_hooks()) return 1;

    for (unsigned i = 0; i < RING; i++) g_ring[i].seq = i;
    g_tls = TlsAlloc();
    FILE *csv = rig_open("calls.csv");
    if (g_tls == TLS_OUT_OF_INDEXES || !csv) {
        rig_log("cannot set up: TLS slot or calls.csv");
        return 1;
    }
    fprintf(csv, "seq,t_in_ms,t_out_ms,thread,function,ecx,return_to,eax,edx,st0,"
                 "a0,a1,a2,a3,a4,a5,a6,a7,"
                 "pre_x,pre_y,pre_z,pre_flags,post_x,post_y,post_z,post_flags,unwound\n");
    fflush(csv);
    if (!AddVectoredExceptionHandler(1, on_exception)) {
        rig_log("AddVectoredExceptionHandler failed (%lu)", GetLastError());
        return 1;
    }
    if (x87_probe() == 2.0)
        rig_log("x87 registers survive an exception here");
    else
        rig_log("this system loses the x87 registers across an exception: hook no function that "
                "takes arguments on the x87 stack, as some C runtime helpers do");
    for (int i = 0; i < g_hook_count; i++) {
        if (!plant(g_hooks[i].address)) {
            rig_log("cannot write 0x%06lx (%lu); no hook beyond the ones above is armed",
                    (unsigned long)g_hooks[i].address, GetLastError());
            break;
        }
        rig_log("armed 0x%06lx, logging %u stack arguments", (unsigned long)g_hooks[i].address,
                g_hooks[i].args);
    }
    rig_log("filter %s", g_only_mover ? "mover" : "none");

    /* Write out finished calls; log the active player's CMovement as it changes. */
    long written = 0, dropped = 0;
    UINT_PTR logged = 0;
    for (unsigned tick = 0;; tick++) {
        if (g_only_mover && tick % 5 == 0) {
            RigPlayer p;
            UINT_PTR movement = rig_find_player(&p) ? p.movement : 0;
            if (movement != logged)
                rig_log("active player movement %08lx", (unsigned long)movement);
            logged = movement;
        }
        Call k;
        while (ring_take(&k)) {
            write_call(csv, &k);
            written++;
        }
        fflush(csv);
        if (g_dropped != dropped) {
            dropped = g_dropped;
            rig_log("%ld calls dropped: the ring of %d was full", dropped, RING);
        }
        if (tick % 100 == 0) rig_log("%ld calls written", written);
        Sleep(100);
    }
}
