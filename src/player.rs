use glam::Vec3;
use raylib::prelude::*;
use crate::blocks::BLOCK_AIR;
use crate::world::World;

pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_HALF_WIDTH: f32 = 0.3;
const EYE_HEIGHT: f32 = 1.62;
const GRAVITY: f32 = 20.0;
const JUMP_SPEED: f32 = 8.0;
const MOVE_SPEED: f32 = 5.0;
const MOUSE_SENSITIVITY: f32 = 0.003;

pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Self {
            position: pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, EYE_HEIGHT, 0.0)
    }

    pub fn look_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn update(&mut self, rl: &RaylibHandle, world: &World, dt: f32) {
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

    pub fn camera(&self) -> Camera3D {
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
