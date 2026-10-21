#define WIN32_LEAN_AND_MEAN
#define VK_USE_PLATFORM_WIN32_KHR
#define VK_NO_PROTOTYPES
#include <windows.h>
#include <vulkan/vulkan.h>
#include <stdio.h>
#include <stdlib.h>

#define GET(name) PFN_vk##name name = (PFN_vk##name)get(instance, "vk" #name); if (!name) return 2
#define OK(call) do { VkResult r = (call); if (r != VK_SUCCESS) { printf("NUCLEON_VULKAN_FAIL %s %d\n", #call, r); return 3; } } while (0)
static LRESULT CALLBACK procedure(HWND w, UINT m, WPARAM a, LPARAM b) { return DefWindowProcW(w,m,a,b); }
int main(void) {
    HMODULE dll = LoadLibraryW(L"vulkan-1.dll");
    if (!dll) { printf("NUCLEON_VULKAN_FAIL load %lu\n", GetLastError()); return 1; }
    PFN_vkGetInstanceProcAddr get = (PFN_vkGetInstanceProcAddr)(void *)GetProcAddress(dll,"vkGetInstanceProcAddr");
    if (!get) return 2;
    VkInstance instance = VK_NULL_HANDLE;
    GET(CreateInstance);
    const char *extensions[] = {VK_KHR_SURFACE_EXTENSION_NAME, VK_KHR_WIN32_SURFACE_EXTENSION_NAME};
    VkApplicationInfo app = {.sType=VK_STRUCTURE_TYPE_APPLICATION_INFO,.pApplicationName="Nucleon FEX Vulkan probe",.apiVersion=VK_API_VERSION_1_3};
    VkInstanceCreateInfo info = {.sType=VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,.pApplicationInfo=&app,
                                .enabledExtensionCount=2,.ppEnabledExtensionNames=extensions};
    OK(CreateInstance(&info,NULL,&instance));
    GET(EnumeratePhysicalDevices); GET(GetPhysicalDeviceProperties); GET(GetPhysicalDeviceFeatures2); GET(GetPhysicalDeviceProperties2);
    GET(GetPhysicalDeviceQueueFamilyProperties); GET(CreateDevice); GET(DestroyDevice); GET(DestroyInstance);
    GET(CreateWin32SurfaceKHR); GET(DestroySurfaceKHR); GET(GetPhysicalDeviceSurfaceSupportKHR);
    VkPhysicalDevice physical = VK_NULL_HANDLE; uint32_t count=1;
    OK(EnumeratePhysicalDevices(instance,&count,&physical)); if (!count) return 4;
    VkPhysicalDeviceProperties properties; GetPhysicalDeviceProperties(physical,&properties);
    VkPhysicalDeviceDriverProperties driver={.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_DRIVER_PROPERTIES};
    VkPhysicalDeviceProperties2 details={.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2,.pNext=&driver};
    GetPhysicalDeviceProperties2(physical,&details);
    printf("NUCLEON_VULKAN_DRIVER %s %s\n",driver.driverName,driver.driverInfo);
    printf("NUCLEON_VULKAN_DEVICE %s api=%u.%u.%u\n",properties.deviceName,
           VK_VERSION_MAJOR(properties.apiVersion),VK_VERSION_MINOR(properties.apiVersion),VK_VERSION_PATCH(properties.apiVersion));
    VkPhysicalDeviceMutableDescriptorTypeFeaturesEXT mut = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_MUTABLE_DESCRIPTOR_TYPE_FEATURES_EXT};
    VkPhysicalDeviceTransformFeedbackFeaturesEXT xfb = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_TRANSFORM_FEEDBACK_FEATURES_EXT,.pNext=&mut};
    VkPhysicalDeviceRobustness2FeaturesEXT robust = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ROBUSTNESS_2_FEATURES_EXT,.pNext=&xfb};
    VkPhysicalDeviceVulkan13Features f13 = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_3_FEATURES,.pNext=&robust};
    VkPhysicalDeviceVulkan12Features f12 = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_2_FEATURES,.pNext=&f13};
    VkPhysicalDeviceVulkan11Features f11 = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_1_FEATURES,.pNext=&f12};
    VkPhysicalDeviceFeatures2 f2 = {.sType=VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2,.pNext=&f11};
    GetPhysicalDeviceFeatures2(physical,&f2);
    printf("NUCLEON_VULKAN_FEATURES robust=%u null=%u xfb=%u mutable=%u bda=%u geometry=%u\n",robust.robustBufferAccess2,robust.nullDescriptor,xfb.transformFeedback,mut.mutableDescriptorType,f12.bufferDeviceAddress,f2.features.geometryShader);
    if (!robust.robustBufferAccess2 || !robust.nullDescriptor || !xfb.transformFeedback || !mut.mutableDescriptorType || !f12.bufferDeviceAddress)
        { puts("NUCLEON_VULKAN_FAIL required-features"); return 5; }
    WNDCLASSW wc = {0}; wc.lpfnWndProc=procedure; wc.hInstance=GetModuleHandleW(NULL); wc.lpszClassName=L"NucleonVulkanProbe";
    RegisterClassW(&wc);
    HWND window = CreateWindowW(wc.lpszClassName,L"Nucleon ARM64 Vulkan probe",WS_OVERLAPPEDWINDOW,
                                 CW_USEDEFAULT,CW_USEDEFAULT,160,120,NULL,NULL,wc.hInstance,NULL);
    if (!window) return 6;
    VkWin32SurfaceCreateInfoKHR si={.sType=VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR,.hinstance=wc.hInstance,.hwnd=window};
    VkSurfaceKHR surface; OK(CreateWin32SurfaceKHR(instance,&si,NULL,&surface));
    GetPhysicalDeviceQueueFamilyProperties(physical,&count,NULL);
    VkQueueFamilyProperties *families=calloc(count,sizeof(*families));
    if (!families) return 7;
    GetPhysicalDeviceQueueFamilyProperties(physical,&count,families);
    uint32_t family=count;
    for (uint32_t i=0;i<count;i++) { VkBool32 supported=VK_FALSE;
        OK(GetPhysicalDeviceSurfaceSupportKHR(physical,i,surface,&supported));
        if ((families[i].queueFlags & VK_QUEUE_GRAPHICS_BIT) && supported) { family=i; break; } }
    free(families); if (family==count) return 8;
    float priority=1;
    VkDeviceQueueCreateInfo qi={.sType=VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,.queueFamilyIndex=family,.queueCount=1,.pQueuePriorities=&priority};
    const char *de[]={VK_KHR_SWAPCHAIN_EXTENSION_NAME,VK_EXT_ROBUSTNESS_2_EXTENSION_NAME,VK_EXT_TRANSFORM_FEEDBACK_EXTENSION_NAME,VK_EXT_MUTABLE_DESCRIPTOR_TYPE_EXTENSION_NAME};
    VkDeviceCreateInfo di={.sType=VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,.pNext=&f2,.queueCreateInfoCount=1,.pQueueCreateInfos=&qi,.enabledExtensionCount=4,.ppEnabledExtensionNames=de};
    VkDevice device; OK(CreateDevice(physical,&di,NULL,&device));
    /* Exercise the WoW64 swapchain ABI and actual GPU output, without shaders. */
    ShowWindow(window,SW_SHOW);
    GET(GetPhysicalDeviceSurfaceCapabilitiesKHR); GET(GetPhysicalDeviceSurfaceFormatsKHR);
    GET(GetPhysicalDeviceMemoryProperties); GET(CreateSwapchainKHR); GET(GetSwapchainImagesKHR);
    GET(DestroySwapchainKHR); GET(GetDeviceQueue); GET(CreateCommandPool); GET(DestroyCommandPool);
    GET(AllocateCommandBuffers); GET(BeginCommandBuffer); GET(EndCommandBuffer);
    GET(CmdPipelineBarrier); GET(CmdClearColorImage); GET(CmdCopyImageToBuffer);
    GET(CreateBuffer); GET(GetBufferMemoryRequirements); GET(AllocateMemory); GET(BindBufferMemory);
    GET(MapMemory); GET(UnmapMemory); GET(DestroyBuffer); GET(FreeMemory);
    GET(CreateSemaphore); GET(DestroySemaphore); GET(CreateFence); GET(DestroyFence);
    GET(AcquireNextImageKHR); GET(QueueSubmit); GET(QueuePresentKHR); GET(WaitForFences); GET(DeviceWaitIdle);
    VkSurfaceCapabilitiesKHR caps;
    OK(GetPhysicalDeviceSurfaceCapabilitiesKHR(physical,surface,&caps));
    VkImageUsageFlags usage=VK_IMAGE_USAGE_TRANSFER_SRC_BIT|VK_IMAGE_USAGE_TRANSFER_DST_BIT;
    if ((caps.supportedUsageFlags&usage)!=usage) return 9;
    uint32_t format_count=0;
    OK(GetPhysicalDeviceSurfaceFormatsKHR(physical,surface,&format_count,NULL));
    VkSurfaceFormatKHR *formats=calloc(format_count,sizeof(*formats)),format={0};
    if (!formats) return 9;
    OK(GetPhysicalDeviceSurfaceFormatsKHR(physical,surface,&format_count,formats));
    for (uint32_t i=0;i<format_count;i++)
        if (formats[i].format==VK_FORMAT_B8G8R8A8_UNORM || formats[i].format==VK_FORMAT_R8G8B8A8_UNORM) { format=formats[i]; break; }
    free(formats); if (!format.format) return 9;
    VkExtent2D extent=caps.currentExtent;
    if (extent.width==UINT32_MAX) { extent.width=160; extent.height=120; }
    VkCompositeAlphaFlagBitsKHR alpha=VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    if (!(caps.supportedCompositeAlpha&alpha))
        alpha=(VkCompositeAlphaFlagBitsKHR)(caps.supportedCompositeAlpha&-(int32_t)caps.supportedCompositeAlpha);
    VkSwapchainCreateInfoKHR sci={.sType=VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,.surface=surface,
        .minImageCount=caps.minImageCount,.imageFormat=format.format,.imageColorSpace=format.colorSpace,
        .imageExtent=extent,.imageArrayLayers=1,.imageUsage=usage,.imageSharingMode=VK_SHARING_MODE_EXCLUSIVE,
        .preTransform=caps.currentTransform,.compositeAlpha=alpha,.presentMode=VK_PRESENT_MODE_FIFO_KHR,.clipped=VK_TRUE};
    VkSwapchainKHR swap; OK(CreateSwapchainKHR(device,&sci,NULL,&swap));
    uint32_t image_count=0; OK(GetSwapchainImagesKHR(device,swap,&image_count,NULL));
    VkImage *images=calloc(image_count,sizeof(*images)); if (!images) return 9;
    OK(GetSwapchainImagesKHR(device,swap,&image_count,images));
    VkSemaphoreCreateInfo sem_info={.sType=VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO};
    VkSemaphore acquired,rendered;
    OK(CreateSemaphore(device,&sem_info,NULL,&acquired)); OK(CreateSemaphore(device,&sem_info,NULL,&rendered));
    uint32_t image; OK(AcquireNextImageKHR(device,swap,10000000000ull,acquired,VK_NULL_HANDLE,&image));
    VkDeviceSize bytes=(VkDeviceSize)extent.width*extent.height*4;
    VkBufferCreateInfo bi={.sType=VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,.size=bytes,.usage=VK_BUFFER_USAGE_TRANSFER_DST_BIT,
        .sharingMode=VK_SHARING_MODE_EXCLUSIVE};
    VkBuffer buffer; OK(CreateBuffer(device,&bi,NULL,&buffer));
    VkMemoryRequirements requirements; GetBufferMemoryRequirements(device,buffer,&requirements);
    VkPhysicalDeviceMemoryProperties memory_properties; GetPhysicalDeviceMemoryProperties(physical,&memory_properties);
    uint32_t memory_type=memory_properties.memoryTypeCount;
    VkMemoryPropertyFlags wanted=VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT|VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
    for (uint32_t i=0;i<memory_properties.memoryTypeCount;i++)
        if ((requirements.memoryTypeBits&(1u<<i)) && (memory_properties.memoryTypes[i].propertyFlags&wanted)==wanted) { memory_type=i; break; }
    if (memory_type==memory_properties.memoryTypeCount) return 10;
    VkMemoryAllocateInfo ai={.sType=VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,.allocationSize=requirements.size,.memoryTypeIndex=memory_type};
    VkDeviceMemory memory; OK(AllocateMemory(device,&ai,NULL,&memory)); OK(BindBufferMemory(device,buffer,memory,0));
    VkCommandPoolCreateInfo pi={.sType=VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,.queueFamilyIndex=family};
    VkCommandPool pool; OK(CreateCommandPool(device,&pi,NULL,&pool));
    VkCommandBufferAllocateInfo cai={.sType=VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,.commandPool=pool,
        .level=VK_COMMAND_BUFFER_LEVEL_PRIMARY,.commandBufferCount=1};
    VkCommandBuffer command; OK(AllocateCommandBuffers(device,&cai,&command));
    VkCommandBufferBeginInfo begin={.sType=VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO}; OK(BeginCommandBuffer(command,&begin));
    VkImageMemoryBarrier barrier={.sType=VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,.oldLayout=VK_IMAGE_LAYOUT_UNDEFINED,
        .newLayout=VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,.dstAccessMask=VK_ACCESS_TRANSFER_WRITE_BIT,
        .srcQueueFamilyIndex=VK_QUEUE_FAMILY_IGNORED,.dstQueueFamilyIndex=VK_QUEUE_FAMILY_IGNORED,
        .image=images[image],.subresourceRange={VK_IMAGE_ASPECT_COLOR_BIT,0,1,0,1}};
    CmdPipelineBarrier(command,VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,VK_PIPELINE_STAGE_TRANSFER_BIT,0,0,NULL,0,NULL,1,&barrier);
    VkClearColorValue green={.float32={0,1,0,1}};
    CmdClearColorImage(command,images[image],VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,&green,1,&barrier.subresourceRange);
    barrier.oldLayout=VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL; barrier.newLayout=VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
    barrier.srcAccessMask=VK_ACCESS_TRANSFER_WRITE_BIT; barrier.dstAccessMask=VK_ACCESS_TRANSFER_READ_BIT;
    CmdPipelineBarrier(command,VK_PIPELINE_STAGE_TRANSFER_BIT,VK_PIPELINE_STAGE_TRANSFER_BIT,0,0,NULL,0,NULL,1,&barrier);
    VkBufferImageCopy region={.imageSubresource={VK_IMAGE_ASPECT_COLOR_BIT,0,0,1},.imageExtent={extent.width,extent.height,1}};
    CmdCopyImageToBuffer(command,images[image],VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,buffer,1,&region);
    barrier.oldLayout=VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL; barrier.newLayout=VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;
    barrier.srcAccessMask=VK_ACCESS_TRANSFER_READ_BIT; barrier.dstAccessMask=0;
    CmdPipelineBarrier(command,VK_PIPELINE_STAGE_TRANSFER_BIT,VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,0,0,NULL,0,NULL,1,&barrier);
    VkBufferMemoryBarrier host={.sType=VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,.srcAccessMask=VK_ACCESS_TRANSFER_WRITE_BIT,
        .dstAccessMask=VK_ACCESS_HOST_READ_BIT,.srcQueueFamilyIndex=VK_QUEUE_FAMILY_IGNORED,.dstQueueFamilyIndex=VK_QUEUE_FAMILY_IGNORED,
        .buffer=buffer,.offset=0,.size=VK_WHOLE_SIZE};
    CmdPipelineBarrier(command,VK_PIPELINE_STAGE_TRANSFER_BIT,VK_PIPELINE_STAGE_HOST_BIT,0,0,NULL,1,&host,0,NULL);
    OK(EndCommandBuffer(command));
    VkQueue queue; GetDeviceQueue(device,family,0,&queue);
    VkPipelineStageFlags stage=VK_PIPELINE_STAGE_TRANSFER_BIT;
    VkSubmitInfo submit={.sType=VK_STRUCTURE_TYPE_SUBMIT_INFO,.waitSemaphoreCount=1,.pWaitSemaphores=&acquired,
        .pWaitDstStageMask=&stage,.commandBufferCount=1,.pCommandBuffers=&command,.signalSemaphoreCount=1,.pSignalSemaphores=&rendered};
    VkFenceCreateInfo fi={.sType=VK_STRUCTURE_TYPE_FENCE_CREATE_INFO}; VkFence fence;
    OK(CreateFence(device,&fi,NULL,&fence)); OK(QueueSubmit(queue,1,&submit,fence));
    OK(WaitForFences(device,1,&fence,VK_TRUE,10000000000ull));
    void *mapped; OK(MapMemory(device,memory,0,bytes,0,&mapped));
    const unsigned char *pixel=mapped;
    if (pixel[0] || pixel[1]!=255 || pixel[2] || pixel[3]!=255) { puts("NUCLEON_VULKAN_FAIL pixel-readback"); return 11; }
    UnmapMemory(device,memory);
    VkPresentInfoKHR present={.sType=VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,.waitSemaphoreCount=1,.pWaitSemaphores=&rendered,
        .swapchainCount=1,.pSwapchains=&swap,.pImageIndices=&image};
    OK(QueuePresentKHR(queue,&present)); OK(DeviceWaitIdle(device));
    printf("ISOTOPE_VULKAN_PRESENT_OK bits=%u extent=%ux%u readback-and-present\n",(unsigned)sizeof(void *)*8,extent.width,extent.height);
    DestroyFence(device,fence,NULL); DestroySemaphore(device,rendered,NULL); DestroySemaphore(device,acquired,NULL);
    DestroyCommandPool(device,pool,NULL); DestroyBuffer(device,buffer,NULL); FreeMemory(device,memory,NULL);
    free(images); DestroySwapchainKHR(device,swap,NULL);
    DestroyDevice(device,NULL); DestroySurfaceKHR(instance,surface,NULL); DestroyWindow(window); DestroyInstance(instance,NULL);
    puts("NUCLEON_VULKAN_OK device-and-win32-surface"); return 0;
}
