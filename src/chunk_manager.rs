use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use glam::IVec2;
use crate::terrain;
use crate::world::{ChunkColumn, World};

pub const LOAD_RADIUS: i32 = 8;
pub const UNLOAD_RADIUS: i32 = 10;
const MAX_INSERTS_PER_FRAME: usize = 3;

pub struct ChunkManager {
    tx_request: Sender<IVec2>,
    rx_result: Receiver<ChunkColumn>,
    pending: HashSet<IVec2>,
    pub dirty: HashSet<IVec2>,
}

impl ChunkManager {
    pub fn new() -> Self {
        let (tx_request, rx_work) = mpsc::channel::<IVec2>();
        let (tx_result, rx_result) = mpsc::channel::<ChunkColumn>();

        let worker_count = thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(2))
            .unwrap_or(2);

        // Shared receiver via Arc<Mutex>
        let rx_work = std::sync::Arc::new(std::sync::Mutex::new(rx_work));

        for _ in 0..worker_count {
            let rx = rx_work.clone();
            let tx = tx_result.clone();
            thread::spawn(move || {
                loop {
                    let key = {
                        let lock = rx.lock().unwrap();
                        match lock.recv() {
                            Ok(k) => k,
                            Err(_) => return, // channel closed
                        }
                    };

                    let blocks = terrain::generate_column_blocks(key.x, key.y);
                    let col = ChunkColumn::new(key.x, key.y, blocks);
                    if tx.send(col).is_err() {
                        return;
                    }
                }
            });
        }

        ChunkManager {
            tx_request,
            rx_result,
            pending: HashSet::new(),
            dirty: HashSet::new(),
        }
    }

    /// Queue columns around the player, receive finished ones, unload far ones.
    pub fn update(&mut self, world: &mut World, player_cx: i32, player_cz: i32) {
        // 1. Queue missing columns in a spiral from center out
        let mut to_request: Vec<(IVec2, i32)> = Vec::new();
        for dx in -LOAD_RADIUS..=LOAD_RADIUS {
            for dz in -LOAD_RADIUS..=LOAD_RADIUS {
                let dist_sq = dx * dx + dz * dz;
                if dist_sq > LOAD_RADIUS * LOAD_RADIUS { continue; }
                let key = IVec2::new(player_cx + dx, player_cz + dz);
                if !world.has_column(&key) && !self.pending.contains(&key) {
                    to_request.push((key, dist_sq));
                }
            }
        }
        // Sort by distance — nearest first
        to_request.sort_by_key(|&(_, d)| d);
        for (key, _) in to_request {
            let _ = self.tx_request.send(key);
            self.pending.insert(key);
        }

        // 2. Receive finished columns (non-blocking, limited per frame)
        let mut inserted = 0;
        while inserted < MAX_INSERTS_PER_FRAME {
            match self.rx_result.try_recv() {
                Ok(col) => {
                    let key = col.position;
                    self.pending.remove(&key);
                    self.dirty.insert(key);

                    // Mark cardinal neighbors dirty (their edge faces may change)
                    for &[dx, dz] in &[[1, 0], [-1, 0], [0, 1], [0, -1]] {
                        let nk = IVec2::new(key.x + dx, key.y + dz);
                        if world.has_column(&nk) {
                            self.dirty.insert(nk);
                        }
                    }

                    world.insert_column(col);
                    inserted += 1;
                }
                Err(_) => break,
            }
        }

        // 3. Unload far columns
        let center = IVec2::new(player_cx, player_cz);
        let to_remove: Vec<IVec2> = world
            .columns
            .keys()
            .filter(|k| {
                let dx = k.x - center.x;
                let dz = k.y - center.y;
                dx * dx + dz * dz > UNLOAD_RADIUS * UNLOAD_RADIUS
            })
            .copied()
            .collect();
        for key in to_remove {
            world.columns.remove(&key);
            self.dirty.remove(&key);
        }
    }

    /// Take up to `max` dirty keys, prioritized by distance to player.
    pub fn take_dirty(&mut self, max: usize, player_cx: i32, player_cz: i32) -> Vec<IVec2> {
        let mut keys: Vec<IVec2> = self.dirty.iter().copied().collect();
        keys.sort_by_key(|k| {
            let dx = k.x - player_cx;
            let dz = k.y - player_cz;
            dx * dx + dz * dz
        });
        keys.truncate(max);
        for k in &keys {
            self.dirty.remove(k);
        }
        keys
    }
}
