use glam::IVec3;
use raylib::prelude::*;

const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

const BLOCK_AIR: u8 = 0;
const BLOCK_GRASS: u8 = 1;

struct Chunk {
    blocks: [u8; CHUNK_VOLUME],
    position: IVec3,
}

impl Chunk {
    fn new(position: IVec3) -> Self {
        let mut blocks = [BLOCK_AIR; CHUNK_VOLUME];

        // Fill bottom layer (y=0) with grass
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                blocks[Self::index(x, 0, z)] = BLOCK_GRASS;
            }
        }

        Chunk { blocks, position }
    }

    /// Convert 3D local coordinate (0-15) to 1D array index.
    fn index(x: usize, y: usize, z: usize) -> usize {
        y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x
    }

    fn get_block(&self, x: usize, y: usize, z: usize) -> u8 {
        self.blocks[Self::index(x, y, z)]
    }

    /// Generate cube meshes for all non-air blocks and draw them.
    fn draw(&self, d: &mut RaylibMode3D<'_, RaylibDrawHandle<'_>>) {
        let origin = self.position * CHUNK_SIZE as i32;

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if self.get_block(x, y, z) == BLOCK_AIR {
                        continue;
                    }

                    let wx = origin.x as f32 + x as f32;
                    let wy = origin.y as f32 + y as f32;
                    let wz = origin.z as f32 + z as f32;

                    let pos = Vector3::new(wx + 0.5, wy + 0.5, wz + 0.5);
                    let size = Vector3::new(1.0, 1.0, 1.0);

                    d.draw_cube_v(pos, size, Color::new(76, 153, 0, 255));
                    d.draw_cube_wires(pos, size.x, size.y, size.z, Color::BLACK);
                }
            }
        }
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("Rust Voxel Engine")
        .build();

    rl.set_target_fps(60);

    let mut camera = Camera3D::perspective(
        Vector3::new(8.0, 5.0, 20.0),  // position
        Vector3::new(8.0, 0.0, 8.0),   // target
        Vector3::new(0.0, 1.0, 0.0),   // up
        60.0,                           // fov
    );

    rl.disable_cursor();

    let chunk = Chunk::new(IVec3::ZERO);

    while !rl.window_should_close() {
        rl.update_camera(&mut camera, CameraMode::CAMERA_FIRST_PERSON);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(135, 206, 235, 255)); // sky blue

        {
            let mut d3 = d.begin_mode3D(camera);
            chunk.draw(&mut d3);
        }

        d.draw_fps(10, 10);
        d.draw_text("WASD to move, Mouse to look", 10, 30, 18, Color::WHITE);
    }
}
