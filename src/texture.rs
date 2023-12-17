pub struct Texture {
    descriptor: wgpu::TextureDescriptor<'static>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler_descriptor: wgpu::SamplerDescriptor<'static>,
    sampler: wgpu::Sampler,
}

impl Texture {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        texture_label: Option<&'static str>,
        sampler_label: Option<&'static str>,
        device: &wgpu::Device,
        size: wgpu::Extent3d,
        format: wgpu::TextureFormat,
        address_mode: wgpu::AddressMode,
        min_filter: wgpu::FilterMode,
        mag_filter: wgpu::FilterMode,
        compare: Option<wgpu::CompareFunction>,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let descriptor = wgpu::TextureDescriptor {
            label: texture_label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        };

        let texture = device.create_texture(&descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler_descriptor = wgpu::SamplerDescriptor {
            label: sampler_label,
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter,
            min_filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare,
            ..Default::default()
        };
        let sampler = device.create_sampler(&sampler_descriptor);

        Self {
            descriptor,
            texture,
            view,
            sampler_descriptor,
            sampler,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: wgpu::Extent3d) -> bool {
        if self.texture.size() != size {
            self.descriptor.size = size;
            self.texture = device.create_texture(&self.descriptor);
            self.view = self
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.sampler = device.create_sampler(&self.sampler_descriptor);
            true
        } else {
            false
        }
    }

    pub fn descriptor(&self) -> &wgpu::TextureDescriptor<'static> {
        &self.descriptor
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}
