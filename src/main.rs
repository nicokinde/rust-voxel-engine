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
const BLOCK_LEAVES: u8 = 5;
const BLOCK_COUNT: u8 = 6;

/// Per-block per-face base colors (R, G, B).
/// Order: top(+Y), bottom(-Y), right(+X), left(-X), front(+Z), back(-Z)
const BLOCK_FACE_BASES: [[[u8; 3]; 6]; BLOCK_COUNT as usize] = [
    // Air (unused)
    [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0]],
    // Grass: green top, dirt bottom, green-brown sides
    [[86,168,40],[134,96,67],[96,130,56],[96,130,56],[96,130,56],[96,130,56]],
    // Dirt: brown all faces, slight variation
    [[134,96,67],[134,96,67],[134,96,67],[134,96,67],[134,96,67],[134,96,67]],
    // Stone: grey with slight blue tint
    [[136,136,136],[120,120,120],[128,128,128],[128,128,128],[128,128,128],[128,128,128]],
    // Wood: brown bark sides, lighter ring top/bottom
    [[187,157,100],[187,157,100],[110,78,42],[110,78,42],[110,78,42],[110,78,42]],
    // Leaves: dark green, slightly brighter top
    [[58,120,30],[40,85,20],[48,100,25],[48,100,25],[48,100,25],[48,100,25]],
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

/// Simple hash for per-vertex color noise (fake texture).
fn hash_noise(x: i32, y: i32, z: i32, face: i32) -> f32 {
    let n = x.wrapping_mul(374761393)
        ^ y.wrapping_mul(668265263)
        ^ z.wrapping_mul(1274126177)
        ^ face.wrapping_mul(1911520717);
    let n = ((n >> 13) ^ n).wrapping_mul(n.wrapping_mul(n).wrapping_mul(60493).wrapping_add(19990303));
    let n = (n >> 13) ^ n;
    // Map to 0.85..1.0 range — subtle variation
    0.85 + (n as u32 as f32 / u32::MAX as f32) * 0.15
}

fn block_face_color_noisy(block_id: u8, face: usize, wx: i32, wy: i32, wz: i32) -> [u8; 4] {
    let base = BLOCK_FACE_BASES[block_id as usize][face];
    let b = FACE_BRIGHTNESS[face];
    let noise = hash_noise(wx, wy, wz, face as i32);
    [
        ((base[0] as f32 * b * noise).min(255.0)) as u8,
        ((base[1] as f32 * b * noise).min(255.0)) as u8,
        ((base[2] as f32 * b * noise).min(255.0)) as u8,
        255,
    ]
}

// ---------------------------------------------------------------------------
// Terrain generation — value noise with octaves
// ---------------------------------------------------------------------------

/// Integer hash → float in [0, 1).
fn hash2d(x: i32, z: i32) -> f32 {
    let n = x.wrapping_mul(374761393).wrapping_add(z.wrapping_mul(668265263));
    let n = ((n >> 13) ^ n).wrapping_mul(n.wrapping_mul(n).wrapping_mul(60493).wrapping_add(19990303));
    (((n >> 13) ^ n) as u32) as f32 / u32::MAX as f32
}

/// Smooth noise with bilinear interpolation.
fn smooth_noise(x: f32, z: f32) -> f32 {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - ix as f32;
    let fz = z - iz as f32;
    // Smoothstep
    let fx = fx * fx * (3.0 - 2.0 * fx);
    let fz = fz * fz * (3.0 - 2.0 * fz);

    let v00 = hash2d(ix, iz);
    let v10 = hash2d(ix + 1, iz);
    let v01 = hash2d(ix, iz + 1);
    let v11 = hash2d(ix + 1, iz + 1);

    let a = v00 + (v10 - v00) * fx;
    let b = v01 + (v11 - v01) * fx;
    a + (b - a) * fz
}

/// Fractal noise: 4 octaves for natural-looking terrain.
fn fbm(x: f32, z: f32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amp = 0.0;
    for _ in 0..4 {
        value += smooth_noise(x * frequency, z * frequency) * amplitude;
        max_amp += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value / max_amp // normalized to [0, 1]
}

const SEA_LEVEL: i32 = 14;

fn terrain_height(wx: i32, wz: i32) -> i32 {
    let x = wx as f32;
    let z = wz as f32;
    // Large-scale hills + detail
    let h = fbm(x * 0.02 + 0.3, z * 0.02 + 0.7) * 24.0 + 8.0;
    (h as i32).clamp(1, WORLD_BLOCKS - 2)
}

/// Determine if a tree should spawn at (wx, wz). Simple hash-based placement.
fn has_tree(wx: i32, wz: i32) -> bool {
    // Only try every 4th column in a grid, then hash to thin out
    if wx % 5 != 0 || wz % 5 != 0 { return false; }
    let h = hash2d(wx.wrapping_mul(13), wz.wrapping_mul(7));
    h < 0.3 // ~30% chance on valid grid points
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
                        BLOCK_GRASS
                    } else if depth <= 3 {
                        BLOCK_DIRT
                    } else {
                        BLOCK_STONE
                    };
                }
            }
        }

        Chunk { blocks, position }
    }

    fn set_block_if_air(&mut self, x: usize, y: usize, z: usize, block: u8) {
        if x < CHUNK_USIZE && y < CHUNK_USIZE && z < CHUNK_USIZE {
            let idx = Self::index(x, y, z);
            if self.blocks[idx] == BLOCK_AIR {
                self.blocks[idx] = block;
            }
        }
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
        let mut world = World { chunks };
        world.place_trees();
        world
    }

    fn place_trees(&mut self) {
        for wx in 0..WORLD_BLOCKS {
            for wz in 0..WORLD_BLOCKS {
                if !has_tree(wx, wz) { continue; }
                let ground = terrain_height(wx, wz);
                if ground < SEA_LEVEL || ground >= WORLD_BLOCKS - 7 { continue; }

                let trunk_height = 4 + ((hash2d(wx * 3, wz * 3) * 2.0) as i32); // 4-5

                // Trunk
                for dy in 1..=trunk_height {
                    self.set_block_if_air(wx, ground + dy, wz, BLOCK_WOOD);
                }

                // Leaf canopy — 3 layers
                let top = ground + trunk_height;
                for dy in -1..=1_i32 {
                    let radius: i32 = if dy == 1 { 1 } else { 2 };
                    for dx in -radius..=radius {
                        for dz in -radius..=radius {
                            // Skip corners for rounder shape
                            if dx.abs() == radius && dz.abs() == radius { continue; }
                            let lx = wx + dx;
                            let ly = top + dy;
                            let lz = wz + dz;
                            self.set_block_if_air(lx, ly, lz, BLOCK_LEAVES);
                        }
                    }
                }
                // Top cap
                self.set_block_if_air(wx, top + 2, wz, BLOCK_LEAVES);
            }
        }
    }

    fn set_block_if_air(&mut self, wx: i32, wy: i32, wz: i32, block: u8) {
        if !Self::in_bounds(wx, wy, wz) { return; }
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cy = wy.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE) as usize;
        let ly = wy.rem_euclid(CHUNK_SIZE) as usize;
        let lz = wz.rem_euclid(CHUNK_SIZE) as usize;
        if let Some(idx) = Self::chunk_index(cx, cy, cz) {
            self.chunks[idx].set_block_if_air(lx, ly, lz, block);
        }
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
                        let col = block_face_color_noisy(block_id, face, wx, wy, wz);

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
        let right = Vec3::new(-self.yaw.cos(), 0.0, self.yaw.sin());

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
    const PLACEABLE: [u8; 5] = [BLOCK_GRASS, BLOCK_DIRT, BLOCK_STONE, BLOCK_WOOD, BLOCK_LEAVES];
    const PLACE_NAMES: [&str; 5] = ["Grass", "Dirt", "Stone", "Wood", "Leaves"];
    let mut selected: usize = 0;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        player.update(&rl, &world, dt);

        // Block selection (keys 1-4)
        if rl.is_key_pressed(KeyboardKey::KEY_ONE)   { selected = 0; }
        if rl.is_key_pressed(KeyboardKey::KEY_TWO)   { selected = 1; }
        if rl.is_key_pressed(KeyboardKey::KEY_THREE) { selected = 2; }
        if rl.is_key_pressed(KeyboardKey::KEY_FOUR)  { selected = 3; }
        if rl.is_key_pressed(KeyboardKey::KEY_FIVE)  { selected = 4; }

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
        let hud = format!("[{}] {} | LMB: break | RMB: place | 1-5/Scroll: select | WASD SPACE", selected + 1, block_name);
        d.draw_text(&hud, 10, 30, 18, Color::WHITE);
    }
}
