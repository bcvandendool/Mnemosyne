use crate::egui_renderer::CallbackContext;
use crate::emulator::EmulatorState;
use crate::vulkan_renderer::EmulatorRenderer;
use egui::PaintCallbackInfo;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::format::Format;
use vulkano::image::sampler::{Sampler, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
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
use vulkano::DeviceSize;
use vulkano_util::context::VulkanoContext;
use vulkano_util::window::VulkanoWindows;

pub(crate) struct GameboyRenderer {
    pipeline: Option<Arc<GraphicsPipeline>>,
    descriptor_set: Option<Arc<DescriptorSet>>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    texture: Arc<ImageView>,
    texture_sampler: Arc<Sampler>,
    upload_buffer: Subbuffer<[u8]>,
    image: Arc<Image>,
}

impl GameboyRenderer {
    pub(crate) fn new(vulkano_context: &VulkanoContext, _: &VulkanoWindows) -> Self {
        let image = Image::new(
            vulkano_context.memory_allocator().clone(),
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

        let texture_sampler = Sampler::new(
            vulkano_context.device().clone(),
            SamplerCreateInfo::default(),
        )
        .unwrap();

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            vulkano_context.device().clone(),
            Default::default(),
        ));

        let upload_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vulkano_context.memory_allocator().clone(),
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

        GameboyRenderer {
            pipeline: None,
            descriptor_set: None,
            descriptor_set_allocator,
            texture,
            texture_sampler,
            upload_buffer,
            image,
        }
    }
}

impl EmulatorRenderer for GameboyRenderer {
    fn create_pipeline(
        &mut self,
        vulkano_context: &VulkanoContext,
        vulkano_windows: &VulkanoWindows,
    ) {
        let vs = vs::load(vulkano_context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();
        let fs = fs::load(vulkano_context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            vulkano_context.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(vulkano_context.device().clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = PipelineRenderingCreateInfo {
            color_attachment_formats: vec![Some(
                vulkano_windows
                    .get_primary_renderer()
                    .unwrap()
                    .swapchain_format(),
            )],
            ..Default::default()
        };

        let pipeline = GraphicsPipeline::new(
            vulkano_context.device().clone(),
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
                ..GraphicsPipelineCreateInfo::layout(layout.clone())
            },
        )
        .unwrap();

        let descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            pipeline.layout().set_layouts()[0].clone(),
            [
                WriteDescriptorSet::sampler(0, self.texture_sampler.clone()),
                WriteDescriptorSet::image_view(1, self.texture.clone()),
            ],
            [],
        )
        .unwrap();

        self.descriptor_set = Some(descriptor_set);
        self.pipeline = Some(pipeline);
    }

    fn sync_render_world(&self, emulator_state: &EmulatorState) {
        if let EmulatorState::GameBoy(gameboy_state) = emulator_state {
            let mut writer = self.upload_buffer.write().unwrap();
            for idx in 0..(160 * 144) {
                let color: u8 = match gameboy_state.frame_buffer[idx] {
                    0 => 0xFF,
                    1 => 0xAA,
                    2 => 0x55,
                    3 => 0x00,
                    _ => panic!("Received invalid color code"),
                };
                writer[idx * 4] = color;
                writer[idx * 4 + 1] = color;
                writer[idx * 4 + 2] = color;
                writer[idx * 4 + 3] = 0xFF;
            }
        }
    }

    fn gpu_upload(&self, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.upload_buffer.clone(),
                self.image.clone(),
            ))
            .unwrap();
    }

    fn render(&self, callback_info: PaintCallbackInfo, callback_context: &mut CallbackContext) {
        let pipeline = match &self.pipeline {
            None => {
                panic!()
            }
            Some(pipeline) => pipeline,
        };

        let descriptor_set = match &self.descriptor_set {
            None => {
                panic!()
            }
            Some(descriptor_set) => descriptor_set,
        };

        // Set gb screen ratio
        let mut width = callback_info.viewport.width();
        let mut height = width * (9.0 / 10.0);

        if height > callback_info.viewport.height() {
            height = callback_info.viewport.height();
            width = height * (10.0 / 9.0);
        }

        let offset_x =
            callback_info.viewport.left() + callback_info.viewport.width() / 2.0 - width / 2.0;
        let offset_y = callback_info.viewport.top();

        callback_context
            .builder
            .set_viewport(
                0,
                [Viewport {
                    offset: [offset_x, offset_y],
                    extent: [width, height],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap()
            .bind_pipeline_graphics(pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                descriptor_set.clone(),
            )
            .unwrap();

        unsafe { callback_context.builder.draw(3, 1, 0, 0) }.unwrap();
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
