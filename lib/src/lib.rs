use winit::event_loop::EventLoop;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

mod app;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use winit::event_loop::EventLoopBuilder;
    use winit::platform::android::EventLoopBuilderExtAndroid;
    use log::LevelFilter;

    android_logger::init_once(android_logger::Config::default().with_max_level(LevelFilter::Trace));

    let event_loop: EventLoop<AndroidApp> = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    app::run(event_loop);
}

#[allow(dead_code)]
fn main() {
    let event_loop = EventLoop::new();
    app::run(event_loop);
}
