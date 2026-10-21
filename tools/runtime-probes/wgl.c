/* Owned WGL regression: client object pointers, sharing, readback and present. */
#include <windows.h>
#include <GL/gl.h>
#include <stdio.h>
typedef HGLRC (WINAPI *create_context_fn)(HDC, HGLRC, const int *);
static void pump(void)
{
    MSG message;
    while (PeekMessageA(&message,NULL,0,0,PM_REMOVE)) { TranslateMessage(&message); DispatchMessageA(&message); }
    Sleep(20);
}
static int fail(const char *stage) { fprintf(stderr,"ISOTOPE_WGL_FAIL %s error=%lu\n",stage,GetLastError()); return 1; }
int main(void)
{
    WNDCLASSA wc = {0};
    wc.style = CS_OWNDC; wc.lpfnWndProc = DefWindowProcA;
    wc.hInstance = GetModuleHandleA(NULL); wc.lpszClassName = "IsotopeWGLProbe";
    if (!RegisterClassA(&wc)) return fail("class");
    HWND window = CreateWindowA(wc.lpszClassName,"Isotope WGL probe",WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,CW_USEDEFAULT,160,120,NULL,NULL,wc.hInstance,NULL);
    if (!window) return fail("window");
    UpdateWindow(window);
    for (unsigned i=0;i<10;i++) pump();
    HDC dc = GetDC(window);
    PIXELFORMATDESCRIPTOR format = {0};
    format.nSize = sizeof(format); format.nVersion = 1;
    format.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
    format.iPixelType = PFD_TYPE_RGBA; format.cColorBits = 32;
    int index = ChoosePixelFormat(dc,&format);
    if (!index || !SetPixelFormat(dc,index,&format)) return fail("pixel-format");
    HGLRC legacy = wglCreateContext(dc);
    if (!legacy || !wglMakeCurrent(dc,legacy)) return fail("legacy-context");
    create_context_fn create = (create_context_fn)wglGetProcAddress("wglCreateContextAttribsARB");
    if (!create) return fail("extension");
    const int attributes[] = {0x2091,3,0x2092,2,0x9126,1,0};
    HGLRC core = create(dc,NULL,attributes);
    if (!core || !wglMakeCurrent(dc,core)) return fail("core-context");
    HGLRC shared = create(dc,core,attributes);
    if (!shared || !wglMakeCurrent(dc,shared)) return fail("shared-context");
    printf("ISOTOPE_WGL_DEVICE bits=%u vendor=%s renderer=%s version=%s\n",
        (unsigned)sizeof(void *)*8,glGetString(GL_VENDOR),glGetString(GL_RENDERER),glGetString(GL_VERSION));
    unsigned char pixel[4] = {0};
    GLenum error=GL_NO_ERROR;
    unsigned attempt;
    /* Cocoa drawable realization and resize are asynchronous. Pump messages and
     * retry a bounded number of frames, rather than testing an unready surface. */
    for (attempt=0;attempt<100;attempt++)
    {
        glViewport(0,0,64,64); glClearColor(0,1,0,1); glClear(GL_COLOR_BUFFER_BIT);
        glReadPixels(16,16,1,1,GL_RGBA,GL_UNSIGNED_BYTE,pixel);
        error=glGetError();
        if (error != GL_NO_ERROR || (!pixel[0] && pixel[1]==255 && !pixel[2])) break;
        if (!SwapBuffers(dc)) return fail("realize-drawable");
        pump();
    }
    if (error != GL_NO_ERROR || attempt==100) {
        fprintf(stderr,"ISOTOPE_WGL_READBACK error=%04x rgba=%u,%u,%u,%u\n",error,pixel[0],pixel[1],pixel[2],pixel[3]);
        return fail("readback");
    }
    if (!SwapBuffers(dc)) return fail("present");
    if (!wglMakeCurrent(NULL,NULL) || !wglDeleteContext(shared) || !wglDeleteContext(core)
        || !wglDeleteContext(legacy)) return fail("delete-context");
    ReleaseDC(window,dc); DestroyWindow(window);
    puts("ISOTOPE_WGL_OK context-share-readback-present");
    return 0;
}
