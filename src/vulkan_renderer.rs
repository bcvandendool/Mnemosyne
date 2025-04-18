use crate::egui_renderer::{CallbackContext, EguiRenderer};
use crate::emulator::EmulatorState;
use egui::PaintCallbackInfo;
use std::sync::{Arc, Mutex};
use std::vec;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
    RenderingAttachmentInfo, RenderingInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{DeviceExtensions, DeviceFeatures};
use vulkano::format::Format;
use vulkano::image::sampler::{Sampler, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::InstanceCreateInfo;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{CullMode, FrontFace, RasterizationState};
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::VertexInputState;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use vulkano::swapchain::{PresentMode, Surface};
use vulkano::sync::GpuFuture;
use vulkano::DeviceSize;
use vulkano_util::context::{VulkanoConfig, VulkanoContext};
use vulkano_util::window::{VulkanoWindows, WindowDescriptor, WindowMode};
use winit::event_loop::{ActiveEventLoop, EventLoop};

pub(crate) trait EmulatorRenderer: Send {
    fn sync_render_world(&mut self, emulator_state: &EmulatorState);
    fn gpu_upload(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>);
    fn render(
        &self,
        paint_callback_info: PaintCallbackInfo,
        callback_context: &mut CallbackContext,
    );
    fn create_pipeline(
        &mut self,
        vulkano_context: &VulkanoContext,
        vulkano_windows: &VulkanoWindows,
    );
}

pub(crate) struct VulkanRenderer {
    pub(crate) context: VulkanoContext,
    pub(crate) windows: VulkanoWindows,
    pub(crate) command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    // pub(crate) descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    // pub(crate) upload_buffer: Subbuffer<[u8]>,
    // image: Arc<Image>,
    // texture: Arc<ImageView>,
    // texture_sampler: Arc<Sampler>,
    emulator_renderer: Option<Arc<Mutex<dyn EmulatorRenderer>>>,
    rcx: Option<RenderContext>,
}

struct RenderContext {
    viewport: Viewport,
}

impl VulkanRenderer {
    pub(crate) fn new(event_loop: &EventLoop<()>) -> Self {
        let context = VulkanoContext::new(VulkanoConfig {
            device_extensions: DeviceExtensions {
                khr_swapchain: true,
                khr_dynamic_rendering: true,
                ..Default::default()
            },
            device_features: DeviceFeatures {
                dynamic_rendering: true,
                ..Default::default()
            },
            instance_create_info: InstanceCreateInfo {
                enabled_extensions: Surface::required_extensions(event_loop).unwrap(),
                application_name: Some("Mnemosyne".parse().unwrap()),
                ..Default::default()
            },
            ..Default::default()
        });
        let windows = VulkanoWindows::default();

        println!(
            "Using device: {} (type: {:?})",
            context.device().physical_device().properties().device_name,
            context.device().physical_device().properties().device_type
        );

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            context.device().clone(),
            Default::default(),
        ));

        // let upload_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        //     context.memory_allocator().clone(),
        //     BufferCreateInfo {
        //         usage: BufferUsage::TRANSFER_SRC,
        //         ..Default::default()
        //     },
        //     AllocationCreateInfo {
        //         memory_type_filter: MemoryTypeFilter::PREFER_HOST
        //             | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
        //         ..Default::default()
        //     },
        //     (160 * 144 * 4) as DeviceSize, // Size of Game Boy screen
        // )
        // .unwrap();
        //
        // let png_bytes = include_bytes!("tetris.png").as_slice();
        // let decoder = png::Decoder::new(png_bytes);
        // let mut reader = decoder.read_info().unwrap();
        //
        // {
        //     reader
        //         .next_frame(&mut upload_buffer.write().unwrap())
        //         .unwrap();
        // }

        VulkanRenderer {
            context,
            windows,
            command_buffer_allocator,
            // descriptor_set_allocator,
            // upload_buffer,
            // image,
            // texture,
            // texture_sampler,
            emulator_renderer: None,
            rcx: None,
        }
    }

    pub(crate) fn set_emulator_renderer(
        &mut self,
        emulator_renderer: Arc<Mutex<dyn EmulatorRenderer>>,
    ) {
        self.emulator_renderer = Some(emulator_renderer);
    }

    pub(crate) fn resize(&mut self) {
        self.windows.get_primary_renderer_mut().unwrap().resize();
    }

    pub(crate) fn request_redraw(&mut self) {
        self.windows
            .get_primary_renderer_mut()
            .unwrap()
            .window()
            .request_redraw();
    }

    pub(crate) fn redraw(&mut self, renderer_egui: &mut EguiRenderer, emu_state: EmulatorState) {
        puffin::profile_scope!("vulkan renderer");
        let window_size = self
            .windows
            .get_primary_renderer()
            .unwrap()
            .window()
            .inner_size();

        if window_size.width == 0 || window_size.height == 0 {
            return;
        }

        let previous_frame_end;
        {
            puffin::profile_scope!("acquire swapchain");
            previous_frame_end = self.windows.get_primary_renderer_mut().unwrap().acquire(
                None,
                |_swapchain_images| {
                    // Update viewport, either 2/3 of width or 2/3 of height max
                    let mut width = (window_size.width as f32 / 6.0) * 4.0;
                    let mut height = (width / 10.0) * 9.0;

                    if height > (window_size.height as f32 / 4.0) * 3.0 {
                        height = (window_size.height as f32 / 4.0) * 3.0;
                        width = (height / 9.0) * 10.0;
                    }

                    let offset_x = (window_size.width as f32 / 2.0 - width / 2.0) * 1.20;
                    self.rcx.as_mut().unwrap().viewport.offset = [offset_x.round(), 0.0];
                    self.rcx.as_mut().unwrap().viewport.extent = [width.round(), height.round()];
                    renderer_egui.update_extent(window_size.into());
                },
            );
        }

        // Create commandbuffer
        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.context.graphics_queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        match &self.emulator_renderer {
            None => {}
            Some(emulator_renderer) => {
                let a = emulator_renderer.lock().expect("Bla");
                a.gpu_upload(&mut builder);
            }
        };

        // Render commands
        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    store_op: AttachmentStoreOp::Store,
                    clear_value: Some([1.0, 1.0, 1.0, 1.0].into()),
                    ..RenderingAttachmentInfo::image_view(
                        self.windows
                            .get_primary_renderer()
                            .unwrap()
                            .swapchain_image_view()
                            .clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap();

        // Render image pipeline
        builder
            .set_viewport(
                0,
                [self.rcx.as_mut().unwrap().viewport.clone()]
                    .into_iter()
                    .collect(),
            )
            .unwrap();
        // .bind_pipeline_graphics(self.rcx.as_mut().unwrap().pipeline.clone())
        // .unwrap()
        // .bind_descriptor_sets(
        //     PipelineBindPoint::Graphics,
        //     self.rcx.as_mut().unwrap().pipeline.layout().clone(),
        //     0,
        //     self.rcx.as_mut().unwrap().descriptor_set.clone(),
        // )
        // .unwrap();

        // unsafe { builder.draw(3, 1, 0, 0) }.unwrap();

        let bla = match &self.emulator_renderer {
            None => {
                panic!()
            }
            Some(emulator_renderer) => emulator_renderer,
        };

        // Render egui pipeline
        renderer_egui.render(
            &self.context,
            &self.windows,
            &mut builder,
            emu_state,
            bla.clone(),
        );

        // Finish rendering state
        builder.end_rendering().unwrap();

        // Build and execute command buffer
        let command_buffer = builder.build().unwrap();

        let future = previous_frame_end
            .unwrap()
            .then_execute(self.context.graphics_queue().clone(), command_buffer)
            .unwrap()
            .boxed();

        self.windows
            .get_primary_renderer_mut()
            .unwrap()
            .present(future, true);
    }

    fn create_windows(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(primary_window_id) = self.windows.primary_window_id() {
            self.windows.remove_renderer(primary_window_id);
        }

        let window_descriptor = WindowDescriptor {
            width: 1920.0,
            height: 1080.0,
            present_mode: PresentMode::Mailbox,
            title: "Mnemosyne".parse().unwrap(),
            ..Default::default()
        };
        self.windows
            .create_window(event_loop, &self.context, &window_descriptor, |_| {});
    }

    pub(crate) fn create_render_context(&mut self, event_loop: &ActiveEventLoop) {
        self.create_windows(event_loop);
        match &self.emulator_renderer {
            None => {}
            Some(test) => {
                let mut bla = test.lock().unwrap();
                bla.create_pipeline(&self.context, &self.windows);
            }
        }

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: self
                .windows
                .get_primary_window()
                .unwrap()
                .inner_size()
                .into(),
            depth_range: 0.0..=1.0,
        };

        self.rcx = Some(RenderContext { viewport });
    }
}
