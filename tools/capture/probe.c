/* probe: tells whether another process can read the running client's memory.
 *
 * The rigs read the client from inside its own process. Where the client runs under a layer that
 * denies other processes access to it, that is the only way in; where it does not, an ordinary
 * outside reader works too. Run this beside the client, in the same Wine prefix or Windows
 * login: it finds the process by its executable name (WoW.exe unless an argument names
 * another), opens it with read rights, and reads the first 16 bytes of the image at 0x400000.
 * "MZ" there means outside reads work.
 *
 * Exit status: 0 read works, 1 no such process, 2 the process cannot be opened, 3 it opens but
 * the read fails. */
#include <windows.h>
#include <tlhelp32.h>
#include <stdio.h>

static DWORD find_process(const char *exe) {
    HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snap == INVALID_HANDLE_VALUE) return 0;
    PROCESSENTRY32 pe;
    pe.dwSize = sizeof pe;
    DWORD pid = 0;
    for (BOOL more = Process32First(snap, &pe); more && !pid; more = Process32Next(snap, &pe))
        if (_stricmp(pe.szExeFile, exe) == 0) pid = pe.th32ProcessID;
    CloseHandle(snap);
    return pid;
}

int main(int argc, char **argv) {
    const char *exe = argc > 1 ? argv[1] : "WoW.exe";
    DWORD pid = find_process(exe);
    if (!pid) {
        printf("probe: no process named %s\n", exe);
        return 1;
    }
    printf("probe: %s is process %lu\n", exe, pid);

    static const DWORD rights[] = {PROCESS_VM_READ, PROCESS_VM_READ | PROCESS_QUERY_INFORMATION,
                                   PROCESS_ALL_ACCESS};
    static const char *const names[] = {"VM_READ", "VM_READ | QUERY_INFORMATION", "ALL_ACCESS"};
    HANDLE h = NULL;
    for (int i = 0; i < 3 && !h; i++) {
        h = OpenProcess(rights[i], FALSE, pid);
        printf("probe: OpenProcess(%s): %s (error %lu)\n", names[i], h ? "opened" : "denied",
               h ? 0 : GetLastError());
    }
    if (!h) return 2;

    unsigned char head[16] = {0};
    SIZE_T got = 0;
    BOOL ok = ReadProcessMemory(h, (LPCVOID)0x400000, head, sizeof head, &got);
    DWORD error = ok ? 0 : GetLastError();
    CloseHandle(h);
    if (!ok || got != sizeof head) {
        printf("probe: ReadProcessMemory(0x400000) failed (error %lu): use a rig\n", error);
        return 3;
    }
    printf("probe: 0x400000:");
    for (int i = 0; i < 16; i++) printf(" %02x", head[i]);
    printf("\nprobe: %s\n", head[0] == 'M' && head[1] == 'Z' ? "MZ: outside reads work"
                                                             : "no MZ at the image base");
    return 0;
}
