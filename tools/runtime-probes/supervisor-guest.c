/* Both the initial Wine guest and CreateProcess child must supervise themselves. */
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(int argc,char **argv)
{
    setbuf(stdout,NULL);
    if (argc==4 && !strcmp(argv[1],"--observe"))
    {
        HANDLE processes[2];
        for (unsigned i=0;i<2;i++) {
            processes[i]=OpenProcess(SYNCHRONIZE|PROCESS_QUERY_LIMITED_INFORMATION,FALSE,strtoul(argv[i+2],NULL,10));
            if (!processes[i] || WaitForSingleObject(processes[i],0)!=WAIT_TIMEOUT) return 4;
        }
        puts("ISOTOPE_OBSERVER_READY");
        if (WaitForMultipleObjects(2,processes,TRUE,10000)!=WAIT_OBJECT_0) return 5;
        for (unsigned i=0;i<2;i++) CloseHandle(processes[i]);
        puts("ISOTOPE_GUEST_TREE_EXIT_OK");
        return 0;
    }
    if (argc==1)
    {
        char exe[MAX_PATH],command[MAX_PATH+16];
        STARTUPINFOA startup={sizeof(startup)}; PROCESS_INFORMATION child;
        if (!GetModuleFileNameA(NULL,exe,sizeof(exe))) return 1;
        snprintf(command,sizeof(command),"\"%s\" --child",exe);
        if (!CreateProcessA(NULL,command,NULL,NULL,TRUE,0,NULL,NULL,&startup,&child)) return 2;
        CloseHandle(child.hThread); CloseHandle(child.hProcess);
    }
    char line[128]; DWORD written;
    int length=snprintf(line,sizeof(line),"ISOTOPE_GUEST_READY role=%s pid=%lu\n",argc==1?"parent":"child",GetCurrentProcessId());
    if (!WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),line,length,&written,NULL) || written!=(DWORD)length) return 6;
    Sleep(90000);
    return 3; /* owner-loss should terminate us long before this */
}
