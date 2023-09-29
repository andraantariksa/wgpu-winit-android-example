#[cfg(target_os = "android")]
use winit::event_loop::EventLoopBuilder;
#[cfg(target_os = "android")]
use log::LevelFilter;
use winit::event_loop::EventLoop;
#[cfg(target_os = "android")]
use winit::platform::android::{EventLoopBuilderExtAndroid, activity::AndroidApp};

mod app;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(LevelFilter::Trace));

    let event_loop: EventLoop<AndroidApp> = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    pollster::block_on(app::run(event_loop));
}

// #[cfg(not(target_os = "android"))]
pub fn main() {
    let event_loop = EventLoop::new();
    pollster::block_on(app::run(event_loop));
}
