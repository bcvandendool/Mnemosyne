use crate::emulator::EmulatorState;
use crate::ui::{create_ui, UIContext, UIState};
use egui::ahash::AHashMap;
use egui::epaint::{ImageDelta, Primitive};
use egui::{ClippedPrimitive, Context, Rect, TextureId};
use egui_winit::State;
use std::sync::Arc;
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BufferImageCopy, CommandBufferUsage, CopyBufferToImageInfo,
    PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::format::{Format, NumericFormat};
use vulkano::image::sampler::{
    ComponentMapping, ComponentSwizzle, Filter, Sampler, SamplerAddressMode, SamplerCreateInfo,
    SamplerMipmapMode,
};
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::image::{
    Image, ImageAspects, ImageCreateInfo, ImageLayout, ImageSubresourceLayers, ImageType,
    ImageUsage,
};
use vulkano::memory::allocator::{AllocationCreateInfo, DeviceLayout, MemoryTypeFilter};
use vulkano::memory::DeviceAlignment;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    PipelineShaderStageCreateInfo,
};
use vulkano::sync::GpuFuture;
use vulkano::{DeviceSize, NonZeroDeviceSize};
use vulkano_util::context::VulkanoContext;
use vulkano_util::window::VulkanoWindows;
use winit::event::WindowEvent;

const VERTICES_PER_QUAD: DeviceSize = 4;
const VERTEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * VERTICES_PER_QUAD;
const INDEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * 2;

type VertexBuffer = Subbuffer<[egui::epaint::Vertex]>;
type IndexBuffer = Subbuffer<[u32]>;

#[repr(C)]
#[derive(BufferContents, Vertex)]
pub struct EguiVertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub tex_coords: [f32; 2],
    #[format(R8G8B8A8_UNORM)]
    pub color: [u8; 4],
}

pub(crate) struct EguiRenderer {
    pub(crate) ui_state: UIState,
    ui_context: UIContext,
    vertex_index_buffer_pool: SubbufferAllocator,
    font_sampler: Arc<Sampler>,
    font_format: Format,
    texture_images: AHashMap<egui::TextureId, Arc<ImageView>>,
    texture_desc_sets: AHashMap<egui::TextureId, Arc<DescriptorSet>>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    rcx: Option<RenderContext>,
}

struct RenderContext {
    egui_context: Context,
    egui_winit_state: State,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
}

impl EguiRenderer {
    pub(crate) fn new(
        vulkano_context: &VulkanoContext,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    ) -> Self {
        let vertex_index_buffer_pool = SubbufferAllocator::new(
            vulkano_context.memory_allocator().clone(),
            SubbufferAllocatorCreateInfo {
                arena_size: INDEX_BUFFER_SIZE + VERTEX_BUFFER_SIZE,
                buffer_usage: BufferUsage::INDEX_BUFFER | BufferUsage::VERTEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let font_sampler = Sampler::new(
            vulkano_context.device().clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                mipmap_mode: SamplerMipmapMode::Linear,
                ..Default::default()
            },
        )
        .unwrap();

        let font_format = {
            let supports_swizzle = !vulkano_context
                .device()
                .physical_device()
                .supported_extensions()
                .khr_portability_subset
                || vulkano_context
                    .device()
                    .physical_device()
                    .supported_features()
                    .image_view_format_swizzle;
            let is_supported = |device: &Device, format: Format| {
                device
                    .physical_device()
                    .image_format_properties(vulkano::image::ImageFormatInfo {
                        format,
                        usage: ImageUsage::SAMPLED
                            | ImageUsage::TRANSFER_DST
                            | ImageUsage::TRANSFER_SRC,
                        ..Default::default()
                    })
                    .is_ok_and(|properties| properties.is_some())
            };
            if supports_swizzle && is_supported(vulkano_context.device(), Format::R8G8_UNORM) {
                Format::R8G8_UNORM
            } else {
                Format::R8G8B8A8_SRGB
            }
        };

        EguiRenderer {
            ui_state: UIState::new(),
            ui_context: UIContext::new(),
            vertex_index_buffer_pool,
            font_sampler,
            font_format,
            texture_images: AHashMap::new(),
            texture_desc_sets: AHashMap::new(),
            command_buffer_allocator,
            descriptor_set_allocator,
            rcx: None,
        }
    }

    pub(crate) fn create_render_context(
        &mut self,
        vulkano_context: &VulkanoContext,
        vulkano_windows: &VulkanoWindows,
    ) {
        let egui_context = Context::default();
        let egui_winit_state = State::new(
            egui_context.clone(),
            egui::viewport::ViewportId::ROOT,
            vulkano_windows.get_primary_window().unwrap(),
            Some(vulkano_windows.get_primary_window().unwrap().scale_factor() as f32),
            None,
            Some(2 * 1024), // default dimension is 2048
        );

        let pipeline = self.create_pipeline(vulkano_context, vulkano_windows);

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: vulkano_windows
                .get_primary_window()
                .unwrap()
                .inner_size()
                .into(),
            depth_range: 0.0..=1.0,
        };

        self.rcx = Some(RenderContext {
            egui_context,
            egui_winit_state,
            pipeline,
            viewport,
        });
    }

    fn create_pipeline(
        &mut self,
        vulkano_context: &VulkanoContext,
        vulkano_windows: &VulkanoWindows,
    ) -> Arc<GraphicsPipeline> {
        let vs = vs::load(vulkano_context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();
        let fs = fs::load(vulkano_context.device().clone())
            .unwrap()
            .entry_point("main")
            .unwrap();

        let mut blend = AttachmentBlend::alpha();
        blend.src_color_blend_factor = BlendFactor::One;
        blend.src_alpha_blend_factor = BlendFactor::OneMinusDstAlpha;
        blend.dst_alpha_blend_factor = BlendFactor::One;
        let blend_state = ColorBlendState {
            attachments: vec![ColorBlendAttachmentState {
                blend: Some(blend),
                ..Default::default()
            }],
            ..ColorBlendState::default()
        };

        let vertex_input_state = EguiVertex::per_vertex().definition(&vs).unwrap();

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

        GraphicsPipeline::new(
            vulkano_context.device().clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(blend_state),
                dynamic_state: [DynamicState::Viewport, DynamicState::Scissor]
                    .into_iter()
                    .collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    }

    pub(crate) fn handle_window_event(
        &mut self,
        vulkano_windows: &VulkanoWindows,
        event: &WindowEvent,
    ) {
        let _ = self
            .rcx
            .as_mut()
            .unwrap()
            .egui_winit_state
            .on_window_event(vulkano_windows.get_primary_window().unwrap(), event);
    }

    pub(crate) fn update_extent(&mut self, extent: [f32; 2]) {
        self.rcx.as_mut().unwrap().viewport.extent = extent;
    }

    fn image_size_bytes(delta: &ImageDelta, font_format: Format) -> usize {
        match &delta.image {
            egui::ImageData::Color(c) => {
                // Always four bytes per pixel for sRGBA
                c.width() * c.height() * 4
            }
            egui::ImageData::Font(f) => {
                f.width()
                    * f.height()
                    * match font_format {
                        Format::R8G8_UNORM => 2,
                        Format::R8G8B8A8_SRGB => 4,
                        // Exhaustive list of valid font formats
                        _ => unreachable!(),
                    }
            }
        }
    }

    fn pack_font_data_into(data: &egui::FontImage, font_format: Format, into: &mut [u8]) {
        match font_format {
            Format::R8G8_UNORM => {
                let linear = data
                    .pixels
                    .iter()
                    .map(|f| (f.clamp(0.0, 1.0 - f32::EPSILON) * 256.0) as u8);
                let bytes = linear
                    .zip(data.srgba_pixels(None))
                    .flat_map(|(linear, srgb)| [linear, srgb.a()]);

                into.iter_mut()
                    .zip(bytes)
                    .for_each(|(into, from)| *into = from);
            }
            Format::R8G8B8A8_SRGB => {
                let bytes = data.srgba_pixels(None).flat_map(|color| color.to_array());
                into.iter_mut()
                    .zip(bytes)
                    .for_each(|(into, from)| *into = from);
            }
            _ => unreachable!(),
        }
    }

    fn get_rect_scissor(
        scale_factor: f32,
        framebuffer_dimensions: [u32; 2],
        rect: Rect,
    ) -> Scissor {
        let min = rect.min;
        let min = egui::Pos2 {
            x: min.x * scale_factor,
            y: min.y * scale_factor,
        };
        let min = egui::Pos2 {
            x: min.x.clamp(0.0, framebuffer_dimensions[0] as f32),
            y: min.y.clamp(0.0, framebuffer_dimensions[1] as f32),
        };
        let max = rect.max;
        let max = egui::Pos2 {
            x: max.x * scale_factor,
            y: max.y * scale_factor,
        };
        let max = egui::Pos2 {
            x: max.x.clamp(min.x, framebuffer_dimensions[0] as f32),
            y: max.y.clamp(min.y, framebuffer_dimensions[1] as f32),
        };
        Scissor {
            offset: [min.x.round() as u32, min.y.round() as u32],
            extent: [
                (max.x.round() - min.x) as u32,
                (max.y.round() - min.y) as u32,
            ],
        }
    }

    fn upload_egui_textures(
        &mut self,
        vulkano_context: &VulkanoContext,
        textures_delta: Vec<(TextureId, ImageDelta)>,
    ) {
        let total_size_bytes = textures_delta
            .iter()
            .map(|(_, set)| Self::image_size_bytes(set, self.font_format))
            .sum::<usize>()
            * 4;
        let total_size_bytes = u64::try_from(total_size_bytes).unwrap();

        if let Ok(total_size_bytes) = vulkano::NonZeroDeviceSize::try_from(total_size_bytes) {
            let buffer = Buffer::new(
                vulkano_context.memory_allocator().clone(),
                BufferCreateInfo {
                    usage: BufferUsage::TRANSFER_SRC,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                DeviceLayout::new(total_size_bytes, DeviceAlignment::MIN).unwrap(),
            )
            .unwrap();
            let buffer = Subbuffer::new(buffer);

            let mut builder = AutoCommandBufferBuilder::primary(
                self.command_buffer_allocator.clone(),
                vulkano_context.graphics_queue().queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            {
                let mut writer = buffer.write().unwrap();
                let mut past_buffer_end = 0usize;

                for (id, delta) in textures_delta {
                    let image_size_bytes = Self::image_size_bytes(&delta, self.font_format);
                    let range = past_buffer_end..(image_size_bytes + past_buffer_end);

                    past_buffer_end += image_size_bytes;
                    let stage = buffer.clone().slice(range.start as u64..range.end as u64);
                    let mapped_stage = &mut writer[range];

                    let format = match &delta.image {
                        egui::ImageData::Color(image) => {
                            assert_eq!(
                                image.width() * image.height(),
                                image.pixels.len(),
                                "Mismatch between texture size and texel count"
                            );
                            let bytes = image.pixels.iter().flat_map(|color| color.to_array());
                            mapped_stage
                                .iter_mut()
                                .zip(bytes)
                                .for_each(|(into, from)| *into = from);
                            Format::R8G8B8A8_SRGB
                        }
                        egui::ImageData::Font(image) => {
                            Self::pack_font_data_into(image, self.font_format, mapped_stage);
                            self.font_format
                        }
                    };

                    if let Some(pos) = delta.pos {
                        let Some(existing_image) = self.texture_images.get(&id) else {
                            panic!("attempt to write into non-existing image");
                        };
                        assert_eq!(existing_image.format(), format);

                        builder
                            .copy_buffer_to_image(CopyBufferToImageInfo {
                                regions: [BufferImageCopy {
                                    image_offset: [pos[0] as u32, pos[1] as u32, 0],
                                    image_extent: [
                                        delta.image.width() as u32,
                                        delta.image.height() as u32,
                                        1,
                                    ],
                                    image_subresource: ImageSubresourceLayers {
                                        aspects: ImageAspects::COLOR,
                                        mip_level: 0,
                                        array_layers: 0..1,
                                    },
                                    ..Default::default()
                                }]
                                .into(),
                                ..CopyBufferToImageInfo::buffer_image(
                                    stage,
                                    existing_image.image().clone(),
                                )
                            })
                            .unwrap();
                    } else {
                        let img = {
                            let extent =
                                [delta.image.width() as u32, delta.image.height() as u32, 1];
                            Image::new(
                                vulkano_context.memory_allocator().clone(),
                                ImageCreateInfo {
                                    image_type: ImageType::Dim2d,
                                    format,
                                    extent,
                                    usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                                    initial_layout: ImageLayout::Undefined,
                                    ..Default::default()
                                },
                                AllocationCreateInfo::default(),
                            )
                            .unwrap()
                        };
                        builder
                            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                                stage,
                                img.clone(),
                            ))
                            .unwrap();
                        let component_mapping = match format {
                            Format::R8G8_UNORM => ComponentMapping {
                                r: ComponentSwizzle::Red,
                                g: ComponentSwizzle::Red,
                                b: ComponentSwizzle::Red,
                                a: ComponentSwizzle::Green,
                            },
                            _ => ComponentMapping::identity(),
                        };
                        let view = ImageView::new(
                            img.clone(),
                            ImageViewCreateInfo {
                                component_mapping,
                                ..ImageViewCreateInfo::from_image(&img)
                            },
                        )
                        .unwrap();
                        let layout = self
                            .rcx
                            .as_mut()
                            .unwrap()
                            .pipeline
                            .layout()
                            .set_layouts()
                            .first()
                            .unwrap();
                        let desc_set = DescriptorSet::new(
                            self.descriptor_set_allocator.clone(),
                            layout.clone(),
                            [WriteDescriptorSet::image_view_sampler(
                                0,
                                view.clone(),
                                self.font_sampler.clone(),
                            )],
                            [],
                        )
                        .unwrap();
                        self.texture_desc_sets.insert(id, desc_set);
                        self.texture_images.insert(id, view);
                    };
                }
            }
            let command_buffer = builder.build().unwrap();
            command_buffer
                .execute(vulkano_context.graphics_queue().clone())
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();
        }
    }

    fn upload_meshes(
        clipped_meshes: &[ClippedPrimitive],
        vertex_index_buffer_pool: &SubbufferAllocator,
    ) -> Option<(VertexBuffer, IndexBuffer)> {
        use egui::epaint::Vertex;
        type Index = u32;
        const VERTEX_ALIGN: DeviceAlignment = DeviceAlignment::of::<Vertex>();
        const INDEX_ALIGN: DeviceAlignment = DeviceAlignment::of::<Index>();

        let meshes = clipped_meshes
            .iter()
            .filter_map(|mesh| match &mesh.primitive {
                Primitive::Mesh(m) => Some(m),
                _ => None,
            });

        let (total_vertices, total_size_bytes) = {
            let mut total_vertices = 0;
            let mut total_indices = 0;

            for mesh in meshes.clone() {
                total_vertices += mesh.vertices.len();
                total_indices += mesh.indices.len();
            }
            if total_indices == 0 || total_vertices == 0 {
                return None;
            }

            let total_size_bytes =
                total_vertices * size_of::<Vertex>() + total_indices * size_of::<Index>();
            (
                total_vertices,
                NonZeroDeviceSize::new(u64::try_from(total_size_bytes).unwrap()).unwrap(),
            )
        };

        let layout = DeviceLayout::new(total_size_bytes, VERTEX_ALIGN.max(INDEX_ALIGN)).unwrap();
        let buffer = vertex_index_buffer_pool.allocate(layout).unwrap();

        assert!(VERTEX_ALIGN >= INDEX_ALIGN);
        let (vertices, indices) = {
            let partition_bytes = total_vertices as u64 * size_of::<Vertex>() as u64;
            (
                buffer
                    .clone()
                    .slice(..partition_bytes)
                    .reinterpret::<[Vertex]>(),
                buffer.slice(partition_bytes..).reinterpret::<[Index]>(),
            )
        };

        {
            let mut vertex_write = vertices.write().unwrap();
            vertex_write
                .iter_mut()
                .zip(meshes.clone().flat_map(|m| &m.vertices).copied())
                .for_each(|(into, from)| *into = from);
        }
        {
            let mut index_write = indices.write().unwrap();
            index_write
                .iter_mut()
                .zip(meshes.flat_map(|m| &m.indices).copied())
                .for_each(|(into, from)| *into = from);
        }

        Some((vertices, indices))
    }

    pub(crate) fn render(
        &mut self,
        vulkano_context: &VulkanoContext,
        vulkano_windows: &VulkanoWindows,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        emu_state: EmulatorState,
    ) {
        let rcx = self.rcx.as_mut().unwrap();
        let egui_winit_state = &mut rcx.egui_winit_state;
        let egui_context = &rcx.egui_context;

        let raw_input =
            egui_winit_state.take_egui_input(vulkano_windows.get_primary_window().unwrap());
        egui_winit_state.egui_ctx().begin_pass(raw_input);

        create_ui(
            egui_context,
            vulkano_windows.get_primary_window().unwrap().inner_size(),
            &mut self.ui_state,
            emu_state,
            &mut self.ui_context,
        );

        let full_output = egui_context.end_pass();
        egui_winit_state.handle_platform_output(
            vulkano_windows.get_primary_window().unwrap(),
            full_output.platform_output,
        );

        let textures_delta = full_output.textures_delta;
        let clipped_meshes = egui_context.tessellate(
            full_output.shapes,
            egui_winit::pixels_per_point(
                egui_context,
                vulkano_windows.get_primary_window().unwrap(),
            ),
        );

        self.upload_egui_textures(vulkano_context, textures_delta.set);
        let rcx = self.rcx.as_mut().unwrap();

        let window_size = vulkano_windows.get_primary_window().unwrap().inner_size();
        let push_constants = vs::PushConstants {
            screen_size: [
                window_size.width as f32
                    / egui_winit::pixels_per_point(
                        &rcx.egui_context,
                        &vulkano_windows.get_primary_window().unwrap(),
                    ),
                window_size.height as f32
                    / egui_winit::pixels_per_point(
                        &rcx.egui_context,
                        &vulkano_windows.get_primary_window().unwrap(),
                    ),
            ],
            output_in_linear_colorspace: (vulkano_windows
                .get_primary_renderer()
                .unwrap()
                .swapchain_format()
                .numeric_format_color()
                .unwrap()
                == NumericFormat::SRGB)
                .into(),
        };

        let mesh_buffers = Self::upload_meshes(&clipped_meshes, &self.vertex_index_buffer_pool);

        let mut vertex_cursor = 0;
        let mut index_cursor = 0;
        let mut needs_full_rebind = true;
        let mut current_rect = None;
        let mut current_texture = None;

        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in clipped_meshes
        {
            match primitive {
                Primitive::Mesh(mesh) => {
                    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                        index_cursor += mesh.indices.len() as u32;
                        vertex_cursor += mesh.vertices.len() as u32;
                        continue;
                    }
                    if needs_full_rebind {
                        needs_full_rebind = false;

                        let Some((vertices, indices)) = mesh_buffers.clone() else {
                            unreachable!()
                        };

                        builder
                            .bind_pipeline_graphics(rcx.pipeline.clone())
                            .unwrap()
                            .bind_index_buffer(indices)
                            .unwrap()
                            .bind_vertex_buffers(0, [vertices])
                            .unwrap()
                            .set_viewport(0, [rcx.viewport.clone()].into_iter().collect())
                            .unwrap()
                            .push_constants(rcx.pipeline.layout().clone(), 0, push_constants)
                            .unwrap();
                    }
                    if current_texture != Some(mesh.texture_id) {
                        if self.texture_desc_sets.get(&mesh.texture_id).is_none() {
                            eprintln!("This texture no longer exists {:?}", mesh.texture_id);
                            continue;
                        }
                        current_texture = Some(mesh.texture_id);
                        let desc_set = self.texture_desc_sets.get(&mesh.texture_id).unwrap();

                        builder
                            .bind_descriptor_sets(
                                PipelineBindPoint::Graphics,
                                rcx.pipeline.layout().clone(),
                                0,
                                desc_set.clone(),
                            )
                            .unwrap();
                    };
                    if current_rect != Some(clip_rect) {
                        current_rect = Some(clip_rect);
                        let new_scissor = Self::get_rect_scissor(
                            egui_winit::pixels_per_point(
                                &rcx.egui_context,
                                vulkano_windows.get_primary_window().unwrap(),
                            ),
                            [window_size.width, window_size.height],
                            clip_rect,
                        );

                        builder
                            .set_scissor(0, [new_scissor].into_iter().collect())
                            .unwrap();
                    }

                    unsafe {
                        builder
                            .draw_indexed(
                                mesh.indices.len() as u32,
                                1,
                                index_cursor,
                                vertex_cursor as i32,
                                0,
                            )
                            .unwrap();
                    }

                    index_cursor += mesh.indices.len() as u32;
                    vertex_cursor += mesh.vertices.len() as u32;
                }
                Primitive::Callback(_) => {}
            }
        }
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450
            
            layout(location = 0) in vec2 position;
            layout(location = 1) in vec2 tex_coords;
            layout(location = 2) in vec4 color;
            
            layout(location = 0) out vec4 v_color;
            layout(location = 1) out vec2 v_tex_coords;
            
            layout(push_constant) uniform PushConstants {
                vec2 screen_size;
                int output_in_linear_colorspace;
            } push_constants;
            
            void main() {
                gl_Position = vec4(
                    2.0 * position.x / push_constants.screen_size.x - 1.0,
                    2.0 * position.y / push_constants.screen_size.y - 1.0,
                    0.0, 1.0
                );
                v_color = color;
                v_tex_coords = tex_coords;
            v_color = color;
            v_tex_coords = tex_coords;
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450
            
            layout(location = 0) in vec4 v_color;
            layout(location = 1) in vec2 v_tex_coords;
            
            layout(location = 0) out vec4 f_color;
            
            layout(binding = 0, set = 0) uniform sampler2D font_texture;
            
            layout(push_constant) uniform PushConstants {
                vec2 screen_size;
                int output_in_linear_colorspace;
            } push_constants;
            
            // 0-1 sRGB  from  0-1 linear
            vec3 srgb_from_linear(vec3 linear) {
                bvec3 cutoff = lessThan(linear, vec3(0.0031308));
                vec3 lower = linear * vec3(12.92);
                vec3 higher = vec3(1.055) * pow(linear, vec3(1./2.4)) - vec3(0.055);
                return mix(higher, lower, vec3(cutoff));
            }
            
            // 0-1 sRGBA  from  0-1 linear
            vec4 srgba_from_linear(vec4 linear) {
                return vec4(srgb_from_linear(linear.rgb), linear.a);
            }
            
            // 0-1 linear  from  0-1 sRGB
            vec3 linear_from_srgb(vec3 srgb) {
                bvec3 cutoff = lessThan(srgb, vec3(0.04045));
                vec3 lower = srgb / vec3(12.92);
                vec3 higher = pow((srgb + vec3(0.055) / vec3(1.055)), vec3(2.4));
                return mix(higher, lower, vec3(cutoff));
            }
            
            // 0-1 linear  from  0-1 sRGB
            vec4 linear_from_srgba(vec4 srgb) {
                return vec4(linear_from_srgb(srgb.rgb), srgb.a);
            }
            
            void main() {
                // ALL calculations should be done in gamma space, this includes texture * color and blending
                vec4 texture_color = srgba_from_linear(texture(font_texture, v_tex_coords));
                vec4 color = v_color * texture_color;
            
                // If output_in_linear_colorspace is true, we are rendering into an sRGB image, for which we'll convert to linear color space.
                // **This will break blending** as it will be performed in linear color space instead of sRGB like egui expects.
                if (push_constants.output_in_linear_colorspace == 1) {
                    color = linear_from_srgba(color);
                }
                f_color = color;
            }
        ",
    }
}
