use std::error::Error;
use std::fmt;
use std::sync::Arc;

use bytemuck::cast_slice;
use openstory_core::viewport::{LogicalSize, PhysicalSize, Viewport};
use wgpu::CurrentSurfaceTexture;
use winit::dpi::PhysicalSize as WindowSize;
use winit::window::Window;

#[derive(Debug)]
pub enum RendererError {
    Surface(String),
    AdapterUnavailable,
    Device(String),
    SurfaceFrame(&'static str),
    InvalidTextureData,
    TextureTooLarge { width: u32, height: u32, limit: u32 },
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Surface(message) => write!(f, "could not create rendering surface: {message}"),
            Self::AdapterUnavailable => write!(f, "no compatible graphics adapter was found"),
            Self::Device(message) => write!(f, "could not create graphics device: {message}"),
            Self::SurfaceFrame(message) => write!(f, "could not acquire a frame: {message}"),
            Self::InvalidTextureData => write!(f, "BGRA texture dimensions do not match its data"),
            Self::TextureTooLarge {
                width,
                height,
                limit,
            } => write!(
                f,
                "texture dimensions {width}x{height} exceed the GPU limit of {limit}"
            ),
        }
    }
}

impl Error for RendererError {}

/// A GPU-resident BGRA image. Upload once and reuse it across frames.
pub struct GpuTexture {
    inner: Arc<GpuTextureInner>,
    /// Dimensions of this image, which can be smaller than its shared atlas page.
    width: u32,
    height: u32,
    /// Normalized bounds of this image within the underlying GPU texture.
    uv: [f32; 4],
}

struct GpuTextureInner {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    page_width: u32,
    page_height: u32,
}

impl GpuTexture {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Decoded image input for a scene atlas upload.
#[derive(Clone, Copy)]
pub struct BgraImage<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
}

struct AtlasPlacement {
    image_index: usize,
    cell_x: u32,
    cell_y: u32,
}

#[derive(Default)]
struct AtlasPage {
    placements: Vec<AtlasPlacement>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    used_width: u32,
    used_height: u32,
}

impl AtlasPage {
    fn try_place(&mut self, image_index: usize, width: u32, height: u32, limit: u32) -> bool {
        let Some(cell_width) = width.checked_add(2) else {
            return false;
        };
        let Some(cell_height) = height.checked_add(2) else {
            return false;
        };
        if cell_width > limit || cell_height > limit {
            return false;
        }

        if self.cursor_x == 0 {
            self.cursor_x = 1;
            self.cursor_y = 1;
        }
        if self.cursor_x + cell_width > limit {
            self.cursor_x = 1;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        if self.cursor_y + cell_height > limit {
            return false;
        }

        let cell_x = self.cursor_x;
        let cell_y = self.cursor_y;
        self.placements.push(AtlasPlacement {
            image_index,
            cell_x,
            cell_y,
        });
        self.cursor_x += cell_width;
        self.row_height = self.row_height.max(cell_height);
        self.used_width = self.used_width.max(self.cursor_x);
        self.used_height = self.used_height.max(self.cursor_y + cell_height);
        true
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Alpha,
    Additive,
}

/// A logical-coordinate sprite rectangle with top-left origin.
pub struct SpriteDraw<'a> {
    pub texture: &'a GpuTexture,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Normalized texture rectangle in `u_min, v_min, u_max, v_max` order.
    pub uv: [f32; 4],
    /// Per-channel RGB multiplier, applied in the sprite shader.
    pub tint: [f32; 3],
    pub blend_mode: BlendMode,
    pub flip_x: bool,
    /// Multiplies the sampled sprite alpha. Values outside 0..=1 are clamped.
    pub opacity: f32,
}

/// Native WGPU surface and reusable textured-quad pipeline.
pub struct GpuRenderer {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window_size: WindowSize<u32>,
    logical_size: LogicalSize,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    sprite_pipeline_alpha: wgpu::RenderPipeline,
    sprite_pipeline_additive: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_capacity: u64,
    vertices: Vec<[f32; 8]>,
    draw_ranges: Vec<(std::ops::Range<u32>, usize)>,
}

impl GpuRenderer {
    pub async fn new(
        window: Arc<Window>,
        logical_size: LogicalSize,
    ) -> Result<Self, RendererError> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| RendererError::Surface(error.to_string()))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| RendererError::AdapterUnavailable)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| RendererError::Device(error.to_string()))?;

        let window_size = window.inner_size();
        let initial = safe_surface_size(window_size);
        let config = surface
            .get_default_config(&adapter, initial.width, initial.height)
            .ok_or(RendererError::AdapterUnavailable)?;
        surface.configure(&device, &config);

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("openstory-sprite-texture-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("openstory-sprite-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("openstory-sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });
        let bind_group_layouts = [Some(&texture_layout)];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("openstory-sprite-pipeline-layout"),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        let sprite_pipeline_alpha = create_sprite_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
            "openstory-sprite-alpha-pipeline",
        );
        let sprite_pipeline_additive = create_sprite_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            config.format,
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            "openstory-sprite-additive-pipeline",
        );
        let vertex_buffer_capacity = 16;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("openstory-sprite-vertices"),
            size: vertex_buffer_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            window_size,
            logical_size,
            texture_layout,
            sampler,
            sprite_pipeline_alpha,
            sprite_pipeline_additive,
            vertex_buffer,
            vertex_buffer_capacity,
            vertices: Vec::with_capacity(256 * 6),
            draw_ranges: Vec::with_capacity(256),
        })
    }

    /// Uploads one decoded NX BGRA bitmap. Pixel memory can be released afterward.
    pub fn upload_bgra(
        &self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<GpuTexture, RendererError> {
        let expected_length = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|length| usize::try_from(length).ok())
            .ok_or(RendererError::InvalidTextureData)?;
        if width == 0 || height == 0 || expected_length != pixels.len() {
            return Err(RendererError::InvalidTextureData);
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(RendererError::TextureTooLarge {
                width,
                height,
                limit,
            });
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("openstory-nx-bitmap"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("openstory-nx-bitmap-bind-group"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        Ok(GpuTexture {
            inner: Arc::new(GpuTextureInner {
                _texture: texture,
                bind_group,
                page_width: width,
                page_height: height,
            }),
            width,
            height,
            uv: [0.0, 0.0, 1.0, 1.0],
        })
    }

    /// Packs decoded scene images into shared GPU texture pages. One-pixel duplicated
    /// gutters protect linear sampling at image edges; images too large for the bounded
    /// atlas are uploaded as standalone textures.
    pub fn upload_bgra_atlas(
        &self,
        images: &[BgraImage<'_>],
    ) -> Result<Vec<GpuTexture>, RendererError> {
        if images.is_empty() {
            return Ok(Vec::new());
        }
        let atlas_limit = 2048_u32.min(self.device.limits().max_texture_dimension_2d);
        let mut order = Vec::new();
        let mut standalone = Vec::new();
        for (index, image) in images.iter().enumerate() {
            let expected_length = image
                .width
                .checked_mul(image.height)
                .and_then(|pixels| pixels.checked_mul(4))
                .and_then(|length| usize::try_from(length).ok())
                .ok_or(RendererError::InvalidTextureData)?;
            if image.width == 0 || image.height == 0 || expected_length != image.pixels.len() {
                return Err(RendererError::InvalidTextureData);
            }
            if image.width.saturating_add(2) <= atlas_limit
                && image.height.saturating_add(2) <= atlas_limit
            {
                order.push(index);
            } else {
                standalone.push(index);
            }
        }
        let mut pages = Vec::<AtlasPage>::new();
        for image_index in order {
            let image = images[image_index];
            if !pages.last_mut().is_some_and(|page| {
                page.try_place(image_index, image.width, image.height, atlas_limit)
            }) {
                let mut page = AtlasPage::default();
                if !page.try_place(image_index, image.width, image.height, atlas_limit) {
                    standalone.push(image_index);
                } else {
                    pages.push(page);
                }
            }
        }

        let mut uploaded = std::iter::repeat_with(|| None)
            .take(images.len())
            .collect::<Vec<Option<GpuTexture>>>();
        for page in pages {
            let page_width = page.used_width.next_power_of_two().max(4);
            let page_height = page.used_height.next_power_of_two().max(4);
            let page_length = page_width
                .checked_mul(page_height)
                .and_then(|pixels| pixels.checked_mul(4))
                .and_then(|length| usize::try_from(length).ok())
                .ok_or(RendererError::InvalidTextureData)?;
            let mut pixels = vec![0; page_length];
            let page_stride = (page_width * 4) as usize;
            for placement in &page.placements {
                let image = images[placement.image_index];
                let source_stride = (image.width * 4) as usize;
                let cell_x = placement.cell_x as usize;
                let cell_y = placement.cell_y as usize;
                let image_width = image.width as usize;
                let image_height = image.height as usize;
                for row in 0..image_height {
                    let source_start = row * source_stride;
                    let source_end = source_start + source_stride;
                    let destination_start = (cell_y + 1 + row) * page_stride + (cell_x + 1) * 4;
                    pixels[destination_start..destination_start + source_stride]
                        .copy_from_slice(&image.pixels[source_start..source_end]);
                    pixels.copy_within(
                        destination_start..destination_start + 4,
                        (cell_y + 1 + row) * page_stride + cell_x * 4,
                    );
                    pixels.copy_within(
                        destination_start + (image_width - 1) * 4
                            ..destination_start + source_stride,
                        (cell_y + 1 + row) * page_stride + (cell_x + image_width + 1) * 4,
                    );
                }
                let cell_row_bytes = (image_width + 2) * 4;
                let first_row = (cell_y + 1) * page_stride + cell_x * 4;
                let top_row = cell_y * page_stride + cell_x * 4;
                pixels.copy_within(first_row..first_row + cell_row_bytes, top_row);
                let last_row = (cell_y + image_height) * page_stride + cell_x * 4;
                let bottom_row = (cell_y + image_height + 1) * page_stride + cell_x * 4;
                pixels.copy_within(last_row..last_row + cell_row_bytes, bottom_row);
            }

            let page_texture = self.upload_bgra(page_width, page_height, &pixels)?;
            for placement in page.placements {
                let image = images[placement.image_index];
                let left = (placement.cell_x + 1) as f32 / page_width as f32;
                let top = (placement.cell_y + 1) as f32 / page_height as f32;
                let right = (placement.cell_x + 1 + image.width) as f32 / page_width as f32;
                let bottom = (placement.cell_y + 1 + image.height) as f32 / page_height as f32;
                uploaded[placement.image_index] = Some(GpuTexture {
                    inner: page_texture.inner.clone(),
                    width: image.width,
                    height: image.height,
                    uv: [left, top, right, bottom],
                });
            }
        }
        for image_index in standalone {
            let image = images[image_index];
            uploaded[image_index] =
                Some(self.upload_bgra(image.width, image.height, image.pixels)?);
        }
        Ok(uploaded
            .into_iter()
            .map(|texture| texture.expect("every scene image was uploaded"))
            .collect())
    }

    /// Replaces pixels in a reusable texture without reallocating GPU resources.
    pub fn update_bgra(&self, texture: &GpuTexture, pixels: &[u8]) -> Result<(), RendererError> {
        let expected_length = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|length| usize::try_from(length).ok())
            .ok_or(RendererError::InvalidTextureData)?;
        if expected_length != pixels.len() {
            return Err(RendererError::InvalidTextureData);
        }
        if texture.uv != [0.0, 0.0, 1.0, 1.0]
            || texture.inner.page_width != texture.width
            || texture.inner.page_height != texture.height
        {
            return Err(RendererError::InvalidTextureData);
        }
        self.queue.write_texture(
            texture.inner._texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width * 4),
                rows_per_image: Some(texture.height),
            },
            wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    /// Updates the swap-chain dimensions. Zero-sized windows remain suspended until
    /// the next nonzero resize event.
    pub fn resize(&mut self, size: WindowSize<u32>) {
        self.window_size = size;
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Clears the frame and renders a list of logical-coordinate sprites.
    pub fn render_sprites(&mut self, sprites: &[SpriteDraw<'_>]) -> Result<(), RendererError> {
        if self.window_size.width == 0 || self.window_size.height == 0 {
            return Ok(());
        }

        let viewport = Viewport::fit(
            self.logical_size,
            PhysicalSize {
                width: self.window_size.width,
                height: self.window_size.height,
            },
        );
        let Some(viewport) = viewport else {
            return Ok(());
        };

        self.vertices.clear();
        self.draw_ranges.clear();
        for (sprite_index, sprite) in sprites.iter().enumerate() {
            if !sprite.x.is_finite()
                || !sprite.y.is_finite()
                || !sprite.width.is_finite()
                || !sprite.height.is_finite()
                || !sprite.opacity.is_finite()
                || sprite.uv.iter().any(|coordinate| !coordinate.is_finite())
                || sprite.tint.iter().any(|channel| !channel.is_finite())
                || sprite.width <= 0.0
                || sprite.height <= 0.0
            {
                continue;
            }
            let opacity = sprite.opacity.clamp(0.0, 1.0);
            if opacity == 0.0 {
                continue;
            }
            let texture_width = sprite.texture.uv[2] - sprite.texture.uv[0];
            let texture_height = sprite.texture.uv[3] - sprite.texture.uv[1];
            let u_min = (sprite.texture.uv[0] + sprite.uv[0] * texture_width).clamp(0.0, 1.0);
            let v_min = (sprite.texture.uv[1] + sprite.uv[1] * texture_height).clamp(0.0, 1.0);
            let u_max = (sprite.texture.uv[0] + sprite.uv[2] * texture_width).clamp(0.0, 1.0);
            let v_max = (sprite.texture.uv[1] + sprite.uv[3] * texture_height).clamp(0.0, 1.0);
            let tint = sprite.tint.map(|channel| channel.clamp(0.0, 1.0));
            if u_max <= u_min || v_max <= v_min {
                continue;
            }
            if sprite.x + sprite.width <= 0.0
                || sprite.y + sprite.height <= 0.0
                || sprite.x >= self.logical_size.width
                || sprite.y >= self.logical_size.height
            {
                continue;
            }

            let left = sprite.x / self.logical_size.width * 2.0 - 1.0;
            let right = (sprite.x + sprite.width) / self.logical_size.width * 2.0 - 1.0;
            let top = 1.0 - sprite.y / self.logical_size.height * 2.0;
            let bottom = 1.0 - (sprite.y + sprite.height) / self.logical_size.height * 2.0;
            let (left_u, right_u) = if sprite.flip_x {
                (u_max, u_min)
            } else {
                (u_min, u_max)
            };
            let start = self.vertices.len() as u32;
            self.vertices.extend_from_slice(&[
                [left, top, left_u, v_min, opacity, tint[0], tint[1], tint[2]],
                [
                    left, bottom, left_u, v_max, opacity, tint[0], tint[1], tint[2],
                ],
                [
                    right, bottom, right_u, v_max, opacity, tint[0], tint[1], tint[2],
                ],
                [left, top, left_u, v_min, opacity, tint[0], tint[1], tint[2]],
                [
                    right, bottom, right_u, v_max, opacity, tint[0], tint[1], tint[2],
                ],
                [
                    right, top, right_u, v_min, opacity, tint[0], tint[1], tint[2],
                ],
            ]);
            let shares_batch_with_previous =
                self.draw_ranges.last().is_some_and(|(_, previous_sprite)| {
                    let previous = &sprites[*previous_sprite];
                    Arc::ptr_eq(&previous.texture.inner, &sprite.texture.inner)
                        && previous.blend_mode == sprite.blend_mode
                });
            if shares_batch_with_previous {
                self.draw_ranges
                    .last_mut()
                    .expect("last draw range was checked")
                    .0
                    .end = start + 6;
            } else {
                self.draw_ranges.push((start..start + 6, sprite_index));
            }
        }

        let vertex_bytes = cast_slice(&self.vertices);
        let needed = vertex_bytes.len().max(16) as u64;
        if needed > self.vertex_buffer_capacity {
            self.vertex_buffer_capacity = needed.next_power_of_two();
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("openstory-sprite-vertices"),
                size: self.vertex_buffer_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !vertex_bytes.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, vertex_bytes);
        }

        let acquired = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => (frame, false),
            CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return Ok(()),
            CurrentSurfaceTexture::Validation => {
                return Err(RendererError::SurfaceFrame(
                    "surface validation failed while acquiring a frame",
                ));
            }
        };
        let (frame, suboptimal) = acquired;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("openstory-frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("openstory-sprites"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_viewport(
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                0.0,
                1.0,
            );
            if !self.draw_ranges.is_empty() {
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                for (range, sprite_index) in &self.draw_ranges {
                    let pipeline = match sprites[*sprite_index].blend_mode {
                        BlendMode::Alpha => &self.sprite_pipeline_alpha,
                        BlendMode::Additive => &self.sprite_pipeline_additive,
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &sprites[*sprite_index].texture.inner.bind_group, &[]);
                    pass.draw(range.clone(), 0..1);
                }
            }
        }

        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }

    /// Clears a frame without drawing sprites.
    pub fn render(&mut self) -> Result<(), RendererError> {
        self.render_sprites(&[])
    }
}

fn create_sprite_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
    label: &'static str,
) -> wgpu::RenderPipeline {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32x3];
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    };
    let color_targets = [Some(wgpu::ColorTargetState {
        format,
        blend: Some(blend),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &[Some(vertex_layout)],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &color_targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn safe_surface_size(size: WindowSize<u32>) -> WindowSize<u32> {
    WindowSize::new(size.width.max(1), size.height.max(1))
}
