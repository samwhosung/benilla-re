/* The player rig: samples the active player's CMovement at a fixed rate into player.csv.
 *
 * The rig reads from its own thread while the game thread writes, so a row is a sample of a live
 * struct, not one frame's state: two fields of one row can come from adjacent frames. To see what
 * one call decided, use the hook rig. */
#include "rig.h"

const char rig_name[] = "player";

/* CMovement fields: their rows in symbols/fields.tsv, and collision/kinematics,
 * collision/falling and collision/mover. */
#define MV_POSITION 0x10        /* float[3], in transport space while a transport GUID is set */
#define MV_FACING 0x1c          /* float, radians */
#define MV_PITCH 0x20           /* float, radians */
#define MV_TRANSPORT 0x38       /* uint64 transport GUID, 0 for none */
#define MV_FLAGS 0x40           /* uint32 movement flags */
#define MV_FALL_TIME 0x78       /* uint32 ms on the fall arc */
#define MV_FALL_START_Z 0x7c    /* float arc start height */
#define MV_CURRENT_SPEED 0x84   /* float */
#define MV_FALL_START_SPEED 0xa0 /* float arc launch speed, down-positive */
#define MV_RADIUS 0xb0          /* float collision radius */
#define MV_HEIGHT 0xb4          /* float collision height */
#define MV_SCALE_RATIO 0xb8     /* float scale ratio */

DWORD rig_main(void) {
    unsigned hz = rig_env_uint("BENILLA_CAPTURE_HZ", 60);
    if (hz == 0 || hz > 1000) hz = 60;
    FILE *csv = rig_open("player.csv");
    if (!csv) return 1;
    fprintf(csv, "t_ms,guid,x,y,z,facing,pitch,transport,flags,fall_ms,fall_z,fall_speed,"
                 "speed,radius,height,scale\n");
    rig_log("sampling the active player at %u Hz", hz);

    RigPlayer p = {0};
    int found = 0;
    DWORD period = 1000 / hz;
    long dropped = 0;
    for (;;) {
        /* Look the player up at every sample: a released object keeps its GUID and CMovement on a
         * reuse list, so a cached pointer would go on yielding plausible rows after a teleport or
         * a relog. rig_player_current() after the reads drops a row the object changed under. */
        RigPlayer now;
        int ok = rig_find_player(&now);
        if (ok && (!found || now.movement != p.movement || now.guid_lo != p.guid_lo ||
                   now.guid_hi != p.guid_hi))
            rig_log("active player %08lx%08lx: unit %08lx, movement %08lx", now.guid_hi,
                    now.guid_lo, (unsigned long)now.unit, (unsigned long)now.movement);
        if (!ok && found) rig_log("active player lost");
        found = ok;
        if (ok) p = now;
        if (found) {
            UINT_PTR m = p.movement;
            float pos[3], f[8];
            DWORD transport[2], flags, fall_ms;
            int read = rig_read(m + MV_POSITION, pos, sizeof pos) &&
                       rig_read(m + MV_TRANSPORT, transport, sizeof transport);
            f[0] = rig_float(m + MV_FACING);
            f[1] = rig_float(m + MV_PITCH);
            flags = rig_dword(m + MV_FLAGS);
            fall_ms = rig_dword(m + MV_FALL_TIME);
            f[2] = rig_float(m + MV_FALL_START_Z);
            f[3] = rig_float(m + MV_FALL_START_SPEED);
            f[4] = rig_float(m + MV_CURRENT_SPEED);
            f[5] = rig_float(m + MV_RADIUS);
            f[6] = rig_float(m + MV_HEIGHT);
            f[7] = rig_float(m + MV_SCALE_RATIO);
            if (read && rig_player_current(&p)) {
                fprintf(csv,
                        "%.2f,%08lx%08lx,%.4f,%.4f,%.4f,%.5f,%.5f,%08lx%08lx,%08lx,%lu,%.4f,%.5f,"
                        "%.4f,%.6f,%.6f,%.6f\n",
                        rig_ms(), p.guid_hi, p.guid_lo, pos[0], pos[1], pos[2], f[0], f[1],
                        transport[1], transport[0], flags, fall_ms, f[2], f[3], f[4], f[5], f[6],
                        f[7]);
                fflush(csv);
            } else if (++dropped % 100 == 1) {
                rig_log("%ld samples dropped: the player object changed during a read", dropped);
            }
        }
        Sleep(period);
    }
}
