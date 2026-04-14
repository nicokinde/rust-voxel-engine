use glam::{IVec3, Vec3};
use raylib::ffi;
use raylib::prelude::*;

const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

const BLOCK_AIR: u8 = 0;
const BLOCK_GRASS: u8 = 1;

const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_HALF_WIDTH: f32 = 0.3;
const EYE_HEIGHT: f32 = 1.62;
const GRAVITY: f32 = 20.0;
const JUMP_SPEED: f32 = 8.0;
const MOVE_SPEED: f32 = 5.0;
const MOUSE_SENSITIVITY: f32 = 0.003;

// 6 vertices per face (2 triangles), offsets from block origin
const FACE_VERTS: [[[f32; 3]; 6]; 6] = [
    // Top (+Y)
    [[0.0,1.0,1.0],[1.0,1.0,1.0],[1.0,1.0,0.0],[0.0,1.0,1.0],[1.0,1.0,0.0],[0.0,1.0,0.0]],
    // Bottom (-Y)
    [[0.0,0.0,0.0],[1.0,0.0,0.0],[1.0,0.0,1.0],[0.0,0.0,0.0],[1.0,0.0,1.0],[0.0,0.0,1.0]],
    // Right (+X)
    [[1.0,0.0,0.0],[1.0,1.0,0.0],[1.0,1.0,1.0],[1.0,0.0,0.0],[1.0,1.0,1.0],[1.0,0.0,1.0]],
    // Left (-X)
    [[0.0,0.0,1.0],[0.0,1.0,1.0],[0.0,1.0,0.0],[0.0,0.0,1.0],[0.0,1.0,0.0],[0.0,0.0,0.0]],
    // Front (+Z)
    [[0.0,0.0,1.0],[1.0,0.0,1.0],[1.0,1.0,1.0],[0.0,0.0,1.0],[1.0,1.0,1.0],[0.0,1.0,1.0]],
    // Back (-Z)
    [[1.0,0.0,0.0],[0.0,0.0,0.0],[0.0,1.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0],[1.0,1.0,0.0]],
];

const FACE_NORMALS: [[f32; 3]; 6] = [
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

const NEIGHBOR_OFFSETS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
];

// Face tint colors for fake directional lighting (R, G, B, A)
const FACE_COLORS: [[u8; 4]; 6] = [
    [100, 200, 30, 255], // top — brightest
    [50, 100, 15, 255],  // bottom — darkest
    [75, 150, 22, 255],  // right
    [75, 150, 22, 255],  // left
    [60, 120, 18, 255],  // front
    [60, 120, 18, 255],  // back
];

// ---------------------------------------------------------------------------
// Chunk
// ---------------------------------------------------------------------------

struct Chunk {
    blocks: [u8; CHUNK_VOLUME],
    position: IVec3,
}

impl Chunk {
    fn new(position: IVec3) -> Self {
        let mut blocks = [BLOCK_AIR; CHUNK_VOLUME];
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                blocks[Self::index(x, 0, z)] = BLOCK_GRASS;
            }
        }
        Chunk { blocks, position }
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x
    }

    fn get_block(&self, x: usize, y: usize, z: usize) -> u8 {
        self.blocks[Self::index(x, y, z)]
    }

    /// Returns AIR for out-of-bounds, so chunk-edge faces are always drawn.
    fn get_block_safe(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= CHUNK_SIZE as i32 || y < 0 || y >= CHUNK_SIZE as i32 || z < 0 || z >= CHUNK_SIZE as i32 {
            BLOCK_AIR
        } else {
            self.get_block(x as usize, y as usize, z as usize)
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh builder — produces a raw ffi::Mesh with face-culled geometry
// ---------------------------------------------------------------------------

struct ChunkMesh {
    mesh: ffi::Mesh,
    material: ffi::Material,
}

impl ChunkMesh {
    fn build(chunk: &Chunk) -> Self {
        let origin = chunk.position * CHUNK_SIZE as i32;
        let mut positions: Vec<f32> = Vec::new();
        let mut normals: Vec<f32> = Vec::new();
        let mut colors: Vec<u8> = Vec::new();

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if chunk.get_block(x, y, z) == BLOCK_AIR {
                        continue;
                    }

                    let bx = origin.x as f32 + x as f32;
                    let by = origin.y as f32 + y as f32;
                    let bz = origin.z as f32 + z as f32;

                    for face in 0..6 {
                        let n = &NEIGHBOR_OFFSETS[face];
                        let nx = x as i32 + n[0];
                        let ny = y as i32 + n[1];
                        let nz = z as i32 + n[2];

                        if chunk.get_block_safe(nx, ny, nz) != BLOCK_AIR {
                            continue; // neighbor is solid — skip this face
                        }

                        let norm = &FACE_NORMALS[face];
                        let col = &FACE_COLORS[face];

                        for v in &FACE_VERTS[face] {
                            positions.push(bx + v[0]);
                            positions.push(by + v[1]);
                            positions.push(bz + v[2]);

                            normals.push(norm[0]);
                            normals.push(norm[1]);
                            normals.push(norm[2]);

                            colors.push(col[0]);
                            colors.push(col[1]);
                            colors.push(col[2]);
                            colors.push(col[3]);
                        }
                    }
                }
            }
        }

        let vertex_count = (positions.len() / 3) as i32;
        let triangle_count = vertex_count / 3;

        let mut mesh: ffi::Mesh = unsafe { std::mem::zeroed() };
        mesh.vertexCount = vertex_count;
        mesh.triangleCount = triangle_count;

        if vertex_count > 0 {
            mesh.vertices = positions.as_mut_ptr();
            mesh.normals = normals.as_mut_ptr();
            mesh.colors = colors.as_mut_ptr();
            std::mem::forget(positions);
            std::mem::forget(normals);
            std::mem::forget(colors);

            unsafe {
                ffi::UploadMesh(&mut mesh, false);
            }
        }

        let material = unsafe { ffi::LoadMaterialDefault() };

        ChunkMesh { mesh, material }
    }

    fn draw(&self) {
        if self.mesh.vertexCount == 0 {
            return;
        }
        let transform = ffi::Matrix {
            m0: 1.0, m4: 0.0, m8: 0.0,  m12: 0.0,
            m1: 0.0, m5: 1.0, m9: 0.0,  m13: 0.0,
            m2: 0.0, m6: 0.0, m10: 1.0, m14: 0.0,
            m3: 0.0, m7: 0.0, m11: 0.0, m15: 1.0,
        };
        unsafe {
            ffi::DrawMesh(self.mesh, self.material, transform);
        }
    }
}

// ---------------------------------------------------------------------------
// Collision helpers
// ---------------------------------------------------------------------------

fn block_is_solid(chunk: &Chunk, bx: i32, by: i32, bz: i32) -> bool {
    chunk.get_block_safe(bx, by, bz) != BLOCK_AIR
}

fn aabb_collides(chunk: &Chunk, min: Vec3, max: Vec3) -> bool {
    let bx_min = min.x.floor() as i32;
    let by_min = min.y.floor() as i32;
    let bz_min = min.z.floor() as i32;
    let bx_max = (max.x - 0.001).floor() as i32;
    let by_max = (max.y - 0.001).floor() as i32;
    let bz_max = (max.z - 0.001).floor() as i32;

    for by in by_min..=by_max {
        for bz in bz_min..=bz_max {
            for bx in bx_min..=bx_max {
                if block_is_solid(chunk, bx, by, bz) {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

struct Player {
    position: Vec3,
    velocity: Vec3,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
}

impl Player {
    fn new(pos: Vec3) -> Self {
        Self {
            position: pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
        }
    }

    fn update(&mut self, rl: &RaylibHandle, chunk: &Chunk, dt: f32) {
        // Mouse look
        let md = rl.get_mouse_delta();
        self.yaw -= md.x * MOUSE_SENSITIVITY;
        self.pitch -= md.y * MOUSE_SENSITIVITY;
        self.pitch = self.pitch.clamp(-1.5, 1.5);

        // Movement input
        let forward = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());

        let mut dir = Vec3::ZERO;
        if rl.is_key_down(KeyboardKey::KEY_W) { dir += forward; }
        if rl.is_key_down(KeyboardKey::KEY_S) { dir -= forward; }
        if rl.is_key_down(KeyboardKey::KEY_D) { dir += right; }
        if rl.is_key_down(KeyboardKey::KEY_A) { dir -= right; }
        let dir = dir.normalize_or_zero();

        self.velocity.x = dir.x * MOVE_SPEED;
        self.velocity.z = dir.z * MOVE_SPEED;

        // Jump
        if self.on_ground && rl.is_key_down(KeyboardKey::KEY_SPACE) {
            self.velocity.y = JUMP_SPEED;
            self.on_ground = false;
        }

        // Gravity
        self.velocity.y -= GRAVITY * dt;

        // Resolve each axis independently: Y first, then X, then Z
        self.resolve_y(chunk, dt);
        self.resolve_xz(chunk, dt);
    }

    fn resolve_y(&mut self, chunk: &Chunk, dt: f32) {
        let new_y = self.position.y + self.velocity.y * dt;
        let min = Vec3::new(
            self.position.x - PLAYER_HALF_WIDTH,
            new_y,
            self.position.z - PLAYER_HALF_WIDTH,
        );
        let max = Vec3::new(
            self.position.x + PLAYER_HALF_WIDTH,
            new_y + PLAYER_HEIGHT,
            self.position.z + PLAYER_HALF_WIDTH,
        );

        if aabb_collides(chunk, min, max) {
            if self.velocity.y <= 0.0 {
                // Falling — find highest solid block top in the AABB
                let by_min = min.y.floor() as i32;
                let by_max = (max.y - 0.001).floor() as i32;
                let bx_min = min.x.floor() as i32;
                let bx_max = (max.x - 0.001).floor() as i32;
                let bz_min = min.z.floor() as i32;
                let bz_max = (max.z - 0.001).floor() as i32;

                let mut highest_top = new_y;
                for by in by_min..=by_max {
                    for bz in bz_min..=bz_max {
                        for bx in bx_min..=bx_max {
                            if block_is_solid(chunk, bx, by, bz) {
                                highest_top = highest_top.max((by + 1) as f32);
                            }
                        }
                    }
                }
                self.position.y = highest_top;
                self.on_ground = true;
            } else {
                // Rising — find lowest solid block bottom in the AABB
                let by_min = min.y.floor() as i32;
                let by_max = (max.y - 0.001).floor() as i32;
                let bx_min = min.x.floor() as i32;
                let bx_max = (max.x - 0.001).floor() as i32;
                let bz_min = min.z.floor() as i32;
                let bz_max = (max.z - 0.001).floor() as i32;

                let mut lowest_bottom = new_y + PLAYER_HEIGHT;
                for by in by_min..=by_max {
                    for bz in bz_min..=bz_max {
                        for bx in bx_min..=bx_max {
                            if block_is_solid(chunk, bx, by, bz) {
                                lowest_bottom = lowest_bottom.min(by as f32);
                            }
                        }
                    }
                }
                self.position.y = lowest_bottom - PLAYER_HEIGHT;
            }
            self.velocity.y = 0.0;
        } else {
            self.position.y = new_y;
            self.on_ground = false;
        }
    }

    fn resolve_xz(&mut self, chunk: &Chunk, dt: f32) {
        // X axis
        let new_x = self.position.x + self.velocity.x * dt;
        let min = Vec3::new(new_x - PLAYER_HALF_WIDTH, self.position.y, self.position.z - PLAYER_HALF_WIDTH);
        let max = Vec3::new(new_x + PLAYER_HALF_WIDTH, self.position.y + PLAYER_HEIGHT, self.position.z + PLAYER_HALF_WIDTH);
        if !aabb_collides(chunk, min, max) {
            self.position.x = new_x;
        }

        // Z axis
        let new_z = self.position.z + self.velocity.z * dt;
        let min = Vec3::new(self.position.x - PLAYER_HALF_WIDTH, self.position.y, new_z - PLAYER_HALF_WIDTH);
        let max = Vec3::new(self.position.x + PLAYER_HALF_WIDTH, self.position.y + PLAYER_HEIGHT, new_z + PLAYER_HALF_WIDTH);
        if !aabb_collides(chunk, min, max) {
            self.position.z = new_z;
        }
    }

    fn camera(&self) -> Camera3D {
        let eye = self.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        let look_dir = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        let target = eye + look_dir;

        Camera3D::perspective(
            Vector3::new(eye.x, eye.y, eye.z),
            Vector3::new(target.x, target.y, target.z),
            Vector3::new(0.0, 1.0, 0.0),
            60.0,
        )
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("Rust Voxel Engine")
        .build();

    rl.set_target_fps(60);
    rl.disable_cursor();

    let chunk = Chunk::new(IVec3::ZERO);
    let chunk_mesh = ChunkMesh::build(&chunk);

    // Spawn player above the grass floor
    let mut player = Player::new(Vec3::new(8.0, 1.0, 8.0));

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        player.update(&rl, &chunk, dt);

        let camera = player.camera();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(135, 206, 235, 255));

        {
            let _d3 = d.begin_mode3D(camera);
            chunk_mesh.draw();
        }

        d.draw_fps(10, 10);
        d.draw_text("WASD + Mouse | SPACE to jump", 10, 30, 18, Color::WHITE);
    }
}
