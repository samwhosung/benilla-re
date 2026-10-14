/* The shared core of the capture rigs (rig.h). */
#include "rig.h"

#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

/* ---- Client addresses, each with the entry that explains it. symbols/ has rows for
 * s_objectManager, ObjectManager +0x00 (m_liveObjects, whose meaning names the buckets at +0x1c
 * and the mask at +0x24) and +0xc0, CGObject_C +0x30, and CGUnit_C +0x118 and +0x9a8.
 * CGObject_C +0x18 has no row; objectlayer/manager names it in the walk and the insert. ---- */

/* s_objectManager: the manager block, null outside the world
 * (objectlayer/manager#manager-block). */
#define OBJECT_MANAGER 0x00b41414u
/* Manager offsets: the active player's GUID at +0xc0/+0xc4, read by ClntObjMgrGetActivePlayer
 * 0x468550 (objectlayer/manager#accessors-and-enumeration); the live table's buckets at +0x1c and
 * its mask at +0x24, -1 before first use (objectlayer/manager#manager-block). */
#define MGR_ACTIVE_GUID 0xc0
#define MGR_BUCKETS 0x1c
#define MGR_MASK 0x24
/* An object's table key: the low GUID dword at +0x18, the GUID at +0x30/+0x34
 * (objectlayer/manager#finding-an-object). */
#define OBJ_HASH 0x18
#define OBJ_GUID 0x30
/* CGUnit_C +0x118 points at the unit's CMovement, which the constructor builds at +0x9a8
 * (objectlayer/units#the-unit-object, collision/mover#construction). */
#define UNIT_MOVEMENT_PTR RIG_UNIT_MOVEMENT_PTR
#define UNIT_MOVEMENT RIG_UNIT_MOVEMENT

/* ---- The build. WoW.exe 1.12.1.5875 has no relocations and loads at 0x400000; its PE header is
 * at 0x110, with TimeDateStamp 0x4510b6db and SizeOfImage 0x906000. ---- */
#define IMAGE_BASE 0x00400000u
#define PE_OFFSET 0x110u
#define PE_TIMESTAMP 0x4510b6dbu
#define PE_IMAGE_SIZE 0x00906000u

static HANDLE g_self;
static HMODULE g_module;
static FILE *g_log;
static LARGE_INTEGER g_freq, g_t0;
static CRITICAL_SECTION g_log_lock;

RIG_NO_FPU int rig_read(UINT_PTR addr, void *out, SIZE_T n) {
    /* ReadProcessMemory on the rig's own process returns FALSE where an access would fault, so a
     * page the game frees during the read costs a sample, not the process. */
    SIZE_T got = 0;
    return ReadProcessMemory(g_self, (LPCVOID)addr, out, n, &got) && got == n;
}

RIG_NO_FPU DWORD rig_dword(UINT_PTR addr) {
    DWORD v = 0;
    rig_read(addr, &v, 4);
    return v;
}

float rig_float(UINT_PTR addr) {
    float v = 0;
    rig_read(addr, &v, 4);
    return v;
}

RIG_NO_FPU LONGLONG rig_ticks(void) {
    LARGE_INTEGER now;
    QueryPerformanceCounter(&now);
    return now.QuadPart;
}

double rig_ticks_ms(LONGLONG ticks) {
    return 1000.0 * (double)(ticks - g_t0.QuadPart) / (double)g_freq.QuadPart;
}

double rig_ms(void) { return rig_ticks_ms(rig_ticks()); }

unsigned rig_env_uint(const char *name, unsigned fallback) {
    char text[32];
    DWORD n = GetEnvironmentVariableA(name, text, sizeof text);
    if (n == 0 || n >= sizeof text) return fallback;
    char *end = NULL;
    unsigned long v = strtoul(text, &end, 0);
    return (end && *end == 0) ? (unsigned)v : fallback;
}

static void output_path(char *out, size_t size, const char *file) {
    char dir[MAX_PATH];
    DWORD n = GetEnvironmentVariableA("BENILLA_CAPTURE_DIR", dir, sizeof dir);
    if (n == 0 || n >= sizeof dir) {
        n = GetModuleFileNameA(g_module, dir, sizeof dir);
        if (n == 0 || n >= sizeof dir) {
            strcpy(dir, ".");
        } else {
            char *slash = strrchr(dir, '\\');
            if (slash) *slash = 0;
        }
    }
    snprintf(out, size, "%s\\%s", dir, file);
}

FILE *rig_open(const char *file) {
    char path[MAX_PATH + 64];
    output_path(path, sizeof path, file);
    FILE *f = fopen(path, "w");
    rig_log(f ? "writing %s" : "cannot open %s", path);
    return f;
}

void rig_log(const char *fmt, ...) {
    if (!g_log) return;
    EnterCriticalSection(&g_log_lock);
    fprintf(g_log, "%10.1f ", rig_ms());
    va_list ap;
    va_start(ap, fmt);
    vfprintf(g_log, fmt, ap);
    va_end(ap);
    fputc('\n', g_log);
    fflush(g_log);
    LeaveCriticalSection(&g_log_lock);
}

int rig_check_build(void) {
#ifdef RIG_TEST_ANY_IMAGE
    /* The hooktest build (build.sh): the rig runs against the test's own image. */
    rig_log("test build: the build check is skipped");
    return 1;
#endif
    DWORD mz = rig_dword(IMAGE_BASE) & 0xffff;
    DWORD lfanew = rig_dword(IMAGE_BASE + 0x3c);
    DWORD sig = rig_dword(IMAGE_BASE + PE_OFFSET);
    DWORD stamp = rig_dword(IMAGE_BASE + PE_OFFSET + 8);
    DWORD size = rig_dword(IMAGE_BASE + PE_OFFSET + 0x18 + 0x38);
    if (mz == 0x5a4d && lfanew == PE_OFFSET && sig == 0x4550 && stamp == PE_TIMESTAMP &&
        size == PE_IMAGE_SIZE)
        return 1;
    rig_log("not WoW.exe 1.12.1.5875 at 0x%08x: MZ %04lx, PE offset 0x%lx, signature %08lx, "
            "timestamp %08lx (want %08x), image size %08lx (want %08x); the rig does nothing",
            IMAGE_BASE, mz, lfanew, sig, stamp, PE_TIMESTAMP, size, PE_IMAGE_SIZE);
    return 0;
}

int rig_text_bounds(UINT_PTR *start, UINT_PTR *end) {
    UINT_PTR pe = IMAGE_BASE + rig_dword(IMAGE_BASE + 0x3c);
    DWORD sections = rig_dword(pe + 4) & 0xffff;
    DWORD optional = rig_dword(pe + 0x14) & 0xffff;
    UINT_PTR table = pe + 0x18 + optional;
    for (DWORD i = 0; i < sections; i++) {
        char name[8];
        UINT_PTR s = table + 40 * i;
        if (!rig_read(s, name, 8)) return 0;
        if (memcmp(name, ".text\0\0\0", 8) == 0) {
            *start = IMAGE_BASE + rig_dword(s + 12);
            *end = *start + rig_dword(s + 8);
            return 1;
        }
    }
    return 0;
}

RIG_NO_FPU int rig_find_player(RigPlayer *p) {
    UINT_PTR mgr = rig_dword(OBJECT_MANAGER);
    if (!mgr) return 0;
    DWORD lo = rig_dword(mgr + MGR_ACTIVE_GUID), hi = rig_dword(mgr + MGR_ACTIVE_GUID + 4);
    if (!(lo | hi)) return 0;
    /* The live-table walk of 0x464890: the bucket is buckets + 12 * (lo & mask); its first dword
     * is the chain's link offset and its third the first object; a pointer with bit 0 set, or
     * null, ends the chain; the next object is at [object + link + 4]. */
    DWORD mask = rig_dword(mgr + MGR_MASK);
    if (mask == 0xffffffffu) return 0;
    UINT_PTR bucket = rig_dword(mgr + MGR_BUCKETS) + 12 * (lo & mask);
    DWORD link = rig_dword(bucket);
    UINT_PTR obj = rig_dword(bucket + 8);
    /* The game thread can change the table during the walk; a bound stops a walk a torn read
     * sends around a loop. */
    for (int step = 0; step < 4096 && obj && !(obj & 1); step++) {
        if (rig_dword(obj + OBJ_HASH) == lo && rig_dword(obj + OBJ_GUID) == lo &&
            rig_dword(obj + OBJ_GUID + 4) == hi) {
            UINT_PTR movement = rig_dword(obj + UNIT_MOVEMENT_PTR);
            if (movement != obj + UNIT_MOVEMENT) return 0; /* not a unit, or a torn read */
            p->unit = obj;
            p->movement = movement;
            p->guid_lo = lo;
            p->guid_hi = hi;
            return 1;
        }
        obj = rig_dword(obj + link + 4);
    }
    return 0;
}

RIG_NO_FPU int rig_player_current(const RigPlayer *p) {
    UINT_PTR mgr = rig_dword(OBJECT_MANAGER);
    return mgr && rig_dword(mgr + MGR_ACTIVE_GUID) == p->guid_lo &&
           rig_dword(mgr + MGR_ACTIVE_GUID + 4) == p->guid_hi &&
           rig_dword(p->unit + OBJ_GUID) == p->guid_lo &&
           rig_dword(p->unit + OBJ_GUID + 4) == p->guid_hi &&
           rig_dword(p->unit + UNIT_MOVEMENT_PTR) == p->movement;
}

static DWORD WINAPI rig_thread(LPVOID arg) {
    (void)arg;
    /* Files are opened here, not in DllMain, which runs under the loader lock. */
    char log[64];
    snprintf(log, sizeof log, "%s.log", rig_name);
    char path[MAX_PATH + 64];
    output_path(path, sizeof path, log);
    g_log = fopen(path, "w");
    rig_log("%s rig loaded into process %lu", rig_name, GetCurrentProcessId());
    DWORD code = rig_check_build() ? rig_main() : 1;
    rig_log("%s rig stopped (%lu)", rig_name, code);
    return code;
}

BOOL WINAPI DllMain(HINSTANCE module, DWORD reason, LPVOID reserved) {
    (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(module);
        g_module = module;
        g_self = GetCurrentProcess();
        QueryPerformanceFrequency(&g_freq);
        QueryPerformanceCounter(&g_t0);
        InitializeCriticalSection(&g_log_lock);
        HANDLE t = CreateThread(NULL, 0, rig_thread, NULL, 0, NULL);
        if (t) CloseHandle(t);
    }
    return TRUE;
}
