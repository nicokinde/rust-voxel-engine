use glam::IVec2;
use raylib::ffi;
use crate::blocks::*;
use crate::terrain::COLUMN_HEIGHT;
use crate::world::World;

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

const FACE_BRIGHTNESS: [f32; 6] = [1.0, 0.5, 0.75, 0.75, 0.6, 0.6];

fn hash_noise(x: i32, y: i32, z: i32, face: i32) -> f32 {
    let n = x.wrapping_mul(374761393)
        ^ y.wrapping_mul(668265263)
        ^ z.wrapping_mul(1274126177)
        ^ face.wrapping_mul(1911520717);
    let n = ((n >> 13) ^ n).wrapping_mul(n.wrapping_mul(n).wrapping_mul(60493).wrapping_add(19990303));
    let n = (n >> 13) ^ n;
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

pub struct ColumnMesh {
    mesh: ffi::Mesh,
    material: ffi::Material,
    has_data: bool,
}

impl ColumnMesh {
    pub fn build(key: IVec2, world: &World) -> Self {
        let col = match world.columns.get(&key) {
            Some(c) => c,
            None => return Self::empty(),
        };

        let ox = key.x * 16;
        let oz = key.y * 16;

        let mut positions: Vec<f32> = Vec::new();
        let mut normals: Vec<f32> = Vec::new();
        let mut colors: Vec<u8> = Vec::new();

        for wy in 0..COLUMN_HEIGHT {
            for lz in 0..16_i32 {
                for lx in 0..16_i32 {
                    let block_id = col.get_block(lx, wy, lz);
                    if block_id == BLOCK_AIR { continue; }

                    let wx = ox + lx;
                    let wz = oz + lz;

                    for face in 0..6 {
                        let n = &NEIGHBOR_OFFSETS[face];
                        let nwx = wx + n[0];
                        let nwy = wy + n[1];
                        let nwz = wz + n[2];

                        if world.get_block(nwx, nwy, nwz) != BLOCK_AIR {
                            continue;
                        }

                        let norm = &FACE_NORMALS[face];
                        let col = block_face_color_noisy(block_id, face, wx, wy, wz);
                        let bx = wx as f32;
                        let by = wy as f32;
                        let bz = wz as f32;

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
        ColumnMesh { mesh, material, has_data }
    }

    fn empty() -> Self {
        ColumnMesh {
            mesh: unsafe { std::mem::zeroed() },
            material: unsafe { ffi::LoadMaterialDefault() },
            has_data: false,
        }
    }

    pub fn draw(&self) {
        if !self.has_data { return; }
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

    pub fn unload(&mut self) {
        if self.has_data {
            unsafe { ffi::UnloadMesh(self.mesh); }
            self.has_data = false;
        }
    }
}
