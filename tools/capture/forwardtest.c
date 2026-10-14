/* forwardtest: checks a built rig in place, without the client.
 *
 * Like the client, the test imports imm32.dll by name. Run it from a directory holding the rig's
 * imm32.dll and imm32_orig.dll, with the rig loaded in imm32's place as the launcher loads it
 * (README.md, "Testing a build"). It passes when:
 *   1. the imm32.dll the process loaded is the one in the test's directory: the rig took the slot;
 *   2. imm32_orig.dll is loaded, and every function the client imports from imm32 resolves into
 *      it: the rig forwards them, it does not implement them;
 *   3. ImmSetCompositionFontW, which the client does not import but user32 can load on demand,
 *      resolves into imm32_orig.dll too;
 *   4. the rig's log says it found no WoW.exe 1.12.1.5875 here and did nothing: the build check
 *      refuses any other process.
 * It exits 0 on a pass and prints the first failure otherwise. */
#include <windows.h>
#include <imm.h>
#include <stdio.h>
#include <string.h>

static const char *const client_imports[] = {
    "ImmAssociateContext",      "ImmAssociateContextEx", "ImmGetCandidateListA",
    "ImmGetCompositionStringA", "ImmGetContext",         "ImmGetConversionStatus",
    "ImmNotifyIME",             "ImmReleaseContext",     "ImmSetConversionStatus",
    "ImmSetCompositionFontW",
};

static int fail(const char *what) {
    printf("forwardtest: FAIL: %s (last error %lu)\n", what, GetLastError());
    return 1;
}

int main(void) {
    char dir[MAX_PATH], path[MAX_PATH + 32], loaded[MAX_PATH];
    DWORD n = GetModuleFileNameA(NULL, dir, sizeof dir);
    if (n == 0 || n >= sizeof dir) return fail("cannot read the test's own path");
    *strrchr(dir, '\\') = 0;

    /* The static import: the loader resolved it before main, through whichever imm32 it chose. */
    HIMC context = ImmGetContext(NULL);
    (void)context;
    HMODULE rig = GetModuleHandleA("imm32.dll");
    if (!rig || !GetModuleFileNameA(rig, loaded, sizeof loaded))
        return fail("no imm32.dll is loaded");
    snprintf(path, sizeof path, "%s\\imm32.dll", dir);
    if (_stricmp(loaded, path) != 0) {
        printf("forwardtest: imm32.dll is %s, not %s\n", loaded, path);
        return fail("the system's imm32 took the slot: check the DLL override and KnownDLLs");
    }
    HMODULE orig = GetModuleHandleA("imm32_orig.dll");
    if (!orig) return fail("imm32_orig.dll is not loaded");
    printf("forwardtest: imm32.dll is the rig at %s\n", loaded);

    for (size_t i = 0; i < sizeof client_imports / sizeof *client_imports; i++) {
        FARPROC f = GetProcAddress(rig, client_imports[i]);
        HMODULE owner = NULL;
        if (!f) {
            printf("forwardtest: %s does not resolve\n", client_imports[i]);
            return fail("an import is missing");
        }
        GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                               GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                           (LPCSTR)f, &owner);
        if (owner != orig) {
            printf("forwardtest: %s resolves to %p, outside imm32_orig.dll\n", client_imports[i],
                   (void *)f);
            return fail("an import is not forwarded");
        }
    }
    printf("forwardtest: %u functions forward to imm32_orig.dll\n",
           (unsigned)(sizeof client_imports / sizeof *client_imports));

    /* The rig's thread writes its log as soon as it runs; give it two seconds. A log older than
     * this process is from an earlier run and does not count. */
    FILETIME created, unused;
    GetProcessTimes(GetCurrentProcess(), &created, &unused, &unused, &unused);
    char logdir[MAX_PATH];
    n = GetEnvironmentVariableA("BENILLA_CAPTURE_DIR", logdir, sizeof logdir);
    if (n == 0 || n >= sizeof logdir) strcpy(logdir, dir);
    char text[4096];
    for (int wait = 0; wait < 20; wait++) {
        Sleep(100);
        static const char *const logs[] = {"player.log", "hook.log"};
        for (int l = 0; l < 2; l++) {
            snprintf(path, sizeof path, "%s\\%s", logdir, logs[l]);
            WIN32_FILE_ATTRIBUTE_DATA info;
            if (!GetFileAttributesExA(path, GetFileExInfoStandard, &info) ||
                CompareFileTime(&info.ftLastWriteTime, &created) < 0)
                continue;
            FILE *f = fopen(path, "r");
            if (!f) continue;
            size_t got = fread(text, 1, sizeof text - 1, f);
            fclose(f);
            text[got] = 0;
            if (strstr(text, "not WoW.exe 1.12.1.5875") && strstr(text, "rig stopped")) {
                printf("forwardtest: %s refuses this process\nforwardtest: PASS\n", logs[l]);
                return 0;
            }
        }
    }
    return fail("no rig log from this run says the build check refused this process");
}
