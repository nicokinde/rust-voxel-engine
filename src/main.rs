use glam::{IVec3, Vec3};
use raylib::ffi;
use raylib::prelude::*;

const CHUNK_SIZE: i32 = 16;
const CHUNK_USIZE: usize = CHUNK_SIZE as usize;
const CHUNK_VOLUME: usize = CHUNK_USIZE * CHUNK_USIZE * CHUNK_USIZE;

const BLOCK_AIR: u8 = 0;
const BLOCK_GRASS: u8 = 1;
const BLOCK_DIRT: u8 = 2;
const BLOCK_STONE: u8 = 3;
const BLOCK_WOOD: u8 = 4;
const BLOCK_COUNT: u8 = 5;

/// Per-block base colors (R, G, B). Face tinting is applied on top.
const BLOCK_COLORS: [[u8; 3]; BLOCK_COUNT as usize] = [
    [0, 0, 0],         // air (unused)
    [100, 200, 30],    // grass — green
    [140, 100, 60],    // dirt — brown
    [130, 130, 130],   // stone — grey
    [160, 120, 60],    // wood — tan
];

const WORLD_CHUNKS: i32 = 4; // 4x4x4 chunks = 64x64x64 blocks
const WORLD_BLOCKS: i32 = WORLD_CHUNKS * CHUNK_SIZE;

const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_HALF_WIDTH: f32 = 0.3;
const EYE_HEIGHT: f32 = 1.62;
const GRAVITY: f32 = 20.0;
const JUMP_SPEED: f32 = 8.0;
const MOVE_SPEED: f32 = 5.0;
const MOUSE_SENSITIVITY: f32 = 0.003;
const REACH_DISTANCE: f32 = 6.0;

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

/// Face brightness multipliers (0.0–1.0) for fake directional lighting.
const FACE_BRIGHTNESS: [f32; 6] = [
    1.0,  // top — full brightness
    0.5,  // bottom — darkest
    0.75, // right
    0.75, // left
    0.6,  // front
    0.6,  // back
];

fn block_face_color(block_id: u8, face: usize) -> [u8; 4] {
    let base = BLOCK_COLORS[block_id as usize];
    let b = FACE_BRIGHTNESS[face];
    [
        (base[0] as f32 * b) as u8,
        (base[1] as f32 * b) as u8,
        (base[2] as f32 * b) as u8,
        255,
    ]
}

// ---------------------------------------------------------------------------
// Terrain generation
// ---------------------------------------------------------------------------

fn terrain_height(wx: i32, wz: i32) -> i32 {
    let x = wx as f32;
    let z = wz as f32;
    let h = 16.0
        + (x * 0.05).sin() * 4.0
        + (z * 0.07).sin() * 4.0
        + (x * 0.1 + z * 0.1).sin() * 2.0
        + (x * 0.02 + z * 0.03).cos() * 6.0;
    (h as i32).clamp(1, WORLD_BLOCKS - 1)
}

// ---------------------------------------------------------------------------
// Chunk
// ---------------------------------------------------------------------------

struct Chunk {
    blocks: [u8; CHUNK_VOLUME],
    position: IVec3, // chunk coordinate (0..3 on each axis)
}

impl Chunk {
    fn generate(position: IVec3) -> Self {
        let mut blocks = [BLOCK_AIR; CHUNK_VOLUME];
        let origin = position * CHUNK_SIZE;

        for lz in 0..CHUNK_USIZE {
            for lx in 0..CHUNK_USIZE {
                let wx = origin.x + lx as i32;
                let wz = origin.z + lz as i32;
                let height = terrain_height(wx, wz);

                for ly in 0..CHUNK_USIZE {
                    let wy = origin.y + ly as i32;
                    if wy > height {
                        continue;
                    }
                    let depth = height - wy;
                    blocks[Self::index(lx, ly, lz)] = if depth == 0 {
                        BLOCK_GRASS // top layer
                    } else if depth <= 3 {
                        BLOCK_DIRT // 3 layers of dirt
                    } else {
                        BLOCK_STONE // everything below
                    };
                }
            }
        }

        Chunk { blocks, position }
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        y * CHUNK_USIZE * CHUNK_USIZE + z * CHUNK_USIZE + x
    }

    fn get_block(&self, x: usize, y: usize, z: usize) -> u8 {
        self.blocks[Self::index(x, y, z)]
    }

    fn set_block(&mut self, x: usize, y: usize, z: usize, block: u8) {
        self.blocks[Self::index(x, y, z)] = block;
    }
}

// ---------------------------------------------------------------------------
// World — fixed 4x4x4 grid of chunks
// ---------------------------------------------------------------------------

struct World {
    chunks: Vec<Chunk>, // flat array: cy * 16 + cz * 4 + cx
}

impl World {
    fn generate() -> Self {
        let total = (WORLD_CHUNKS * WORLD_CHUNKS * WORLD_CHUNKS) as usize;
        let mut chunks = Vec::with_capacity(total);
        for cy in 0..WORLD_CHUNKS {
            for cz in 0..WORLD_CHUNKS {
                for cx in 0..WORLD_CHUNKS {
                    chunks.push(Chunk::generate(IVec3::new(cx, cy, cz)));
                }
            }
        }
        World { chunks }
    }

    fn chunk_index(cx: i32, cy: i32, cz: i32) -> Option<usize> {
        if cx < 0 || cx >= WORLD_CHUNKS || cy < 0 || cy >= WORLD_CHUNKS || cz < 0 || cz >= WORLD_CHUNKS {
            return None;
        }
        Some((cy * WORLD_CHUNKS * WORLD_CHUNKS + cz * WORLD_CHUNKS + cx) as usize)
    }

    fn get_block(&self, wx: i32, wy: i32, wz: i32) -> u8 {
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cy = wy.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE) as usize;
        let ly = wy.rem_euclid(CHUNK_SIZE) as usize;
        let lz = wz.rem_euclid(CHUNK_SIZE) as usize;

        match Self::chunk_index(cx, cy, cz) {
            Some(idx) => self.chunks[idx].get_block(lx, ly, lz),
            None => BLOCK_AIR,
        }
    }

    fn set_block(&mut self, wx: i32, wy: i32, wz: i32, block: u8) -> Option<usize> {
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cy = wy.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE) as usize;
        let ly = wy.rem_euclid(CHUNK_SIZE) as usize;
        let lz = wz.rem_euclid(CHUNK_SIZE) as usize;

        let idx = Self::chunk_index(cx, cy, cz)?;
        self.chunks[idx].set_block(lx, ly, lz, block);
        Some(idx)
    }

    fn in_bounds(wx: i32, wy: i32, wz: i32) -> bool {
        wx >= 0 && wx < WORLD_BLOCKS && wy >= 0 && wy < WORLD_BLOCKS && wz >= 0 && wz < WORLD_BLOCKS
    }

    /// Returns the chunk indices that should be rebuilt when a block at (wx,wy,wz) changes.
    /// Includes the chunk itself plus neighbors if the block is on a chunk boundary.
    fn dirty_chunks_for_block(&self, wx: i32, wy: i32, wz: i32) -> Vec<usize> {
        let mut result = Vec::new();
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cy = wy.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);

        if let Some(idx) = Self::chunk_index(cx, cy, cz) {
            result.push(idx);
        }

        let lx = wx.rem_euclid(CHUNK_SIZE);
        let ly = wy.rem_euclid(CHUNK_SIZE);
        let lz = wz.rem_euclid(CHUNK_SIZE);

        // If on a boundary, also rebuild the neighbor chunk
        if lx == 0 { if let Some(i) = Self::chunk_index(cx - 1, cy, cz) { result.push(i); } }
        if lx == CHUNK_SIZE - 1 { if let Some(i) = Self::chunk_index(cx + 1, cy, cz) { result.push(i); } }
        if ly == 0 { if let Some(i) = Self::chunk_index(cx, cy - 1, cz) { result.push(i); } }
        if ly == CHUNK_SIZE - 1 { if let Some(i) = Self::chunk_index(cx, cy + 1, cz) { result.push(i); } }
        if lz == 0 { if let Some(i) = Self::chunk_index(cx, cy, cz - 1) { result.push(i); } }
        if lz == CHUNK_SIZE - 1 { if let Some(i) = Self::chunk_index(cx, cy, cz + 1) { result.push(i); } }

        result
    }
}

// ---------------------------------------------------------------------------
// Mesh builder — produces a raw ffi::Mesh with face-culled geometry
// ---------------------------------------------------------------------------

struct ChunkMesh {
    mesh: ffi::Mesh,
    material: ffi::Material,
    has_data: bool,
}

impl ChunkMesh {
    fn build(chunk: &Chunk, world: &World) -> Self {
        let origin = chunk.position * CHUNK_SIZE;
        let mut positions: Vec<f32> = Vec::new();
        let mut normals: Vec<f32> = Vec::new();
        let mut colors: Vec<u8> = Vec::new();

        for y in 0..CHUNK_USIZE {
            for z in 0..CHUNK_USIZE {
                for x in 0..CHUNK_USIZE {
                    let block_id = chunk.get_block(x, y, z);
                    if block_id == BLOCK_AIR {
                        continue;
                    }

                    let wx = origin.x + x as i32;
                    let wy = origin.y + y as i32;
                    let wz = origin.z + z as i32;

                    for face in 0..6 {
                        let n = &NEIGHBOR_OFFSETS[face];
                        let nwx = wx + n[0];
                        let nwy = wy + n[1];
                        let nwz = wz + n[2];

                        // Use world lookup for cross-chunk face culling
                        if world.get_block(nwx, nwy, nwz) != BLOCK_AIR {
                            continue;
                        }

                        let norm = &FACE_NORMALS[face];
                        let bx = wx as f32;
                        let by = wy as f32;
                        let bz = wz as f32;
                        let col = block_face_color(block_id, face);

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

        let has_data = vertex_count > 0;
        if has_data {
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
        ChunkMesh { mesh, material, has_data }
    }

    fn draw(&self) {
        if !self.has_data {
            return;
        }
        let identity = ffi::Matrix {
            m0: 1.0, m4: 0.0, m8: 0.0,  m12: 0.0,
            m1: 0.0, m5: 1.0, m9: 0.0,  m13: 0.0,
            m2: 0.0, m6: 0.0, m10: 1.0, m14: 0.0,
            m3: 0.0, m7: 0.0, m11: 0.0, m15: 1.0,
        };
        unsafe {
            ffi::DrawMesh(self.mesh, self.material, identity);
        }
    }

    fn unload(&mut self) {
        if self.has_data {
            unsafe { ffi::UnloadMesh(self.mesh); }
            self.has_data = false;
        }
    }
}

// ---------------------------------------------------------------------------
// DDA Voxel Raycast
// ---------------------------------------------------------------------------

struct RayHit {
    block: IVec3,
    adjacent: IVec3,
}

fn raycast_voxel(world: &World, origin: Vec3, direction: Vec3, max_dist: f32) -> Option<RayHit> {
    let dir = direction.normalize();

    let mut voxel = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );

    let step = IVec3::new(
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    );

    let t_delta = Vec3::new(
        if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::MAX },
        if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::MAX },
        if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::MAX },
    );

    let mut t_max = Vec3::new(
        if dir.x > 0.0 { ((voxel.x as f32 + 1.0) - origin.x) / dir.x }
        else if dir.x < 0.0 { (voxel.x as f32 - origin.x) / dir.x }
        else { f32::MAX },
        if dir.y > 0.0 { ((voxel.y as f32 + 1.0) - origin.y) / dir.y }
        else if dir.y < 0.0 { (voxel.y as f32 - origin.y) / dir.y }
        else { f32::MAX },
        if dir.z > 0.0 { ((voxel.z as f32 + 1.0) - origin.z) / dir.z }
        else if dir.z < 0.0 { (voxel.z as f32 - origin.z) / dir.z }
        else { f32::MAX },
    );

    let mut prev = voxel;

    for _ in 0..((max_dist * 2.0) as usize + 1) {
        if world.get_block(voxel.x, voxel.y, voxel.z) != BLOCK_AIR {
            return Some(RayHit { block: voxel, adjacent: prev });
        }

        prev = voxel;

        if t_max.x < t_max.y && t_max.x < t_max.z {
            if t_max.x > max_dist { break; }
            voxel.x += step.x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            if t_max.y > max_dist { break; }
            voxel.y += step.y;
            t_max.y += t_delta.y;
        } else {
            if t_max.z > max_dist { break; }
            voxel.z += step.z;
            t_max.z += t_delta.z;
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Collision helpers
// ---------------------------------------------------------------------------

fn aabb_collides(world: &World, min: Vec3, max: Vec3) -> bool {
    let bx_min = min.x.floor() as i32;
    let by_min = min.y.floor() as i32;
    let bz_min = min.z.floor() as i32;
    let bx_max = (max.x - 0.001).floor() as i32;
    let by_max = (max.y - 0.001).floor() as i32;
    let bz_max = (max.z - 0.001).floor() as i32;

    for by in by_min..=by_max {
        for bz in bz_min..=bz_max {
            for bx in bx_min..=bx_max {
                if world.get_block(bx, by, bz) != BLOCK_AIR {
                    return true;
                }
            }
        }
    }
    false
}

fn find_highest_solid(world: &World, min: Vec3, max: Vec3, new_y: f32) -> f32 {
    let bx_min = min.x.floor() as i32;
    let by_min = min.y.floor() as i32;
    let bz_min = min.z.floor() as i32;
    let bx_max = (max.x - 0.001).floor() as i32;
    let by_max = (max.y - 0.001).floor() as i32;
    let bz_max = (max.z - 0.001).floor() as i32;

    let mut highest_top = new_y;
    for by in by_min..=by_max {
        for bz in bz_min..=bz_max {
            for bx in bx_min..=bx_max {
                if world.get_block(bx, by, bz) != BLOCK_AIR {
                    highest_top = highest_top.max((by + 1) as f32);
                }
            }
        }
    }
    highest_top
}

fn find_lowest_solid(world: &World, min: Vec3, max: Vec3, head_y: f32) -> f32 {
    let bx_min = min.x.floor() as i32;
    let by_min = min.y.floor() as i32;
    let bz_min = min.z.floor() as i32;
    let bx_max = (max.x - 0.001).floor() as i32;
    let by_max = (max.y - 0.001).floor() as i32;
    let bz_max = (max.z - 0.001).floor() as i32;

    let mut lowest_bottom = head_y;
    for by in by_min..=by_max {
        for bz in bz_min..=bz_max {
            for bx in bx_min..=bx_max {
                if world.get_block(bx, by, bz) != BLOCK_AIR {
                    lowest_bottom = lowest_bottom.min(by as f32);
                }
            }
        }
    }
    lowest_bottom
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

    fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, EYE_HEIGHT, 0.0)
    }

    fn look_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }

    fn update(&mut self, rl: &RaylibHandle, world: &World, dt: f32) {
        let md = rl.get_mouse_delta();
        self.yaw -= md.x * MOUSE_SENSITIVITY;
        self.pitch -= md.y * MOUSE_SENSITIVITY;
        self.pitch = self.pitch.clamp(-1.5, 1.5);

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

        if self.on_ground && rl.is_key_down(KeyboardKey::KEY_SPACE) {
            self.velocity.y = JUMP_SPEED;
            self.on_ground = false;
        }

        self.velocity.y -= GRAVITY * dt;

        self.resolve_y(world, dt);
        self.resolve_xz(world, dt);
    }

    fn resolve_y(&mut self, world: &World, dt: f32) {
        let new_y = self.position.y + self.velocity.y * dt;
        let min = Vec3::new(self.position.x - PLAYER_HALF_WIDTH, new_y, self.position.z - PLAYER_HALF_WIDTH);
        let max = Vec3::new(self.position.x + PLAYER_HALF_WIDTH, new_y + PLAYER_HEIGHT, self.position.z + PLAYER_HALF_WIDTH);

        if aabb_collides(world, min, max) {
            if self.velocity.y <= 0.0 {
                self.position.y = find_highest_solid(world, min, max, new_y);
                self.on_ground = true;
            } else {
                self.position.y = find_lowest_solid(world, min, max, new_y + PLAYER_HEIGHT) - PLAYER_HEIGHT;
            }
            self.velocity.y = 0.0;
        } else {
            self.position.y = new_y;
            self.on_ground = false;
        }
    }

    fn resolve_xz(&mut self, world: &World, dt: f32) {
        let new_x = self.position.x + self.velocity.x * dt;
        let min = Vec3::new(new_x - PLAYER_HALF_WIDTH, self.position.y, self.position.z - PLAYER_HALF_WIDTH);
        let max = Vec3::new(new_x + PLAYER_HALF_WIDTH, self.position.y + PLAYER_HEIGHT, self.position.z + PLAYER_HALF_WIDTH);
        if !aabb_collides(world, min, max) {
            self.position.x = new_x;
        }

        let new_z = self.position.z + self.velocity.z * dt;
        let min = Vec3::new(self.position.x - PLAYER_HALF_WIDTH, self.position.y, new_z - PLAYER_HALF_WIDTH);
        let max = Vec3::new(self.position.x + PLAYER_HALF_WIDTH, self.position.y + PLAYER_HEIGHT, new_z + PLAYER_HALF_WIDTH);
        if !aabb_collides(world, min, max) {
            self.position.z = new_z;
        }
    }

    fn camera(&self) -> Camera3D {
        let eye = self.eye_position();
        let target = eye + self.look_direction();

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

    let mut world = World::generate();

    // Build all chunk meshes
    let num_chunks = world.chunks.len();
    let mut meshes: Vec<ChunkMesh> = Vec::with_capacity(num_chunks);
    for i in 0..num_chunks {
        // Split borrow: read world for neighbor lookups, read chunk for block data
        let chunk = &world.chunks[i];
        meshes.push(ChunkMesh::build(chunk, &world));
    }

    // Spawn player at world center, above terrain
    let spawn_x = WORLD_BLOCKS / 2;
    let spawn_z = WORLD_BLOCKS / 2;
    let spawn_y = terrain_height(spawn_x, spawn_z) + 2;
    let mut player = Player::new(Vec3::new(spawn_x as f32, spawn_y as f32, spawn_z as f32));

    let mut dirty: Vec<bool> = vec![false; num_chunks];

    // Block types the player can place (cycle with 1-4 keys)
    const PLACEABLE: [u8; 4] = [BLOCK_GRASS, BLOCK_DIRT, BLOCK_STONE, BLOCK_WOOD];
    const PLACE_NAMES: [&str; 4] = ["Grass", "Dirt", "Stone", "Wood"];
    let mut selected: usize = 0;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        player.update(&rl, &world, dt);

        // Block selection (keys 1-4)
        if rl.is_key_pressed(KeyboardKey::KEY_ONE)   { selected = 0; }
        if rl.is_key_pressed(KeyboardKey::KEY_TWO)   { selected = 1; }
        if rl.is_key_pressed(KeyboardKey::KEY_THREE) { selected = 2; }
        if rl.is_key_pressed(KeyboardKey::KEY_FOUR)  { selected = 3; }

        // Scroll wheel to cycle
        let scroll = rl.get_mouse_wheel_move();
        if scroll > 0.0 { selected = (selected + 1) % PLACEABLE.len(); }
        if scroll < 0.0 { selected = (selected + PLACEABLE.len() - 1) % PLACEABLE.len(); }

        // Raycast
        let eye = player.eye_position();
        let dir = player.look_direction();
        let hit = raycast_voxel(&world, eye, dir, REACH_DISTANCE);

        // Block break
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some(ref h) = hit {
                let b = h.block;
                if World::in_bounds(b.x, b.y, b.z) {
                    let affected = world.dirty_chunks_for_block(b.x, b.y, b.z);
                    world.set_block(b.x, b.y, b.z, BLOCK_AIR);
                    for idx in affected { dirty[idx] = true; }
                }
            }
        }

        // Block place
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
            if let Some(ref h) = hit {
                let a = h.adjacent;
                if World::in_bounds(a.x, a.y, a.z)
                    && world.get_block(a.x, a.y, a.z) == BLOCK_AIR
                {
                    let pmin = player.position - Vec3::new(PLAYER_HALF_WIDTH, 0.0, PLAYER_HALF_WIDTH);
                    let pmax = player.position + Vec3::new(PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HALF_WIDTH);
                    let bmin = Vec3::new(a.x as f32, a.y as f32, a.z as f32);
                    let bmax = bmin + Vec3::ONE;

                    let overlaps = pmin.x < bmax.x && pmax.x > bmin.x
                        && pmin.y < bmax.y && pmax.y > bmin.y
                        && pmin.z < bmax.z && pmax.z > bmin.z;

                    if !overlaps {
                        let affected = world.dirty_chunks_for_block(a.x, a.y, a.z);
                        world.set_block(a.x, a.y, a.z, PLACEABLE[selected]);
                        for idx in affected { dirty[idx] = true; }
                    }
                }
            }
        }

        // Rebuild dirty meshes
        for i in 0..num_chunks {
            if dirty[i] {
                meshes[i].unload();
                meshes[i] = ChunkMesh::build(&world.chunks[i], &world);
                dirty[i] = false;
            }
        }

        let camera = player.camera();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(135, 206, 235, 255));

        {
            let mut d3 = d.begin_mode3D(camera);

            for m in &meshes {
                m.draw();
            }

            if let Some(ref h) = hit {
                let b = h.block;
                let center = Vector3::new(b.x as f32 + 0.5, b.y as f32 + 0.5, b.z as f32 + 0.5);
                d3.draw_cube_wires(center, 1.01, 1.01, 1.01, Color::WHITE);
            }
        }

        // Crosshair
        let cx = 1280 / 2;
        let cy = 720 / 2;
        d.draw_line(cx - 10, cy, cx + 10, cy, Color::WHITE);
        d.draw_line(cx, cy - 10, cx, cy + 10, Color::WHITE);

        d.draw_fps(10, 10);
        let block_name = PLACE_NAMES[selected];
        let hud = format!("[{}] {} | LMB: break | RMB: place | 1-4/Scroll: select | WASD SPACE", selected + 1, block_name);
        d.draw_text(&hud, 10, 30, 18, Color::WHITE);
    }
}
