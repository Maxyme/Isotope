/* Cold security/RPC startup regression; no credentials or network required. */
#define SECURITY_WIN32
#include <windows.h>
#include <security.h>
#include <winsvc.h>
#include <stdio.h>

int main(void)
{
    HMODULE module = LoadLibraryW(L"secur32.dll");
    if (!module) {
        printf("SECURITY_RPC_FAIL LoadLibraryW(secur32) error=%lu\n", GetLastError());
        return 1;
    }
    ENUMERATE_SECURITY_PACKAGES_FN_W enumerate =
        (ENUMERATE_SECURITY_PACKAGES_FN_W)GetProcAddress(module, "EnumerateSecurityPackagesW");
    FREE_CONTEXT_BUFFER_FN release =
        (FREE_CONTEXT_BUFFER_FN)GetProcAddress(module, "FreeContextBuffer");
    if (!enumerate || !release) return 2;
    ULONG count = 0;
    PSecPkgInfoW packages = NULL;
    SECURITY_STATUS status = enumerate(&count, &packages);
    if (status != SEC_E_OK || !count || !packages) {
        printf("SECURITY_RPC_FAIL EnumerateSecurityPackagesW status=%lx count=%lu\n",
               (unsigned long)status, count);
        return 3;
    }
    status = release(packages);
    if (status != SEC_E_OK) return 4;
    SC_HANDLE manager = OpenSCManagerW(NULL, NULL, SC_MANAGER_CONNECT);
    if (!manager) {
        printf("SECURITY_RPC_FAIL OpenSCManagerW error=%lu\n", GetLastError());
        return 5;
    }
    if (!CloseServiceHandle(manager)) return 6;
    FreeLibrary(module);
    puts("ISOTOPE_SECURITY_RPC_OK");
    return 0;
}
