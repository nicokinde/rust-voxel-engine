use crate::blocks::*;

pub const COLUMN_HEIGHT: i32 = 128;

/// Integer hash → float in [0, 1).
pub fn hash2d(x: i32, z: i32) -> f32 {
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

/// Fractal noise: 4 octaves.
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
    value / max_amp
}

pub fn terrain_height(wx: i32, wz: i32) -> i32 {
    let x = wx as f32;
    let z = wz as f32;
    let h = fbm(x * 0.02 + 0.3, z * 0.02 + 0.7) * 24.0 + 8.0;
    (h as i32).clamp(1, COLUMN_HEIGHT - 2)
}

pub fn has_tree(wx: i32, wz: i32) -> bool {
    if wx % 5 != 0 || wz % 5 != 0 { return false; }
    hash2d(wx.wrapping_mul(13), wz.wrapping_mul(7)) < 0.3
}

/// Generate a full column of blocks (16 x COLUMN_HEIGHT x 16).
/// Includes terrain + trees fully contained within this column.
pub fn generate_column_blocks(cx: i32, cz: i32) -> Vec<u8> {
    let chunk_size = 16;
    let total = (chunk_size * COLUMN_HEIGHT * chunk_size) as usize;
    let mut blocks = vec![BLOCK_AIR; total];

    let ox = cx * chunk_size;
    let oz = cz * chunk_size;

    // Terrain pass
    for lz in 0..chunk_size {
        for lx in 0..chunk_size {
            let wx = ox + lx;
            let wz = oz + lz;
            let height = terrain_height(wx, wz);

            for wy in 0..COLUMN_HEIGHT.min(height + 1) {
                let depth = height - wy;
                let block = if depth == 0 {
                    BLOCK_GRASS
                } else if depth <= 3 {
                    BLOCK_DIRT
                } else {
                    BLOCK_STONE
                };
                blocks[column_index(lx, wy, lz)] = block;
            }
        }
    }

    // Tree pass — only trees whose trunk is in this column.
    // Trees may poke leaves into neighbor columns, but we accept minor clipping
    // at column edges for simplicity.
    for lz in 0..chunk_size {
        for lx in 0..chunk_size {
            let wx = ox + lx;
            let wz = oz + lz;
            if !has_tree(wx, wz) { continue; }
            let ground = terrain_height(wx, wz);
            if ground < 14 || ground >= COLUMN_HEIGHT - 7 { continue; }

            let trunk_height = 4 + (hash2d(wx * 3, wz * 3) * 2.0) as i32;

            // Trunk
            for dy in 1..=trunk_height {
                let y = ground + dy;
                if y < COLUMN_HEIGHT {
                    let idx = column_index(lx, y, lz);
                    if blocks[idx] == BLOCK_AIR { blocks[idx] = BLOCK_WOOD; }
                }
            }

            // Leaves canopy
            let top = ground + trunk_height;
            for dy in -1..=1_i32 {
                let radius: i32 = if dy == 1 { 1 } else { 2 };
                for dx in -radius..=radius {
                    for ddz in -radius..=radius {
                        if dx.abs() == radius && ddz.abs() == radius { continue; }
                        let bx = lx + dx;
                        let by = top + dy;
                        let bz = lz + ddz;
                        if bx >= 0 && bx < chunk_size && bz >= 0 && bz < chunk_size
                            && by >= 0 && by < COLUMN_HEIGHT
                        {
                            let idx = column_index(bx, by, bz);
                            if blocks[idx] == BLOCK_AIR { blocks[idx] = BLOCK_LEAVES; }
                        }
                    }
                }
            }
            // Top cap
            let cap_y = top + 2;
            if cap_y < COLUMN_HEIGHT {
                let idx = column_index(lx, cap_y, lz);
                if blocks[idx] == BLOCK_AIR { blocks[idx] = BLOCK_LEAVES; }
            }
        }
    }

    blocks
}

#[inline]
pub fn column_index(x: i32, y: i32, z: i32) -> usize {
    (y * 16 * 16 + z * 16 + x) as usize
}
