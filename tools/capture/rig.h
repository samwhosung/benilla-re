/* The shared core of the capture rigs: the build check, fault-safe reads of the client's own
 * memory, the output files, the clock, and the read-only lookup of the active player.
 *
 * Each rig is one DLL built from rig.c and its own source, which defines rig_main(). The DLL is
 * loaded as the client's imm32.dll (README.md, "How a rig gets into the client"); its DllMain
 * starts one thread that runs rig_main(). */
#ifndef BENILLA_CAPTURE_RIG_H
#define BENILLA_CAPTURE_RIG_H

#include <windows.h>
#include <stdio.h>

/* The rig's name, used for its log file ("<name>.log"). Defined by each rig. */
extern const char rig_name[];

/* Runs on the rig's own thread once the DLL is loaded. Returns when the rig stops. */
DWORD rig_main(void);

/* Writes one line to <name>.log, flushed. */
void rig_log(const char *fmt, ...);

/* Opens <output directory>\<file> for writing. The directory is BENILLA_CAPTURE_DIR when it is
 * set, else the directory the DLL was loaded from. */
FILE *rig_open(const char *file);

/* Milliseconds since the rig started, from QueryPerformanceCounter. */
double rig_ms(void);

/* The raw counter, for code that must not touch the x87 (hook.c), and its conversion. */
LONGLONG rig_ticks(void);
double rig_ticks_ms(LONGLONG ticks);

/* An environment variable as an unsigned number (decimal or 0x hex), or fallback. */
unsigned rig_env_uint(const char *name, unsigned fallback);

/* Copies n bytes at addr; 0 when any of it is unreadable. Never faults. */
int rig_read(UINT_PTR addr, void *out, SIZE_T n);
DWORD rig_dword(UINT_PTR addr); /* 0 when unreadable */
float rig_float(UINT_PTR addr); /* 0 when unreadable */

/* 1 when the image at 0x400000 is WoW.exe 1.12.1.5875, by its PE header; logs why not. */
int rig_check_build(void);

/* The bounds of the image's .text section, read from the loaded header. */
int rig_text_bounds(UINT_PTR *start, UINT_PTR *end);

/* The active player, found the way the client finds an object, without calling into it. */
typedef struct {
    UINT_PTR unit;     /* the CGUnit_C */
    UINT_PTR movement; /* its CMovement */
    DWORD guid_lo, guid_hi;
} RigPlayer;

/* 1 and fills *p when the client is in the world and its active player is in the live table. */
int rig_find_player(RigPlayer *p);

#endif
