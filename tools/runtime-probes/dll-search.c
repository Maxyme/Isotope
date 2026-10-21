/* Verify the loaded image, not just the search environment. */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc,char **argv)
{
    if (argc!=2) return 1;
    HMODULE module=LoadLibraryA("d3d11.dll");
    if (!module) return 2;
    const BYTE *base=(const BYTE *)module;
    const IMAGE_DOS_HEADER *dos=(const IMAGE_DOS_HEADER *)base;
    const IMAGE_NT_HEADERS *nt=(const IMAGE_NT_HEADERS *)(base+dos->e_lfanew);
    unsigned long expected=strtoul(argv[1],NULL,10),actual=nt->FileHeader.TimeDateStamp;
    char path[MAX_PATH];
    if (!GetModuleFileNameA(module,path,sizeof(path))) return 3;
    printf("ISOTOPE_DLL_IMAGE timestamp=%lu expected=%lu path=%s\n",actual,expected,path);
    if (actual!=expected) return 4;
    FreeLibrary(module);
    puts("ISOTOPE_DLL_SEARCH_OK");
    return 0;
}
