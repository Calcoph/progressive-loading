use std::mem::size_of;

use image::{ImageBuffer, Rgb, Rgba};
use imgui::TextureId;
use imgui_wgpu::{Renderer, TextureConfig};
use wgpu::{Device, Queue};

pub struct DataState {
    pub server: ServerData,
    pub client: ClientData
}

impl DataState {
    pub fn new(device: &Device, renderer: &mut Renderer, queue: &Queue) -> DataState {
        let server = ServerData::new(device, renderer, queue);
        let client = ClientData::new(device, renderer, queue, server.image_size[0] as u32, server.image_size[1] as u32);

        DataState {
            server,
            client
        }
    }
}

pub struct ServerData {
    sending_image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub image_size: [f32; 2],
    pub texture_id: TextureId,
    send_stage: SendStage
}

const DEFAULT_IMAGE: &'static str = "flores.jpg";

impl ServerData {
    fn new(device: &Device, renderer: &mut Renderer, queue: &Queue) -> ServerData {
        let sending_image = image::open(DEFAULT_IMAGE).unwrap();
        let sending_image = sending_image.to_rgba8();
        let width = sending_image.width();
        let height = sending_image.height();

        let texture_id = get_texture_id(device, renderer, queue, width, height, &sending_image.as_raw());

        ServerData {
            image_size: [width as f32, height as f32],
            sending_image,
            texture_id,
            send_stage: SendStage::init(),
        }
    }

    pub fn send(&mut self) -> Vec<u8> {
        let step = self.send_stage.step();
        self.send_stage.next().unwrap();

        todo!()
    }

    pub(crate) fn clear(&self) {
        todo!()
    }
}

fn get_texture_id(device: &Device, renderer: &mut Renderer, queue: &Queue, width: u32, height: u32, data: &[u8]) -> TextureId {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = imgui_wgpu::Texture::new(device, renderer, TextureConfig {
        size,
        label: None,
        format: None,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        sampler_desc: wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        },
    });
    texture.write(&queue, data, width, height);

    renderer.textures.insert(texture)
}

pub struct ClientData {
    pub texture_id: TextureId,
    pub size: [f32; 2],
    receiving_image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    send_stage: SendStage
}

impl ClientData {
    fn new(device: &Device, renderer: &mut Renderer, queue: &Queue, width: u32, height: u32) -> ClientData {
        let receiving_image = ImageBuffer::new(width, height);
        let texture_id = get_texture_id(device, renderer, queue, width, height, &receiving_image.as_raw());
        ClientData {
            texture_id,
            size: [width as f32, height as f32],
            receiving_image,
            send_stage: SendStage::init()
        }
    }

    pub fn receive(&mut self, data: &[u8]) {
        todo!()
    }

    pub(crate) fn clear(&mut self, device: &Device, renderer: &mut Renderer, queue: &Queue) {
        let width = self.size[0] as u32;
        let height = self.size[1] as u32;
        let receiving_image = ImageBuffer::new(width, height);
        let texture_id = get_texture_id(device, renderer, queue, width, height, &receiving_image.as_raw());

        self.texture_id = texture_id;
        self.receiving_image = receiving_image;
    }
}

enum SendStage {
    S64x64,
    S32x32,
    S16x16,
    S8x8,
    S4x4,
    S2x2,
    S1x1
}

impl SendStage {
    fn init() -> SendStage {
        SendStage::S64x64
    }

    fn next(&mut self) -> Result<(), ()> {
        match self {
            SendStage::S64x64 => *self = SendStage::S32x32,
            SendStage::S32x32 => *self = SendStage::S16x16,
            SendStage::S16x16 => *self = SendStage::S8x8,
            SendStage::S8x8 => *self = SendStage::S4x4,
            SendStage::S4x4 => *self = SendStage::S2x2,
            SendStage::S2x2 => *self = SendStage::S1x1,
            SendStage::S1x1 => return Err(()),
        }

        Ok(())
    }

    fn step(&self) -> u32 {
        match self {
            SendStage::S64x64 => 64,
            SendStage::S32x32 => 32,
            SendStage::S16x16 => 16,
            SendStage::S8x8 => 8,
            SendStage::S4x4 => 4,
            SendStage::S2x2 => 2,
            SendStage::S1x1 => 1,
        }
    }
}
