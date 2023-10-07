use std::time::Instant;
use log::debug;
use wgpu::{ColorTargetState, ColorWrites, PresentMode, SurfaceConfiguration, TextureViewDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::Window,
};
use winit::dpi::PhysicalSize;
use winit::platform::run_return::EventLoopExtRunReturn;

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
    surface: wgpu::Surface,
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
            dx12_shader_compiler: wgpu::Dx12Compiler::Dxc {
                dxc_path: None,
                dxil_path: None,
            },
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let limits = wgpu::Limits::downlevel_webgl2_defaults();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits,
                },
                None,
            )
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
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: self.surface_state.as_ref().unwrap().view_format,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
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
        };
        surface_state.surface.configure(&self.device, &surface_configuration);
    }

    fn resumed<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        let window = Window::new(event_loop).unwrap();
        let surface = unsafe {
            self.instance.create_surface(&window)
        }.unwrap();
        let cap = surface.get_capabilities(&self.adapter);
        self.surface_state = Some(SurfaceState {
            surface,
            view_format: cap.formats[0],
            alpha_mode: cap.alpha_modes[0],
        });

        self.setup_swapchain(window.inner_size());
        pollster::block_on(self.create_renderer());
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
            let render_texture = surface_state.surface.get_current_texture().unwrap();
            let render_texture_view = render_texture.texture.create_view(&TextureViewDescriptor::default());

            let mut encoder = self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let t = (self.last_time.elapsed().as_secs_f64() / 5.0).sin();
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &render_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: t,
                                b: 1.0 - t,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                rpass.set_pipeline(&renderer.render_pipeline);
                rpass.draw(0..3, 0..1);
            }

            self.queue.submit(Some(encoder.finish()));

            render_texture.present();
        }
    }
}

pub fn run<T: std::fmt::Debug>(mut event_loop: EventLoop<T>) {
    let mut app = pollster::block_on(App::new());

    event_loop.run_return(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                debug!("resized");

                app.resize(size);
            }
            Event::Resumed => {
                debug!("resumed");

                app.resumed(event_loop);
            }
            Event::Suspended => {
                debug!("suspended");

                app.suspended();
            }
            Event::MainEventsCleared => {
                debug!("main events cleared");

                app.render();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                debug!("quit");

                *control_flow = ControlFlow::Exit
            }
            e => {
                debug!("other event {:?}", e);
            }
        }
    });
}