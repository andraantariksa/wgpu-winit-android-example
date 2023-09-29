use std::time::Instant;
use log::debug;
use wgpu::{ColorTargetState, ColorWrites, PresentMode, SurfaceConfiguration, TextureViewDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
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
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader: wgpu::ShaderModule,
    render_pipeline: wgpu::RenderPipeline,
}



struct App {
    window: Window,
    instance: wgpu::Instance,
    renderer: Option<Renderer>,
    surface: Option<wgpu::Surface>,
    last_time: Instant,
}

impl App {
    fn new(window: Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Dxc {
                dxc_path: None,
                dxil_path: None,
            },
        });
        Self {
            window,
            instance,
            renderer: None,
            surface: None,
            last_time: Instant::now(),
        }
    }

    async fn create_renderer(&mut self) {
        let adapter = self.instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: surface_format,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })

        self.renderer = Some(Renderer {
            queue,
            device,
            adapter,
            shader,
            render_pipeline,
        });
    }

    fn configure_swapchain(&mut self) {
        let mut surface_configuration = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width: 1,
            height: 1,
            present_mode: PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Inherit,
            view_formats: vec![],
        };
    }

    fn create_surface(&mut self) {
        let surface = unsafe {
            self.instance.create_surface(&self.window)
        }.unwrap();
        self.surface = Some(surface);
    }

    fn resumed(&mut self) {
        self.create_surface();
        self.create_renderer();
    }

    fn suspended(&mut self) {
        self.renderer.take();
    }

    fn resize(&mut self, window_size: PhysicalSize<u32>) {
        self.configure_swapchain();
    }

    fn render(&mut self) {
        if let (Some(surface), Some(renderer)) = (&self.surface, &self.renderer) {
            let render_texture = surface.get_current_texture().unwrap();
            let render_texture_view = render_texture.texture.create_view(&TextureViewDescriptor::default());

            let mut encoder = renderer.device
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
                rpass.set_pipeline(renderer.render_pipeline.as_ref().unwrap());
                rpass.draw(0..3, 0..1);
            }

            renderer.queue.submit(Some(encoder.finish()));

            render_texture.present();
        }
    }
}

pub async fn run<T: std::fmt::Debug>(mut event_loop: EventLoop<T>) {
    let window = Window::new(&event_loop).unwrap();
    let mut app = App::new(window);

    // It's not recommended to use `run` on Android because it will call
    // `std::process::exit` when finished which will short-circuit any
    // Java lifecycle handling
    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
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

                app.resumed();

                // let _surface = unsafe { instance.create_surface(&window) }.unwrap();
                // let surface_caps = _surface.get_capabilities(&adapter);
                // let surface_format = surface_caps.formats.iter()
                //     .copied().find(|f| f.is_srgb())
                //     .unwrap_or(surface_caps.formats[0]);
                // surface_configuration.present_mode = surface_caps.present_modes[0];
                // surface_configuration.alpha_mode = surface_caps.alpha_modes[0];
                // surface_configuration.format = surface_format;
                //
                // render_pipeline = Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                //     label: None,
                //     layout: Some(&pipeline_layout),
                //     vertex: wgpu::VertexState {
                //         module: &shader,
                //         entry_point: "vs_main",
                //         buffers: &[],
                //     },
                //     fragment: Some(wgpu::FragmentState {
                //         module: &shader,
                //         entry_point: "fs_main",
                //         targets: &[Some(ColorTargetState {
                //             format: surface_format,
                //             blend: None,
                //             write_mask: ColorWrites::all(),
                //         })],
                //     }),
                //     primitive: wgpu::PrimitiveState::default(),
                //     depth_stencil: None,
                //     multisample: wgpu::MultisampleState::default(),
                //     multiview: None,
                // }));
                //
                // _surface.configure(&device, &surface_configuration);
                // surface = Some(_surface);
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