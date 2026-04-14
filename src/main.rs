mod blocks;
mod chunk_manager;
mod mesher;
mod player;
mod raycast;
mod terrain;
mod world;

use std::collections::HashMap;
use glam::{IVec2, Vec3};
use raylib::prelude::*;

use blocks::*;
use chunk_manager::ChunkManager;
use mesher::ColumnMesh;
use player::{Player, PLAYER_HEIGHT, PLAYER_HALF_WIDTH};
use raycast::{raycast_voxel, REACH_DISTANCE};
use terrain::terrain_height;
use world::World;

const MAX_MESH_REBUILDS_PER_FRAME: usize = 3;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("Rust Voxel Engine")
        .build();

    rl.set_target_fps(60);
    rl.disable_cursor();

    let mut world = World::new();
    let mut chunk_mgr = ChunkManager::new();
    let mut meshes: HashMap<IVec2, ColumnMesh> = HashMap::new();

    // Spawn player at (0, 0) world origin, above terrain
    let spawn_x = 0;
    let spawn_z = 0;
    let spawn_y = terrain_height(spawn_x, spawn_z) + 2;
    let mut player = Player::new(Vec3::new(spawn_x as f32, spawn_y as f32, spawn_z as f32));

    let mut selected: usize = 0;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();

        // -- Chunk management --
        let player_cx = player.position.x.floor() as i32 >> 4;
        let player_cz = player.position.z.floor() as i32 >> 4;
        chunk_mgr.update(&mut world, player_cx, player_cz);

        // Remove meshes for unloaded columns
        meshes.retain(|k, mesh| {
            if world.has_column(k) {
                true
            } else {
                mesh.unload();
                false
            }
        });

        // Rebuild dirty meshes (limited per frame)
        let dirty_keys = chunk_mgr.take_dirty(MAX_MESH_REBUILDS_PER_FRAME, player_cx, player_cz);
        for key in dirty_keys {
            if world.has_column(&key) {
                if let Some(old) = meshes.get_mut(&key) {
                    old.unload();
                }
                meshes.insert(key, ColumnMesh::build(key, &world));
            }
        }

        // -- Player update --
        player.update(&rl, &world, dt);

        // Block selection
        if rl.is_key_pressed(KeyboardKey::KEY_ONE)   { selected = 0; }
        if rl.is_key_pressed(KeyboardKey::KEY_TWO)   { selected = 1; }
        if rl.is_key_pressed(KeyboardKey::KEY_THREE) { selected = 2; }
        if rl.is_key_pressed(KeyboardKey::KEY_FOUR)  { selected = 3; }
        if rl.is_key_pressed(KeyboardKey::KEY_FIVE)  { selected = 4; }

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
                let affected = world.dirty_columns_for_block(b.x, b.z);
                world.set_block(b.x, b.y, b.z, BLOCK_AIR);
                for key in affected { chunk_mgr.dirty.insert(key); }
            }
        }

        // Block place
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
            if let Some(ref h) = hit {
                let a = h.adjacent;
                if world.get_block(a.x, a.y, a.z) == BLOCK_AIR {
                    let pmin = player.position - Vec3::new(PLAYER_HALF_WIDTH, 0.0, PLAYER_HALF_WIDTH);
                    let pmax = player.position + Vec3::new(PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HALF_WIDTH);
                    let bmin = Vec3::new(a.x as f32, a.y as f32, a.z as f32);
                    let bmax = bmin + Vec3::ONE;

                    let overlaps = pmin.x < bmax.x && pmax.x > bmin.x
                        && pmin.y < bmax.y && pmax.y > bmin.y
                        && pmin.z < bmax.z && pmax.z > bmin.z;

                    if !overlaps {
                        let affected = world.dirty_columns_for_block(a.x, a.z);
                        world.set_block(a.x, a.y, a.z, PLACEABLE[selected]);
                        for key in affected { chunk_mgr.dirty.insert(key); }
                    }
                }
            }
        }

        // -- Rendering --
        let camera = player.camera();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(135, 206, 235, 255));

        {
            let mut d3 = d.begin_mode3D(camera);

            for m in meshes.values() {
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
        let chunks_loaded = world.columns.len();
        let hud = format!(
            "[{}] {} | Chunks: {} | LMB: break | RMB: place | 1-5/Scroll | WASD SPACE",
            selected + 1, block_name, chunks_loaded
        );
        d.draw_text(&hud, 10, 30, 18, Color::WHITE);
    }
}
