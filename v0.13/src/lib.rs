use android_logger::Config;
use log::{debug, LevelFilter};
use wgpu::{ColorTargetState, ColorWrites, PresentMode, SurfaceConfiguration, TextureViewDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

const SHADER: &'static str = r#"
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

async fn run(event_loop: EventLoop<()>, window: Window) {
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let adapter = instance
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

    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    // works on my android..
    let swapchain_format = wgpu::TextureFormat::Rgba8UnormSrgb;

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                format: swapchain_format,
                blend: None,
                write_mask: ColorWrites::all(),
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let window_size = window.inner_size();
    let mut surface_configuration = SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: window_size.width,
        height: window_size.height,
        present_mode: PresentMode::Fifo,
    };

    let mut surface = None;

    let start = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                debug!("resized");

                surface_configuration.width = size.width;
                surface_configuration.height = size.height;
            }
            Event::Resumed => {
                debug!("resumed");

                surface = Some(unsafe { instance.create_surface(&window) });
                surface.as_ref().unwrap().configure(&device, &surface_configuration);
                debug!("surface: {:?}", surface);
            }
            Event::Suspended => {
                debug!("suspended");

                surface.take();
            }
            Event::MainEventsCleared => {
                debug!("main events cleared");

                if let Some(_surface) = &surface {
                    let render_texture = _surface.get_current_texture().unwrap();
                    let render_texture_view = render_texture.texture.create_view(&TextureViewDescriptor::default());

                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let t = (start.elapsed().as_secs_f64() / 10.0).sin();
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
                        rpass.set_pipeline(&render_pipeline);
                        rpass.draw(0..3, 0..1);
                    }

                    queue.submit(Some(encoder.finish()));

                    render_texture.present();
                }
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

#[cfg_attr(
target_os = "android",
ndk_glue::main(backtrace = "on", logger(level = "debug", tag = "wgpu"))
)]
fn main() {
    android_logger::init_once(
        Config::default().with_max_level(LevelFilter::Trace),
    );
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    pollster::block_on(run(event_loop, window));
}
