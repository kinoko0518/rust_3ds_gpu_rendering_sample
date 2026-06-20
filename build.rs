use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, process::Command};

use const_format::concatcp;
use image::open;
use swizzle_3ds::pix::{ImageFormat, ImageView};

const ASSETS: &str = concatcp!(env!("CARGO_MANIFEST_DIR"), "/assets");

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out_dir);

    // 画像の変換
    const IMAGE_PATH: &str = concatcp!(ASSETS, "/ferris.png");
    println!("cargo:rerun-if-changed={}", IMAGE_PATH);

    let img = open(IMAGE_PATH).expect("Failed to open image file");
    let (width, height) = (img.width(), img.height());

    assert!(width.is_power_of_two() && height.is_power_of_two());

    // ミップマップを作成
    let mipmaps_count = width.ilog2().max(height.ilog2());
    let mipmaps = (0..=mipmaps_count)
        .flat_map(|level| {
            // 仕様上はオリジナルのサイズから1x1になるまでミップマップを作らないといけないが、
            // 3DSのGPUが読み込む1ブロックは8x8が最小
            let nwidth = (width >> level).max(8);
            let nheight = (height >> level).max(8);
            let resized = img.resize(nwidth, nheight, image::imageops::FilterType::Nearest);
            let img_view = ImageView::new(
                resized.as_bytes(),
                nwidth as usize,
                nheight as usize,
                ImageFormat::Rgba8,
            );
            // Swizzle処理を適用
            let swizzled_pixels = swizzle_3ds::swizzle_image(&img_view);
            swizzled_pixels.as_raw().to_vec()
        })
        .collect::<Vec<u8>>();
    let dest_path = out_path.join("swizzled_ferris.bin");
    let mut f = File::create(&dest_path).unwrap();
    f.write_all(&mipmaps).unwrap();

    // シェーダのコンパイル
    const SHADER_PATH: &str = concatcp!(ASSETS, "/sprite.v.pica");
    println!("cargo:rerun-if-changed={}", SHADER_PATH);

    let status = Command::new("picasso")
        .arg(SHADER_PATH)
        .arg("-o")
        .arg(out_path.join("sprite.shbin"))
        .status()
        .unwrap();

    assert!(
        status.success(),
        "Failed to compile the shader with picasso"
    );
}
