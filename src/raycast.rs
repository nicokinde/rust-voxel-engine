use glam::{IVec3, Vec3};
use crate::blocks::BLOCK_AIR;
use crate::world::World;

pub const REACH_DISTANCE: f32 = 6.0;

pub struct RayHit {
    pub block: IVec3,
    pub adjacent: IVec3,
}

pub fn raycast_voxel(world: &World, origin: Vec3, direction: Vec3, max_dist: f32) -> Option<RayHit> {
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
