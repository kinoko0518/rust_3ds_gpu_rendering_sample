use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, process::Command};

use const_format::concatcp;
use image::open;
use swizzle_3ds::pix::{ImageFormat, ImageView};

const ASSETS: &str = concatcp!(env!("CARGO_MANIFEST_DIR"), "/assets");

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir: &str = out_dir.to_str().unwrap();
    let out_path = PathBuf::from_str(out_dir).unwrap();

    // 画像の変換
    const IMAGE_PATH: &str = concatcp!(ASSETS, "/ferris.png");
    println!("cargo:rerun-if-changed={}", IMAGE_PATH);

    let img = open(IMAGE_PATH).expect("Failed to open image file");
    let (width, height) = (img.width(), img.height());

    assert!(width.is_power_of_two() && height.is_power_of_two());

    let img_view = ImageView::new(
        img.as_bytes(),
        width as usize,
        height as usize,
        ImageFormat::Rgba8,
    );

    let swizzled_pixels = swizzle_3ds::swizzle_image(&img_view);

    let dest_path = out_path.join("swizzled_ferris.bin");
    let mut f = File::create(&dest_path).unwrap();

    f.write_all(swizzled_pixels.as_raw()).unwrap();

    // シェーダのコンパイル
    const SHADER_PATH: &str = concatcp!(ASSETS, "/sprite.v.pica");
    println!("cargo:rerun-if-changed={}", SHADER_PATH);

    Command::new("picasso")
        .arg(SHADER_PATH)
        .arg("-o")
        .arg(out_path.join("sprite.shbin"))
        .status()
        .unwrap();
}
