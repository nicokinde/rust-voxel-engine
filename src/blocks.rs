pub const BLOCK_AIR: u8 = 0;
pub const BLOCK_GRASS: u8 = 1;
pub const BLOCK_DIRT: u8 = 2;
pub const BLOCK_STONE: u8 = 3;
pub const BLOCK_WOOD: u8 = 4;
pub const BLOCK_LEAVES: u8 = 5;
pub const BLOCK_COUNT: u8 = 6;

/// Per-block per-face base colors (R, G, B).
/// Order: top(+Y), bottom(-Y), right(+X), left(-X), front(+Z), back(-Z)
pub const BLOCK_FACE_BASES: [[[u8; 3]; 6]; BLOCK_COUNT as usize] = [
    // Air (unused)
    [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0]],
    // Grass: green top, dirt bottom, green-brown sides
    [[86,168,40],[134,96,67],[96,130,56],[96,130,56],[96,130,56],[96,130,56]],
    // Dirt
    [[134,96,67],[134,96,67],[134,96,67],[134,96,67],[134,96,67],[134,96,67]],
    // Stone
    [[136,136,136],[120,120,120],[128,128,128],[128,128,128],[128,128,128],[128,128,128]],
    // Wood: bark sides, lighter ring top/bottom
    [[187,157,100],[187,157,100],[110,78,42],[110,78,42],[110,78,42],[110,78,42]],
    // Leaves
    [[58,120,30],[40,85,20],[48,100,25],[48,100,25],[48,100,25],[48,100,25]],
];

pub const PLACEABLE: [u8; 5] = [BLOCK_GRASS, BLOCK_DIRT, BLOCK_STONE, BLOCK_WOOD, BLOCK_LEAVES];
pub const PLACE_NAMES: [&str; 5] = ["Grass", "Dirt", "Stone", "Wood", "Leaves"];
