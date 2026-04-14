use std::collections::HashMap;
use glam::IVec2;
use crate::blocks::*;
use crate::terrain::{self, COLUMN_HEIGHT};

pub const CHUNK_SIZE: i32 = 16;

pub struct ChunkColumn {
    pub blocks: Vec<u8>, // 16 * COLUMN_HEIGHT * 16
    pub position: IVec2, // (cx, cz)
}

impl ChunkColumn {
    pub fn new(cx: i32, cz: i32, blocks: Vec<u8>) -> Self {
        ChunkColumn {
            blocks,
            position: IVec2::new(cx, cz),
        }
    }

    #[inline]
    pub fn get_block(&self, lx: i32, y: i32, lz: i32) -> u8 {
        if lx < 0 || lx >= 16 || y < 0 || y >= COLUMN_HEIGHT || lz < 0 || lz >= 16 {
            return BLOCK_AIR;
        }
        self.blocks[terrain::column_index(lx, y, lz)]
    }

    #[inline]
    pub fn set_block(&mut self, lx: i32, y: i32, lz: i32, block: u8) {
        if lx >= 0 && lx < 16 && y >= 0 && y < COLUMN_HEIGHT && lz >= 0 && lz < 16 {
            self.blocks[terrain::column_index(lx, y, lz)] = block;
        }
    }
}

pub struct World {
    pub columns: HashMap<IVec2, ChunkColumn>,
}

impl World {
    pub fn new() -> Self {
        World {
            columns: HashMap::new(),
        }
    }

    pub fn insert_column(&mut self, col: ChunkColumn) {
        self.columns.insert(col.position, col);
    }

    pub fn get_block(&self, wx: i32, wy: i32, wz: i32) -> u8 {
        if wy < 0 || wy >= COLUMN_HEIGHT {
            return BLOCK_AIR;
        }
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE);
        let lz = wz.rem_euclid(CHUNK_SIZE);

        match self.columns.get(&IVec2::new(cx, cz)) {
            Some(col) => col.get_block(lx, wy, lz),
            None => BLOCK_AIR,
        }
    }

    pub fn set_block(&mut self, wx: i32, wy: i32, wz: i32, block: u8) {
        if wy < 0 || wy >= COLUMN_HEIGHT { return; }
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE);
        let lz = wz.rem_euclid(CHUNK_SIZE);

        if let Some(col) = self.columns.get_mut(&IVec2::new(cx, cz)) {
            col.set_block(lx, wy, lz, block);
        }
    }

    /// Returns chunk column keys that need remeshing when a block at (wx, wy, wz) changes.
    pub fn dirty_columns_for_block(&self, wx: i32, wz: i32) -> Vec<IVec2> {
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE);
        let lz = wz.rem_euclid(CHUNK_SIZE);

        let mut result = vec![IVec2::new(cx, cz)];

        if lx == 0               { result.push(IVec2::new(cx - 1, cz)); }
        if lx == CHUNK_SIZE - 1  { result.push(IVec2::new(cx + 1, cz)); }
        if lz == 0               { result.push(IVec2::new(cx, cz - 1)); }
        if lz == CHUNK_SIZE - 1  { result.push(IVec2::new(cx, cz + 1)); }

        result
    }

    pub fn has_column(&self, key: &IVec2) -> bool {
        self.columns.contains_key(key)
    }
}
