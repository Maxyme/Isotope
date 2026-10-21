/* No shader compiler dependency: clear, synchronized readback and presentation. */
#define COBJMACROS
#include <d3d11.h>
#include <stdio.h>
#include <string.h>
#include "graphics-abi.h"
#define CHECK(call) do { HRESULT hr=(call); if (FAILED(hr)) { fprintf(stderr,"ISOTOPE_D3D11_FAIL %s hr=%08lx\n",#call,(unsigned long)hr); return 1; } } while(0)
int main(void)
{
    WNDCLASSA wc = {0}; wc.lpfnWndProc=DefWindowProcA;
    wc.hInstance=GetModuleHandleA(NULL); wc.lpszClassName="IsotopeD3D11Probe";
    if (!RegisterClassA(&wc)) return 2;
    HWND window=CreateWindowA(wc.lpszClassName,"Isotope D3D11 probe",WS_OVERLAPPEDWINDOW|WS_VISIBLE,
        CW_USEDEFAULT,CW_USEDEFAULT,160,120,NULL,NULL,wc.hInstance,NULL);
    if (!window) return 2;
    DXGI_SWAP_CHAIN_DESC desc={0};
    desc.BufferDesc.Width=desc.BufferDesc.Height=64; desc.BufferDesc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;
    desc.SampleDesc.Count=1; desc.BufferUsage=DXGI_USAGE_RENDER_TARGET_OUTPUT;
    desc.BufferCount=1; desc.OutputWindow=window; desc.Windowed=TRUE; desc.SwapEffect=DXGI_SWAP_EFFECT_DISCARD;
    IDXGISwapChain *swap=NULL; ID3D11Device *device=NULL; ID3D11DeviceContext *context=NULL;
    D3D_FEATURE_LEVEL requested=D3D_FEATURE_LEVEL_10_0,selected=0;
    CHECK(D3D11CreateDeviceAndSwapChain(NULL,D3D_DRIVER_TYPE_HARDWARE,NULL,0,&requested,1,D3D11_SDK_VERSION,
        &desc,&swap,&device,&selected,&context));
    if (!graphics_module_abi("d3d11.dll") || !graphics_module_abi("dxgi.dll")) return 3;
    ID3D11Texture2D *back=NULL,*staging=NULL; ID3D11RenderTargetView *view=NULL;
    CHECK(IDXGISwapChain_GetBuffer(swap,0,&IID_ID3D11Texture2D,(void **)&back));
    CHECK(ID3D11Device_CreateRenderTargetView(device,(ID3D11Resource *)back,NULL,&view));
    D3D11_TEXTURE2D_DESC texture;
    ID3D11Texture2D_GetDesc(back,&texture);
    texture.Usage=D3D11_USAGE_STAGING; texture.BindFlags=0;
    texture.CPUAccessFlags=D3D11_CPU_ACCESS_READ; texture.MiscFlags=0;
    CHECK(ID3D11Device_CreateTexture2D(device,&texture,NULL,&staging));
    float green[]={0,1,0,1};
    ID3D11DeviceContext_ClearRenderTargetView(context,view,green);
    ID3D11DeviceContext_CopyResource(context,(ID3D11Resource *)staging,(ID3D11Resource *)back);
    D3D11_MAPPED_SUBRESOURCE mapped;
    CHECK(ID3D11DeviceContext_Map(context,(ID3D11Resource *)staging,0,D3D11_MAP_READ,0,&mapped));
    const unsigned char *pixel=mapped.pData;
    if (pixel[0] || pixel[1]!=255 || pixel[2] || pixel[3]!=255) return 4;
    ID3D11DeviceContext_Unmap(context,(ID3D11Resource *)staging,0);
    CHECK(IDXGISwapChain_Present(swap,0,0));
    ID3D11RenderTargetView_Release(view); ID3D11Texture2D_Release(staging); ID3D11Texture2D_Release(back);
    IDXGISwapChain_Release(swap); ID3D11DeviceContext_Release(context); ID3D11Device_Release(device);
    DestroyWindow(window);
    printf("ISOTOPE_D3D11_OK bits=%u feature=%04x readback-and-present\n",(unsigned)sizeof(void *)*8,selected);
    return 0;
}
