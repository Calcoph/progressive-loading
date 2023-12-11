use std::borrow::Cow;
use rayon::prelude::*;

use image::{ImageBuffer, Rgba};
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
    pub send_stage: SendStage,
}

//const DEFAULT_IMAGE: &'static str = "flores.jpg";
//const DEFAULT_IMAGE: &'static str = "depositphotos_70604961-stock-photo-loberia-argentina.webp";
//const DEFAULT_IMAGE: &'static str = "IMG-20231119-WA0014.jpg";
const DEFAULT_IMAGE: &'static str = "IMG-20231119-WA0014_4.jpg";

impl ServerData {
    fn new(device: &Device, renderer: &mut Renderer, queue: &Queue) -> ServerData {
        let sending_image = image::open(DEFAULT_IMAGE).unwrap();
        let mut sending_image = sending_image.to_rgba8();
        for pixel in sending_image.pixels_mut() {
            unsafe {
                let r: *mut u8 = &mut pixel.0[0];
                let b: *mut u8 = &mut pixel.0[2];
                std::ptr::swap(r, b)
            }
        }
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
        let y_step = self.send_stage.y_step();

        let v_size = if self.send_stage == SendStage::init() {
            (self.image_size[0]/y_step as f32).ceil() as usize
            * (self.image_size[1]/y_step as f32).ceil() as usize
            * 4
        } else {
            (self.image_size[0]/y_step as f32).ceil() as usize
            * (self.image_size[1]/y_step as f32).ceil() as usize
            * 3
            // /4 * 4 == noop
        };
        let mut v = Vec::with_capacity(v_size);
        let start_x = if self.send_stage == SendStage::init() {
            0
        } else {
            y_step
        };
        let mut x = start_x;
        let mut y = 0;
        let height = self.sending_image.height();
        let width = self.sending_image.width();
        while y < height {
            let x_step = self.send_stage.x_step(y/y_step);
            while x < width {
                let pixel = self.sending_image.get_pixel(x, y);
                v.extend(pixel.0.as_slice());
                x += x_step;
            }
            if x_step != y_step {
                x = 0;
            } else {
                x = start_x;
            }
            y += y_step;
        }

        self.send_stage.next().unwrap();
        v
    }

    pub(crate) fn clear(&mut self) {
        self.send_stage = SendStage::init()
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
    send_stage: SendStage,
    pub blur: bool,
}

impl ClientData {
    fn new(device: &Device, renderer: &mut Renderer, queue: &Queue, width: u32, height: u32) -> ClientData {
        let receiving_image = ImageBuffer::new(width, height);
        let texture_id = get_texture_id(device, renderer, queue, width, height, &receiving_image.as_raw());
        ClientData {
            texture_id,
            size: [width as f32, height as f32],
            receiving_image,
            send_stage: SendStage::init(),
            blur: false
        }
    }

    pub fn receive(&mut self, device: &Device, renderer: &mut Renderer, queue: &Queue, data: &[u8]) {
        let y_step = self.send_stage.y_step();

        let mut pixels = Vec::with_capacity(data.len()/4);
        let mut i = 0;
        while i+3 < data.len() {
            let pixel: Rgba<u8> = [data[i], data[i+1], data[i+2], data[i+3]].into();
            pixels.push(pixel);
            i += 4;
        };

        let start_x = if self.send_stage == SendStage::init() {
            0
        } else {
            y_step
        };
        let mut x = start_x;
        let mut y = 0;
        let width = self.size[0] as u32;
        let height = self.size[1] as u32;
        for pixel in pixels.into_iter() {
            if y < height && x < width {
            }
            for _ in 0..y_step {
                if y < height {
                    for _ in 0..y_step {
                        if x < width {
                            self.receiving_image.put_pixel(x, y, pixel);
                        }
                        x += 1
                    }
                    x -= y_step;
                }
                y += 1
            }
            y -= y_step;
            let pixel_y = y / y_step;
            let x_step = self.send_stage.x_step(pixel_y);
            x += x_step;
            if x+1 > width {
                x = 0;
                y += y_step;
                let pixel_y = y / y_step;
                let x_step = self.send_stage.x_step(pixel_y);
                if y_step != x_step {
                    x += start_x;
                }
            }
        }

        self.send_stage.next().unwrap();
        self.update_texture(device, renderer, queue);
    }

    pub(crate) fn clear(&mut self, device: &Device, renderer: &mut Renderer, queue: &Queue) {
        let width = self.size[0] as u32;
        let height = self.size[1] as u32;
        let receiving_image = ImageBuffer::new(width, height);
        let texture_id = get_texture_id(device, renderer, queue, width, height, &receiving_image.as_raw());

        self.texture_id = texture_id;
        self.receiving_image = receiving_image;
        self.send_stage = SendStage::init();
    }

    pub(crate) fn update_texture(&mut self, device: &Device, renderer: &mut Renderer, queue: &Queue) {
        let width = self.size[0] as u32;
        let height = self.size[1] as u32;

        let data = if self.blur && self.send_stage != SendStage::End {
            let mut copy = self.receiving_image.clone();
            blur(&mut copy);
            Cow::Owned(copy)
        } else {
            Cow::Borrowed(&self.receiving_image)
        };

        self.texture_id = get_texture_id(device, renderer, queue, width, height, &data);
    }
}

fn blur(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let height = img.height() as i32;
    let width = img.width() as i32;
    let xcl = |x: i32| x.clamp(0, width-1) as u32;
    let ycl = |y: i32| y.clamp(0, height-1) as u32;
    let copy = img.clone();

    img.par_chunks_exact_mut(4).enumerate().for_each(|(i, pixel)| {
        let y = i as i32 / width;
        let x = i as i32 - y * width;

        let outer_corners: Vec<_> = vec![-2,2].into_iter()
            .flat_map(|i| vec![-2, 2].into_iter().map(move |j| (i, j)))
            .map(|(i, j)| copy.get_pixel(xcl(x+i), ycl(y+j)))
            .collect();

        let outer_edges: Vec<_> = vec![-2, -1, 1, 2].into_iter()
            .flat_map(|i| vec![-2, -1, 1, 2].into_iter().map(move |j| (i, j)))
            .filter(|(i, j)| i != j && *i != -j)
            .map(|(i, j)| copy.get_pixel(xcl(x+i), ycl(y+j)))
            .collect();

        let outer_mid_edges: Vec<_> = vec![-2, 0, 2].into_iter()
            .flat_map(|i| vec![-2, 0, 2].into_iter().map(move |j| (i, j)))
            .filter(|(i, j)| i != j && *i != -j)
            .map(|(i, j)| copy.get_pixel(xcl(x+i), ycl(y+j)))
            .collect();

        let inner_corners: Vec<_> = vec![-1,1].into_iter()
            .flat_map(|i| vec![-1, 1].into_iter().map(move |j| (i, j)))
            .map(|(i, j)| copy.get_pixel(xcl(x+i), ycl(y+j)))
            .collect();

        let inner_edges: Vec<_> = vec![-1, 0, 1].into_iter()
            .flat_map(|i| vec![-1, 0, 1].into_iter().map(move |j| (i, j)))
            .filter(|(i, j)| i != j && *i != -j)
            .map(|(i, j)| copy.get_pixel(xcl(x+i), ycl(y+j)))
            .collect();

        let r: f32 = gauss(&outer_corners, &outer_edges, &outer_mid_edges, &inner_corners, &inner_edges, pixel, 0);
        let g: f32 = gauss(&outer_corners, &outer_edges, &outer_mid_edges, &inner_corners, &inner_edges, pixel, 1);
        let b: f32 = gauss(&outer_corners, &outer_edges, &outer_mid_edges, &inner_corners, &inner_edges, pixel, 2);
        pixel[0] = r as u8;
        pixel[1] = g as u8;
        pixel[2] = b as u8;
    });
}

const OUTER_CORNER_WEIGHT: f32 =   0.0396455;
const OUTER_EDGE_WEIGHT: f32 =     0.0399107;
const OUTER_MID_EDGE_WEIGHT: f32 = 0.0399994;
const INNER_CORNER_WEIGHT: f32 =   0.0401776;
const INNER_EDGE_WEIGHT: f32 =     0.0402670;
const SELF_WEIGHT: f32 =           0.0403566;
fn gauss(
    outer_corners: &[&Rgba<u8>], outer_edges: &[&Rgba<u8>],
    outer_mid_edges: &[&Rgba<u8>], inner_corners: &[&Rgba<u8>],
    inner_edges: &[&Rgba<u8>], s: &[u8], i: usize
) -> f32 {
    outer_corners.into_iter().map(|p| p.0[i] as f32 * OUTER_CORNER_WEIGHT).sum::<f32>()
    + outer_edges.into_iter().map(|p| p.0[i] as f32 * OUTER_EDGE_WEIGHT).sum::<f32>()
    + outer_mid_edges.into_iter().map(|p| p.0[i] as f32 * OUTER_MID_EDGE_WEIGHT).sum::<f32>()
    + inner_corners.into_iter().map(|p| p.0[i] as f32 * INNER_CORNER_WEIGHT).sum::<f32>()
    + inner_edges.into_iter().map(|p| p.0[i] as f32 * INNER_EDGE_WEIGHT).sum::<f32>()
    + s[i] as f32 * SELF_WEIGHT
}

#[derive(Debug, PartialEq, Eq)]
pub enum SendStage {
    S64x64,
    S32x32,
    S16x16,
    S8x8,
    S4x4,
    S2x2,
    S1x1,
    End
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
            SendStage::S1x1 => *self = SendStage::End,
            SendStage::End => return Err(()),
        }

        Ok(())
    }

    fn y_step(&self) -> u32 {
        match self {
            SendStage::S64x64 => 64,
            SendStage::S32x32 => 32,
            SendStage::S16x16 => 16,
            SendStage::S8x8 => 8,
            SendStage::S4x4 => 4,
            SendStage::S2x2 => 2,
            SendStage::S1x1 => 1,
            SendStage::End => panic!(),
        }
    }

    fn x_step(&self, y: u32) -> u32 {
        match self {
            SendStage::S64x64 => 64,
            SendStage::S32x32 => if y % 2 == 0 {
                64
            } else {
                32
            },
            SendStage::S16x16 => if y % 2 == 0 {
                32
            } else {
                16
            },
            SendStage::S8x8 => if y % 2 == 0 {
                16
            } else {
                8
            },
            SendStage::S4x4 => if y % 2 == 0 {
                8
            } else {
                4
            },
            SendStage::S2x2 => if y % 2 == 0 {
                4
            } else {
                2
            },
            SendStage::S1x1 => if y % 2 == 0 {
                2
            } else {
                1
            },
            SendStage::End => panic!(),
        }
    }
}
