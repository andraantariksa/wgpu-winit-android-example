use std::sync::Arc;
use std::time::Instant;
use log::debug;
use wgpu::{ColorTargetState, ColorWrites, PresentMode, SurfaceConfiguration, TextureViewDescriptor};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use winit::dpi::PhysicalSize;

const SHADER: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

struct Renderer {
    render_pipeline: wgpu::RenderPipeline,
}

struct SurfaceState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    view_format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
}

struct App {
    instance: wgpu::Instance,
    renderer: Option<Renderer>,
    surface_state: Option<SurfaceState>,
    last_time: Instant,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl App {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: if cfg!(not(target_os = "android")) {
                wgpu::Backends::all()
            } else {
                wgpu::Backends::GL
            },
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let limits = wgpu::Limits::downlevel_webgl2_defaults();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .expect("Failed to create device");
        Self {
            instance,
            device,
            adapter,
            queue,
            renderer: None,
            surface_state: None,
            last_time: Instant::now(),
        }
    }

    async fn create_renderer(&mut self) {
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: self.surface_state.as_ref().unwrap().view_format,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        self.renderer = Some(Renderer {
            render_pipeline,
        });
    }

    fn setup_swapchain(&mut self, size: PhysicalSize<u32>) {
        let surface_state = self.surface_state.as_ref().unwrap();
        let surface_configuration = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_state.view_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_state.alpha_mode,
            view_formats: vec![surface_state.view_format],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface_state.surface.configure(&self.device, &surface_configuration);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let surface = self.instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&self.adapter);
        let size = window.inner_size();
        self.surface_state = Some(SurfaceState {
            window,
            surface,
            view_format: cap.formats[0],
            alpha_mode: cap.alpha_modes[0],
        });

        self.setup_swapchain(size);
        pollster::block_on(self.create_renderer());
        self.surface_state.as_ref().unwrap().window.request_redraw();
    }

    fn suspended(&mut self) {
        self.renderer.take();
        self.surface_state.take();
    }

    fn resize(&mut self, window_size: PhysicalSize<u32>) {
        self.setup_swapchain(window_size);
    }

    fn render(&mut self) {
        if let (Some(surface_state), Some(renderer)) = (&self.surface_state, &self.renderer) {
            let render_texture = match surface_state.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                other => panic!("Failed to acquire surface texture: {:?}", other),
            };
            let render_texture_view = render_texture.texture.create_view(&TextureViewDescriptor::default());

            let mut encoder = self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let t = (self.last_time.elapsed().as_secs_f64() / 5.0).sin();
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &render_texture_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: t,
                                b: 1.0 - t,
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
                rpass.set_pipeline(&renderer.render_pipeline);
                rpass.draw(0..3, 0..1);
            }

            self.queue.submit(Some(encoder.finish()));

            self.queue.present(render_texture);
            surface_state.window.request_redraw();
        }
    }
}

impl<T: 'static> ApplicationHandler<T> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        debug!("resumed");
        App::resumed(self, event_loop);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        debug!("suspended");
        App::suspended(self);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                debug!("resized");
                self.resize(size);
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CloseRequested => {
                debug!("quit");
                event_loop.exit();
            }
            e => {
                debug!("other event {:?}", e);
            }
        }
    }
}

pub fn run<T: std::fmt::Debug>(event_loop: EventLoop<T>) {
    let mut app = pollster::block_on(App::new());
    event_loop.run_app(&mut app).unwrap();
}
