# `wgpu` & `winit` Android Example

This example use `wgpu` v0.13, checkout the other branch for the other version.

Tested on

- Realme 9 Pro Android 12
- Asus Zenfone Max Pro Android 9

using Vulkan and OpenGL backend.

## Requirements

- `cargo apk`. You can install it by running `cargo install cargo-apk`
- Toolchain target. You can install it by running `rustup target install armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android`

## Getting Started

1. Connect Android device
2. `cargo apk run`

## Screenshot

You should see a triangle as below

![Triangle](assets/1.jpg)
