/* Bounded GDI window/message test. No Steam or external application. */
#include <windows.h>
#include <stdio.h>

static unsigned paints;

static LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam)
{
    if (message == WM_PAINT)
    {
        PAINTSTRUCT paint;
        HDC dc = BeginPaint(window, &paint);
        RECT area;
        GetClientRect(window, &area);
        HBRUSH brush = CreateSolidBrush(RGB(40, 160, 90));
        if (brush && FillRect(dc, &area, brush)) ++paints;
        if (brush) DeleteObject(brush);
        EndPaint(window, &paint);
        return 0;
    }
    if (message == WM_TIMER)
    {
        DestroyWindow(window);
        return 0;
    }
    if (message == WM_DESTROY)
    {
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(window, message, wparam, lparam);
}

static int font_metrics(void)
{
    NONCLIENTMETRICSW metrics = {0};
    metrics.cbSize = sizeof(metrics);
    if (!SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0)) return 10;
    if (metrics.iCaptionHeight < 1 || metrics.iCaptionHeight > 100 ||
        metrics.iMenuHeight < 1 || metrics.iMenuHeight > 100 ||
        metrics.iSmCaptionHeight < 1 || metrics.iSmCaptionHeight > 100) return 11;
    HDC dc = GetDC(NULL);
    HFONT font = CreateFontW(-13,0,0,0,FW_NORMAL,0,0,0,DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,CLIP_DEFAULT_PRECIS,DEFAULT_QUALITY,DEFAULT_PITCH,L"Tahoma");
    if (!dc || !font) return 12;
    HGDIOBJ previous = SelectObject(dc, font);
    TEXTMETRICW text = {0};
    BOOL measured = GetTextMetricsW(dc, &text);
    SelectObject(dc, previous);
    DeleteObject(font);
    ReleaseDC(NULL, dc);
    if (!measured || text.tmHeight < 1 || text.tmHeight > 100) return 13;
    printf("FONT_METRICS caption=%d menu=%d small=%d text=%ld\n",
        metrics.iCaptionHeight, metrics.iMenuHeight, metrics.iSmCaptionHeight, text.tmHeight);
    return 0;
}

int wmain(int argc, WCHAR **argv)
{
    int checked = font_metrics();
    if (checked) { printf("FONT_METRICS_FAILED stage=%d\n", checked); return checked; }

    HINSTANCE instance = GetModuleHandleW(NULL);
    WNDCLASSW cls = {0};
    cls.lpfnWndProc = window_proc;
    cls.hInstance = instance;
    cls.lpszClassName = L"NucleonFexWindowProbe";
    if (!RegisterClassW(&cls)) return 1;
    RECT wanted = {0, 0, 480, 240};
    if (!AdjustWindowRectEx(&wanted, WS_OVERLAPPEDWINDOW, FALSE, 0)) return 14;
    HWND window = CreateWindowExW(0, cls.lpszClassName, L"Nucleon ARM64/FEX test",
        WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, wanted.right-wanted.left, wanted.bottom-wanted.top, NULL, NULL, instance, NULL);
    if (!window) return 2;
    RECT client = {0};
    if (!GetClientRect(window, &client) || client.right != 480 || client.bottom != 240) {
        printf("WINDOW_METRICS_FAILED client=%ldx%ld\n", client.right, client.bottom);
        DestroyWindow(window);
        return 15;
    }
    printf("NUCLEON_FEX_FONT_METRICS_OK client=%ldx%ld\n", client.right, client.bottom);
    if (argc > 1 && !wcscmp(argv[1], L"--metrics-only")) { DestroyWindow(window); return 0; }
    ShowWindow(window, SW_SHOWNOACTIVATE);
    UpdateWindow(window);
    if (!SetTimer(window, 1, 1000, NULL)) return 3;
    MSG message;
    int status;
    while ((status = GetMessageW(&message, NULL, 0, 0)) > 0)
    {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    if (status < 0 || !paints) return 4;
    printf("NUCLEON_FEX_WINDOW_OK paints=%u\n", paints);
    return 0;
}
