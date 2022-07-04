use std::borrow::Cow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

const SHADER: &'static str = r#"
[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] in_vertex_index: u32) -> [[builtin(position)]] vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let instance = wgpu::Instance::new(wgpu::BackendBit::all());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
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
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        flags: wgpu::ShaderFlags::all(),
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
            targets: &[swapchain_format.into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: 0,
        height: 0,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut surface = None;
    let mut swap_chain = None;

    let start = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &shader, &pipeline_layout);

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                println!("resized");
                sc_desc.width = size.width;
                sc_desc.height = size.height;
                swap_chain = Some(device.create_swap_chain(&surface.as_ref().unwrap(), &sc_desc));
            }
            Event::Resumed =>
            {
                println!("resume");
                let size = window.inner_size();
                sc_desc.width = size.width;
                sc_desc.height = size.height;
                surface = Some(unsafe { instance.create_surface(&window) });
                swap_chain = Some(device.create_swap_chain(&surface.as_ref().unwrap(), &sc_desc));
                println!("surface: {:?}", surface);
            },
            Event::Suspended =>
            {
                println!("suspend");
                swap_chain.take();
                surface.take();
            },
            Event::MainEventsCleared => {
                if let Some(swap_chain) = swap_chain.as_mut() {
                    let frame = swap_chain
                        .get_current_frame()
                        .expect("Failed to acquire next swap chain texture")
                        .output;
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let t = (start.elapsed().as_secs_f64() / 10.0).sin();
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[wgpu::RenderPassColorAttachment {
                                view: &frame.view,
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
                            }],
                            depth_stencil_attachment: None,
                        });
                        rpass.set_pipeline(&render_pipeline);
                        rpass.draw(0..3, 0..1);
                    }

                    queue.submit(Some(encoder.finish()));
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on", logger(level = "debug", tag = "wgpu")))]
fn main() {
    log::info!("start");
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    pollster::block_on(run(event_loop, window));
}
