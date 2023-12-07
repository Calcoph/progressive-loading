use imgui::Context;
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::WinitPlatform;
use te_renderer::{initial_config::InitialConfiguration, state::{GpuState, TeState}};
use winit::{window::Window, event::WindowEvent};

use crate::ActionTaken;

use self::data::{DataState, ServerData, ClientData};
mod data;

pub struct ImState {
    gpu: GpuState,
    pub context: Context,
    pub platform: WinitPlatform,
    renderer: Renderer,
    pub state: TeState,

    data: DataState
}

impl ImState {
    pub(crate) async fn new(window: &Window, config: InitialConfiguration) -> ImState {
        let size = window.inner_size();
        let gpu = GpuState::new(size, window).await;
        let state = TeState::new(window, &gpu, config.clone()).await;
        let mut context = imgui::Context::create();
        context.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut context);
        platform.attach_window(context.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);

        let renderer_config = RendererConfig {
            texture_format: gpu.config.format,
            ..Default::default()
        };

        let mut renderer = Renderer::new(&mut context, &gpu.device, &gpu.queue, renderer_config);

        ImState {
            data: DataState::new(&gpu.device, &mut renderer, &gpu.queue),
            gpu,
            context,
            platform,
            renderer,
            state,
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.gpu.resize(new_size);
            self.state.resize(new_size)
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.state.input(event)
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.state.update(dt, &self.gpu);
    }

    pub(crate) fn render(&mut self, window: &Window) -> Result<Option<ActionTaken>, wgpu::SurfaceError> {
        let output = self.gpu.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        render_state(&view, &mut self.state, &self.gpu);
        let action_taken = self.render_imgui(&view, window);
        output.present();

        Ok(action_taken)
    }

    pub(crate) fn render_imgui(&mut self, view: &wgpu::TextureView, window: &Window) -> Option<ActionTaken> {
        self.platform.prepare_frame(self.context.io_mut(), window).expect("Failed to prepare frame");
        let ui = self.context.frame();
        ui.dockspace_over_main_viewport();
        let action_taken = {
            let mut action_taken = None;

            if let None = action_taken {
                if let Some(action) = sender_window(ui, &self.data.server) {
                    action_taken = Some(action);
                };
            }

            if let None = action_taken {
                if let Some(action) = receiver_window(ui, &self.data.client) {
                    action_taken = Some(action);
                };
            }

            action_taken
        };

        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ImGui Render Encoder")
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.render(self.context.render(), &self.gpu.queue, &self.gpu.device, &mut render_pass).expect("Rendering failed");
        }
        self.gpu.queue.submit(std::iter::once(encoder.finish())); // TODO: call submit only once. right now it is called twice
        action_taken
    }

    pub(crate) fn send(&mut self) {
        let data = self.data.server.send();
        self.data.client.receive(&data);
    }

    pub(crate) fn clear(&mut self) {
        self.data.server.clear();
        self.data.client.clear(&self.gpu.device, &mut self.renderer, &self.gpu.queue);
    }
}

fn render_state(view: &wgpu::TextureView, state: &mut TeState, gpu: &GpuState) {
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder")
    });
    {
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0
                            }),
                            store: wgpu::StoreOp::Store
                        }
                    })
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gpu.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store
                    }),
                    stencil_ops: None
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            state.draw_opaque(&mut render_pass, &state.pipelines.render_3d);
            state.draw_transparent(&mut render_pass, &state.pipelines.transparent);
        }
    }

    gpu.queue.submit(std::iter::once(encoder.finish()));
}

fn receiver_window(ui: &mut imgui::Ui, data: &ClientData) -> Option<ActionTaken> {
    let mut action = None;
    ui.window("Receiver").build(|| {
        imgui::Image::new(data.texture_id, data.size).build(ui);
        if ui.button("Clear") {
            action = Some(ActionTaken::Clear)
        }
    });

    action
}

fn sender_window(ui: &mut imgui::Ui, data: &ServerData) -> Option<ActionTaken> {
    let mut action = None;
    ui.window("Sender").build(|| {
        let size = [data.image_size[0] / 4.0, data.image_size[1] / 4.0];
        imgui::Image::new(data.texture_id, size).build(ui);
        if ui.button("Send") {
            action = Some(ActionTaken::Send)
        }
    });

    action
}
