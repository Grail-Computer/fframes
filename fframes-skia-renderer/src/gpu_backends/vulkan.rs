use crate::renderer_backend::SkiaFFramesRenderer;
use fframes_renderer::FFramesRendererResult;
use skia_safe::gpu;

impl SkiaFFramesRenderer {
    #[cfg(feature = "vulkan")]
    /// Provides default implementation of Vulkan based backend, which is possible to replicate
    /// manually with `new_gpu` method.
    pub fn new_vulkan(width: usize, height: usize) -> FFramesRendererResult<Self> {
        use ash::{vk, Entry};
        use skia_safe::{
            gpu::{backend_render_targets, vk::BackendContext, SurfaceOrigin},
            ColorType,
        };

        // Initialize Vulkan
        let entry = unsafe { Entry::load() }.map_err(|e| {
            fframes_renderer::FFramesRendererError::Skia(
                format!("Failed to create Vulkan entry: {}", e)
            )
        })?;

        // Create Vulkan instance
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: std::ptr::null(),
            p_application_name: std::ptr::null(),
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: std::ptr::null(),
            engine_version: vk::make_api_version(0, 1, 0, 0),
            api_version: vk::API_VERSION_1_1,
            ..Default::default()
        };

        let instance_create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::InstanceCreateFlags::empty(),
            p_application_info: &app_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: 0,
            pp_enabled_extension_names: std::ptr::null(),
            ..Default::default()
        };

        let instance = unsafe {
            entry.create_instance(&instance_create_info, None)
        }.map_err(|e| {
            fframes_renderer::FFramesRendererError::Skia(
                format!("Failed to create Vulkan instance: {}", e)
            )
        })?;

        // Get physical device
        let physical_devices = unsafe {
            instance.enumerate_physical_devices()
        }.map_err(|e| {
            fframes_renderer::FFramesRendererError::Skia(
                format!("Failed to enumerate physical devices: {}", e)
            )
        })?;

        let physical_device = physical_devices.first().ok_or_else(|| {
            fframes_renderer::FFramesRendererError::Skia(
                "No Vulkan physical device found".to_string()
            )
        })?;

        // Create logical device
        let queue_family_properties = unsafe {
            instance.get_physical_device_queue_family_properties(*physical_device)
        };

        let queue_family_index = queue_family_properties
            .iter()
            .position(|props| props.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .ok_or_else(|| {
                fframes_renderer::FFramesRendererError::Skia(
                    "No graphics queue family found".to_string()
                )
            })? as u32;

        let priorities = [1.0f32];
        let device_queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index,
            queue_count: 1,
            p_queue_priorities: priorities.as_ptr(),
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: 1,
            p_queue_create_infos: &device_queue_create_info,
            enabled_extension_count: 0,
            pp_enabled_extension_names: std::ptr::null(),
            p_enabled_features: std::ptr::null(),
            ..Default::default()
        };

        let device = unsafe {
            instance.create_device(*physical_device, &device_create_info, None)
        }.map_err(|e| {
            fframes_renderer::FFramesRendererError::Skia(
                format!("Failed to create logical device: {}", e)
            )
        })?;

        // Get device queue
        let queue = unsafe {
            device.get_device_queue(queue_family_index, 0)
        };

        // Create image
        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::ImageCreateFlags::empty(),
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            extent: vk::Extent3D {
                width: width as u32,
                height: height as u32,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };

        let image = unsafe {
            device.create_image(&image_create_info, None)
        }.map_err(|e| {
            fframes_renderer::FFramesRendererError::Skia(
                format!("Failed to create image: {}", e)
            )
        })?;

        // Create get_proc function
        let get_proc = |name: &std::ffi::CStr| {
            unsafe {
                entry.get_instance_proc_addr(instance.handle(), name.as_ptr())
            }
        };

        // Create Skia Vulkan backend context
        let backend_context = unsafe {
            BackendContext::new( 
                instance.handle().as_raw() as _,
                physical_device.as_raw() as _,
                device.handle().as_raw() as _,
                (queue, queue_family_index as usize),
                &get_proc,
            )
        };

        let mut gpu_context = gpu::direct_contexts::make_vulkan(&backend_context, None)
            .ok_or_else(|| {
                fframes_renderer::FFramesRendererError::Skia(
                    "Failed to create Vulkan GPU context".to_string()
                )
            })?;

        // Create image info for backend render target
        let image_info = skia_safe::gpu::vk::ImageInfo {
            image: image.as_raw() as _,
            format: skia_safe::gpu::vk::Format::R8G8B8A8_UNORM,
            ..Default::default()
        };

        // Create surface
        let surface = {
            let backend_render_target = backend_render_targets::make_vk(
                (width as i32, height as i32),
                &image_info,
            );

            gpu::surfaces::wrap_backend_render_target(
                &mut gpu_context,
                &backend_render_target,
                SurfaceOrigin::TopLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .ok_or_else(|| {
                fframes_renderer::FFramesRendererError::Skia(
                    "Failed to wrap backend render target".to_string(),
                )
            })?
        };

        Ok(Self::new_gpu(surface, Some(gpu_context)))
    }
}
