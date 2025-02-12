use crate::egui_renderer::EguiRenderer;
use std::sync::Arc;
use std::time::Duration;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, RenderingAttachmentInfo,
    RenderingInfo,
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
use vulkano::swapchain::Surface;
use vulkano::sync::GpuFuture;
use vulkano::DeviceSize;
use vulkano_util::context::{VulkanoConfig, VulkanoContext};
use vulkano_util::window::VulkanoWindows;
use winit::event_loop::{ActiveEventLoop, EventLoop};

pub(crate) struct VulkanRenderer {
    pub(crate) context: VulkanoContext,
    pub(crate) windows: VulkanoWindows,
    pub(crate) command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub(crate) descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub(crate) upload_buffer: Subbuffer<[u8]>,
    image: Arc<Image>,
    texture: Arc<ImageView>,
    texture_sampler: Arc<Sampler>,
    rcx: Option<RenderContext>,
}

struct RenderContext {
    viewport: Viewport,
    pipeline: Arc<GraphicsPipeline>,
    descriptor_set: Arc<DescriptorSet>,
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
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            context.device().clone(),
            Default::default(),
        ));

        let texture_sampler =
            Sampler::new(context.device().clone(), SamplerCreateInfo::default()).unwrap();

        let upload_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            context.memory_allocator().clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (160 * 144 * 4) as DeviceSize, // Size of Game Boy screen
        )
        .unwrap();

        let png_bytes = include_bytes!("tetris.png").as_slice();
        let decoder = png::Decoder::new(png_bytes);
        let mut reader = decoder.read_info().unwrap();

        {
            reader
                .next_frame(&mut upload_buffer.write().unwrap())
                .unwrap();
        }

        let image = Image::new(
            context.memory_allocator().clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [160, 144, 1], // Size of Game Boy screen
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        let texture = ImageView::new_default(image.clone()).unwrap();

        VulkanRenderer {
            context,
            windows,
            command_buffer_allocator,
            descriptor_set_allocator,
            upload_buffer,
            image,
            texture,
            texture_sampler,
            rcx: None,
        }
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

    pub(crate) fn redraw(&mut self, renderer_egui: &mut EguiRenderer) {
        let window_size = self
            .windows
            .get_primary_renderer()
            .unwrap()
            .window()
            .inner_size();

        if window_size.width == 0 || window_size.height == 0 {
            return;
        }

        let previous_frame_end = self.windows.get_primary_renderer_mut().unwrap().acquire(
            Some(Duration::from_millis(1000)),
            |_swapchain_images| {
                self.rcx.as_mut().unwrap().viewport.extent = window_size.into();
                renderer_egui.update_extent(window_size.into());
            },
        );

        // Create commandbuffer
        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.context.graphics_queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // upload texture to gpu
        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.upload_buffer.clone(),
                self.image.clone(),
            ))
            .unwrap();

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
            .unwrap()
            .bind_pipeline_graphics(self.rcx.as_mut().unwrap().pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.rcx.as_mut().unwrap().pipeline.layout().clone(),
                0,
                self.rcx.as_mut().unwrap().descriptor_set.clone(),
            )
            .unwrap();

        unsafe { builder.draw(3, 1, 0, 0) }.unwrap();

        // Render egui pipeline
        renderer_egui.render(&self.context, &self.windows, &mut builder);

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

        self.windows
            .create_window(event_loop, &self.context, &Default::default(), |_| {});
    }

    fn create_pipeline(&mut self) -> Arc<GraphicsPipeline> {
        let vs = vs::load(self.context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();
        let fs = fs::load(self.context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            self.context.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(self.context.device().clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = PipelineRenderingCreateInfo {
            color_attachment_formats: vec![Some(
                self.windows
                    .get_primary_renderer()
                    .unwrap()
                    .swapchain_format(),
            )],
            ..Default::default()
        };

        GraphicsPipeline::new(
            self.context.device().clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(VertexInputState::default()),
                input_assembly_state: Some(InputAssemblyState {
                    topology: PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                }),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState {
                    cull_mode: CullMode::Front,
                    front_face: FrontFace::CounterClockwise,
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.color_attachment_formats.len() as u32,
                    ColorBlendAttachmentState {
                        blend: Some(AttachmentBlend::alpha()),
                        ..Default::default()
                    },
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    }

    pub(crate) fn create_render_context(&mut self, event_loop: &ActiveEventLoop) {
        self.create_windows(event_loop);
        let pipeline = self.create_pipeline();

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

        let layout = &pipeline.layout().set_layouts()[0];
        let descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            layout.clone(),
            [
                WriteDescriptorSet::sampler(0, self.texture_sampler.clone()),
                WriteDescriptorSet::image_view(1, self.texture.clone()),
            ],
            [],
        )
        .unwrap();

        self.rcx = Some(RenderContext {
            pipeline,
            viewport,
            descriptor_set,
        });
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) out vec2 tex_coords;

            void main() {
                tex_coords = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
                gl_Position = vec4(tex_coords * 2.0f + -1.0f, 0.0f, 1.0f);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec2 tex_coords;
            layout(location = 0) out vec4 f_color;

            layout(set = 0, binding = 0) uniform sampler s;
            layout(set = 0, binding = 1) uniform texture2D tex;

            void main() {
                f_color = texture(sampler2D(tex, s), tex_coords);
            }
        ",
    }
}
