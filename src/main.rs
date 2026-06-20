use std::time::Instant;

use citro3d::{
    Instance,
    attrib::{Permutation, Register},
    buffer::{Buffer, Info},
    render::{ScreenTarget, Target},
    shader::{Library, Program},
    texture::{Face, Texture, TextureParameters},
    uniform::Index,
};
use ctru::prelude::*;

const SCREEN_WIDTH: usize = 240;
const SCREEN_HEIGHT: usize = 400;

const IMAGE_X_SIZE: u16 = 256;
const IMAGE_Y_SIZE: u16 = 256;

const DISP_SIZE: f32 = 128.;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn layout_permutation() -> Permutation {
        Permutation::from_layout(&[Register::V0, Register::V1]).unwrap()
    }
}

fn create_texture(image: &[u8], img_size: (u16, u16)) -> Texture {
    // テクスチャの生成
    let face = Face::default();
    let mut texture = Texture::new(TextureParameters::new_2d(
        img_size.0,
        img_size.1,
        citro3d::texture::ColorFormat::Rgba8,
    ))
    .unwrap();
    texture.load_image(image, face).unwrap();

    texture
}

fn create_3d_model(disp_size: (f32, f32)) -> Info {
    // 3Dモデルを定義
    const Z: f32 = 0.;
    let vertexs: [Vertex; 4] = [
        Vertex {
            pos: [0.0, 0.0, Z],
            uv: [0.0, 0.0],
        },
        Vertex {
            pos: [disp_size.0, 0.0, Z],
            uv: [1.0, 0.0],
        },
        Vertex {
            pos: [0.0, disp_size.1, Z],
            uv: [0.0, 1.0],
        },
        Vertex {
            pos: [disp_size.0, disp_size.1, Z],
            uv: [1.0, 1.0],
        },
    ];

    // LinearMemoryに確保したメモリ上に頂点座標をコピー
    let buff = Buffer::new(&vertexs);
    let mut info = citro3d::buffer::Info::default();

    info.add(buff.clone(), Vertex::layout_permutation())
        .unwrap();

    info
}

fn render_frame(
    texture: &Texture,
    info: &Info,
    coord: (f32, f32, f32),
    target: &mut ScreenTarget<'static>,
    citro: &mut Instance,
    program: &Program,
    pm_idx: Index,
) {
    // プロジェクション行列を定義
    let projection_matrix: citro3d::math::Matrix4 = citro3d::math::Projection::orthographic(
        0.0..SCREEN_HEIGHT as f32,
        0.0..SCREEN_WIDTH as f32,
        citro3d::math::ClipPlanes {
            near: 0.0,
            far: 1.0,
        },
    )
    .into();

    let mut attr_info = citro3d::attrib::Info::new();
    attr_info
        .add_loader(
            citro3d::attrib::Register::V0,
            citro3d::attrib::Format::Float,
            3,
        )
        .unwrap();
    attr_info
        .add_loader(
            citro3d::attrib::Register::V1,
            citro3d::attrib::Format::Float,
            2,
        )
        .unwrap();

    let texenv = citro3d::texenv::TexEnv::new()
        .src(
            citro3d::texenv::Mode::BOTH,
            citro3d::texenv::Source::Texture0,
            None,
            None,
        )
        .func(
            citro3d::texenv::Mode::BOTH,
            citro3d::texenv::CombineFunc::Replace,
        );

    // 画面クリア
    target.clear(citro3d::render::ClearFlags::ALL, 0xFF000000, 0);

    citro.render_frame_with(|mut frame| {
        frame.select_render_target(target).unwrap();
        frame.set_attr_info(&attr_info);
        frame.bind_program(&program);
        frame.set_texenvs(&[texenv.clone()]);

        let mut matrix = projection_matrix;
        unsafe {
            // 座標をGPUに教える
            citro3d_sys::Mtx_Translate(matrix.as_raw_mut(), coord.0, coord.1, coord.2, true);
        }

        frame.bind_vertex_uniform(pm_idx, &matrix);
        frame.bind_texture(citro3d::texture::Index::Texture0, texture);

        frame
            .draw_arrays(citro3d::buffer::Primitive::TriangleStrip, info, None)
            .unwrap();

        frame
    });
}

fn main() {
    let apt = Apt::new().unwrap();
    let gfx: &'static Gfx = Box::leak(Box::new(Gfx::new().unwrap()));
    let mut citro: Instance = citro3d::Instance::new().unwrap();

    // シェーダの構築
    static SHADER_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sprite.shbin"));
    let library = Library::from_bytes(SHADER_BYTES).unwrap();

    let binary = library.get(0).unwrap();
    let program = Program::new(binary).expect("Failed to create shader program");

    let pm_uniform_idx = program
        .get_vertex_uniform("pm")
        .expect("Uniform 'pm' not found in shader");

    // スクリーンターゲットの構築
    let top_screen = gfx.top_screen.borrow_mut();
    let mut target: ScreenTarget<'static> = citro
        .render_target(SCREEN_WIDTH, SCREEN_HEIGHT, top_screen, None)
        .unwrap();

    // テクスチャと3Dモデルを構築
    let image = include_bytes!(concat!(env!("OUT_DIR"), "/swizzled_ferris.bin"));
    let texture = create_texture(image, (IMAGE_X_SIZE, IMAGE_Y_SIZE));
    let model = create_3d_model((DISP_SIZE, DISP_SIZE));

    let instant = Instant::now();
    while apt.main_loop() {
        gfx.wait_for_vblank();

        // Ferrisくんを往復させる
        const SECS_PER_ROUND_TRIP: f32 = 4.;
        let progress: f32 =
            (instant.elapsed().as_secs_f32() % SECS_PER_ROUND_TRIP) / SECS_PER_ROUND_TRIP;
        let x = if progress < 0.5 {
            // 右に進んでいるときの座標計算
            let right_prog = progress * 2.;
            SCREEN_WIDTH as f32 * right_prog
        } else {
            // 左に進んでいるときの座標計算
            let left_prog = (progress % 0.5) * 2.;
            SCREEN_WIDTH as f32 * (1. - left_prog)
        };

        render_frame(
            &texture,
            &model,
            (x, 0., 0.),
            &mut target,
            &mut citro,
            &program,
            pm_uniform_idx,
        );
    }
}
