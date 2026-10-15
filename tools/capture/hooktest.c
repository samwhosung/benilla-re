/* hooktest: runs the hook rig's mechanism against functions of this program, without the client.
 *
 * It needs the test build of the hook rig, build/hooktest/imm32.dll, which skips the build check
 * and is otherwise the hook rig; run it as forwardtest is run (README.md, "Testing a build").
 *
 * The test starts itself again as a child with BENILLA_CAPTURE_HOOKS naming five of its
 * functions, so the rig reads them when it loads. The image loads at 0x400000 with no relocation
 * (build.sh links it so), so the child's addresses are the parent's. The child writes into
 * hooktest-out\, waits until all five functions begin with int3, calls them, and exits; the
 * parent then reads calls.csv and checks:
 *   - add3(1, 2, 3): one row, its three stack arguments, eax 6;
 *   - fact(4), which calls itself: four rows, the innermost first, each eax the factorial of its
 *     argument and each return address paired with its own call;
 *   - half(3.0f), which returns on the x87 stack: st0 1.5, and 1.5 reaches the caller;
 *   - catcher(5), which calls thrower(5), which leaves by longjmp back into catcher, as a Lua 5.0
 *     error does: thrower's row marked unwound, then catcher's with eax 6. Without the unwind
 *     rule catcher's return would pop thrower's entry and jump to thrower's return address;
 *   - thrower(9) called from the child's own setjmp, then add3(4, 5, 6) at the same depth: the
 *     new call drops the stale entry (an unwound row), and add3 returns 15 to its caller;
 *   - the x87 control word the child sets, the client's 0x023f, is unchanged after the calls.
 * It exits 0 on a pass and prints the first failure otherwise. */
#include <windows.h>
#include <imm.h>
#include <setjmp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Built with -O0 and frame pointers, so each begins with push ebp. */
__attribute__((noinline)) int add3(int a, int b, int c) { return a + b + c; }
__attribute__((noinline)) int fact(int n) { return n <= 1 ? 1 : n * fact(n - 1); }
__attribute__((noinline)) float half(float x) { return x * 0.5f; }

static jmp_buf g_catch;
__attribute__((noinline)) int thrower(int n) {
    longjmp(g_catch, n);
}
__attribute__((noinline)) int catcher(int n) {
    volatile int got = 0;
    if (setjmp(g_catch) == 0) thrower(n);
    else got = n + 1;
    return got;
}

static int fail(const char *what) {
    printf("hooktest: FAIL: %s\n", what);
    return 1;
}

#define NF 5
static unsigned char *const funcs[NF] = {(unsigned char *)add3, (unsigned char *)fact,
                                         (unsigned char *)half, (unsigned char *)catcher,
                                         (unsigned char *)thrower};

static int child(void) {
    for (int wait = 0; wait < 100; wait++) {
        int armed = 0;
        for (int i = 0; i < NF; i++) armed += funcs[i][0] == 0xcc;
        if (armed == NF) {
            /* The client's control word (docs/METHOD.md, The harness) must survive the hooks; bit 6
             * is reserved and changes no result. */
            unsigned short cw = 0x023f;
            __asm__ volatile("fldcw %0" : : "m"(cw));
            volatile int a = add3(1, 2, 3);
            volatile int b = fact(4);
            volatile float c = half(3.0f);
            volatile int d = catcher(5);
            volatile int e = 0;
            if (setjmp(g_catch) == 0) thrower(9);
            else e = add3(4, 5, 6);
            __asm__ volatile("fnstcw %0" : "=m"(cw));
            printf("hooktest child: add3 %d, fact %d, half %g, catcher %d, add3 after longjmp %d, "
                   "control word %04x\n", a, b, (double)c, d, e, cw);
            Sleep(1500); /* the rig writes calls.csv every 100 ms */
            return a == 6 && b == 24 && c == 1.5f && d == 6 && e == 15 &&
                           (cw & ~0x40) == 0x023f
                       ? 0
                       : 1;
        }
        Sleep(50);
    }
    printf("hooktest child: the rig never armed\n");
    return 2;
}

typedef struct {
    long seq;
    unsigned long function, return_to, eax, a0, a1, a2;
    char st0[64], eax_text[16];
    int unwound;
} Row;

static int read_rows(const char *path, Row *rows, int max) {
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    char line[1024];
    int n = 0;
    if (!fgets(line, sizeof line, f)) n = -1; /* the header */
    while (n >= 0 && n < max && fgets(line, sizeof line, f)) {
        /* seq,t_in,t_out,thread,function,ecx,return_to,eax,edx,st0,a0..a7,pre_x..post_flags,
         * unwound: 27 cells */
        char *cell[27] = {0};
        int c = 0;
        for (char *p = line; c < 27; c++) {
            cell[c] = p;
            p = strchr(p, ',');
            if (!p) break;
            *p++ = 0;
        }
        if (c < 26) continue;
        Row *r = &rows[n++];
        r->seq = strtol(cell[0], NULL, 10);
        r->function = strtoul(cell[4], NULL, 16);
        r->return_to = strtoul(cell[6], NULL, 16);
        r->eax = strtoul(cell[7], NULL, 16);
        snprintf(r->eax_text, sizeof r->eax_text, "%s", cell[7]);
        r->unwound = atoi(cell[26]);
        snprintf(r->st0, sizeof r->st0, "%s", cell[9]);
        r->a0 = strtoul(cell[10], NULL, 16);
        r->a1 = strtoul(cell[11], NULL, 16);
        r->a2 = strtoul(cell[12], NULL, 16);
    }
    fclose(f);
    return n;
}

int main(int argc, char **argv) {
    ImmGetContext(NULL); /* the import that loads the rig, as the client's does */
    if (argc > 1 && strcmp(argv[1], "child") == 0) return child();

    for (int i = 0; i < NF; i++)
        if (funcs[i][0] != 0x55) return fail("a test function does not begin with push ebp");

    char dir[MAX_PATH], self[MAX_PATH], path[MAX_PATH + 32];
    GetModuleFileNameA(NULL, self, sizeof self);
    strcpy(dir, self);
    snprintf(strrchr(dir, '\\'), 16, "\\hooktest-out");
    CreateDirectoryA(dir, NULL);
    snprintf(path, sizeof path, "%s\\calls.csv", dir);
    DeleteFileA(path);

    /* The child's variables go in its own environment block, so this process's copy of the rig
     * never sees them and writes nothing into the child's directory. */
    static char env[32768];
    size_t used = 0;
    char *parent = GetEnvironmentStringsA();
    for (char *v = parent; *v; v += strlen(v) + 1) {
        if (strncmp(v, "BENILLA_CAPTURE_", 16) == 0) continue;
        size_t len = strlen(v) + 1;
        if (used + len + 512 > sizeof env) return fail("the environment is too large");
        memcpy(env + used, v, len);
        used += len;
    }
    FreeEnvironmentStringsA(parent);
    used += snprintf(env + used, 256, "BENILLA_CAPTURE_HOOKS=0x%lx:3,0x%lx:1,0x%lx:1,0x%lx:1,0x%lx:1",
                     (unsigned long)(UINT_PTR)add3, (unsigned long)(UINT_PTR)fact,
                     (unsigned long)(UINT_PTR)half, (unsigned long)(UINT_PTR)catcher,
                     (unsigned long)(UINT_PTR)thrower) + 1;
    used += snprintf(env + used, MAX_PATH + 32, "BENILLA_CAPTURE_DIR=%s", dir) + 1;
    env[used] = 0;

    char command[MAX_PATH + 16];
    snprintf(command, sizeof command, "\"%s\" child", self);
    STARTUPINFOA si;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    if (!CreateProcessA(self, command, NULL, NULL, FALSE, 0, env, NULL, &si, &pi))
        return fail("cannot start the child");
    WaitForSingleObject(pi.hProcess, 30000);
    DWORD code = 1;
    GetExitCodeProcess(pi.hProcess, &code);
    if (code != 0) return fail("the child failed (see its lines above and hooktest-out\\hook.log)");

    Row rows[16];
    int n = read_rows(path, rows, 16);
    if (n != 10) {
        printf("hooktest: %d rows in calls.csv\n", n);
        return fail("want 10 rows: add3, four fact, half, thrower, catcher, thrower, add3");
    }
    for (int i = 0; i < 10; i++) {
        int want = i == 6 || i == 8;
        if (rows[i].unwound != want || (want && rows[i].eax_text[0] != 0))
            return fail("a row's unwound flag: only the two thrower rows are unwound");
    }
    /* add3 */
    if (rows[0].function != (UINT_PTR)add3 || rows[0].eax != 6 || rows[0].a0 != 1 ||
        rows[0].a1 != 2 || rows[0].a2 != 3)
        return fail("the add3 row");
    /* fact: written innermost first, so seq falls; eax is n! for n = a0 */
    static const unsigned long factorial[] = {1, 1, 2, 6, 24};
    for (int i = 1; i <= 4; i++) {
        Row *r = &rows[i];
        if (r->function != (UINT_PTR)fact || r->a0 != (unsigned long)i ||
            r->eax != factorial[r->a0])
            return fail("a fact row: argument or result");
        if (i > 1 && r->seq != rows[i - 1].seq - 1) return fail("the fact rows' order");
        /* the inner three return into fact, the outer one into main's caller code */
        int inner = r->return_to > (UINT_PTR)fact && r->return_to < (UINT_PTR)fact + 0x80;
        if (inner != (i < 4)) return fail("a fact row's return address");
    }
    /* half */
    if (rows[5].function != (UINT_PTR)half || strcmp(rows[5].st0, "1.5") != 0) {
        printf("hooktest: half's st0 is '%s'\n", rows[5].st0);
        return fail("the half row");
    }
    /* catcher(5) and the thrower it called, which left by longjmp */
    if (rows[6].function != (UINT_PTR)thrower || rows[6].a0 != 5) return fail("the first thrower row");
    if (rows[7].function != (UINT_PTR)catcher || rows[7].a0 != 5 || rows[7].eax != 6 ||
        rows[7].seq != rows[6].seq - 1)
        return fail("the catcher row");
    /* thrower(9), dropped when add3(4, 5, 6) entered at its depth */
    if (rows[8].function != (UINT_PTR)thrower || rows[8].a0 != 9) return fail("the second thrower row");
    if (rows[9].function != (UINT_PTR)add3 || rows[9].eax != 15 || rows[9].a0 != 4 ||
        rows[9].seq != rows[8].seq + 1)
        return fail("the add3 row after the longjmp");
    printf("hooktest: 10 calls logged with their arguments, results and pairing, two of them left "
           "by longjmp\nhooktest: PASS\n");
    return 0;
}
