pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

impl Texture {
    pub fn new(device: &wgpu::Device, dimensions: (u32, u32), label: Option<&str>) -> Self {
        let (width, height) = dimensions;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_DST
                | wgpu::TextureUsage::STORAGE,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            format,
        }
    }
}
