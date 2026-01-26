// Mario Minigame - Instanced Render Shader
// Efficient sprite rendering using instanced quads

//=============================================================================
// DATA STRUCTURES
//=============================================================================

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    delta_time: f32,
    mouse: vec2<f32>,
    mouse_click: u32,
    frame: u32,
};

struct Entity {
    pos: vec2<f32>,
    vel: vec2<f32>,
    kind: u32,
    state: u32,
    flags: u32,
    timer: u32,
};

struct Block {
    pos: vec2<f32>,
    kind: u32,
    flags: u32,
};

struct Platform {
    x: f32,
    y: f32,
    width: f32,
    is_ground: u32,
};

// Bindings - read-only for rendering
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> entities: array<Entity, 128>;
@group(0) @binding(2) var<storage, read> blocks: array<Block, 512>;
@group(0) @binding(3) var<storage, read> platforms: array<Platform, 512>;
@group(0) @binding(4) var<storage, read> sprites: array<u32, 256>;
@group(0) @binding(5) var<storage, read> palettes: array<u32, 48>;

//=============================================================================
// CONSTANTS
//=============================================================================

const TILE: f32 = 8.0;
const ENTITY_COUNT: u32 = 128u;
const BLOCK_COUNT: u32 = 512u;
const PLATFORM_COUNT: u32 = 512u;  // Many platforms

// Total instances = platforms + blocks + entities
const PLATFORM_OFFSET: u32 = 0u;
const BLOCK_OFFSET: u32 = 512u;        // After platforms (512)
const ENTITY_OFFSET: u32 = 1024u;      // After blocks (512 + 512)
const TOTAL_INSTANCES: u32 = 1152u;    // 512 + 512 + 128

// Entity kinds
const KIND_MARIO: u32 = 0u;
const KIND_GOOMBA: u32 = 1u;
const KIND_KOOPA: u32 = 2u;
const KIND_COIN: u32 = 3u;

// Sprite indices
const SPR_MARIO_STAND: u32 = 0u;
const SPR_MARIO_WALK1: u32 = 1u;
const SPR_MARIO_WALK2: u32 = 2u;
const SPR_MARIO_JUMP: u32 = 3u;
const SPR_GOOMBA: u32 = 4u;
const SPR_BRICK: u32 = 5u;
const SPR_QUESTION: u32 = 6u;
const SPR_GROUND: u32 = 7u;
const SPR_KOOPA: u32 = 8u;
const SPR_COIN: u32 = 9u;

// Palette indices
const PAL_MARIO: u32 = 0u;
const PAL_LUIGI: u32 = 1u;
const PAL_GOOMBA: u32 = 2u;
const PAL_BRICK: u32 = 3u;
const PAL_QUESTION: u32 = 4u;
const PAL_GROUND: u32 = 5u;
const PAL_KOOPA: u32 = 6u;
const PAL_COIN: u32 = 7u;
const PAL_PLAYER: u32 = 8u;

// Flags
const FLAG_FLIP: u32 = 1u;
const FLAG_ALIVE: u32 = 2u;
const FLAG_GROUND: u32 = 4u;
const FLAG_PLAYER: u32 = 16u;

//=============================================================================
// VERTEX SHADER - Instanced quad rendering
//=============================================================================

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) sprite_id: u32,
    @location(2) @interpolate(flat) palette_id: u32,
    @location(3) @interpolate(flat) flags: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> VertexOut {
    var out: VertexOut;

    // Quad vertices (2 triangles, 6 vertices)
    // 0--1    Triangles: 0-1-2, 2-1-3
    // |\ |
    // | \|
    // 2--3
    var quad_pos: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );

    var quad_uv: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );

    let local_pos = quad_pos[vertex_idx];
    let local_uv = quad_uv[vertex_idx];

    // Determine what we're rendering based on instance index
    var world_pos: vec2<f32>;
    var size: vec2<f32> = vec2<f32>(TILE, TILE);
    var sprite_id: u32 = 0u;
    var palette_id: u32 = 0u;
    var flags: u32 = 0u;
    var visible: bool = true;

    if (instance_idx < BLOCK_OFFSET) {
        // Platform instance - render ALL platforms, not just ground
        let plat_idx = instance_idx;
        if (plat_idx < PLATFORM_COUNT) {
            let p = platforms[plat_idx];
            if (p.width > 0.0) {
                world_pos = vec2<f32>(p.x * TILE, p.y * TILE);
                size = vec2<f32>(p.width * TILE, TILE);
                // Use ground sprite for all platforms (or could use brick for floating)
                sprite_id = select(SPR_BRICK, SPR_GROUND, p.is_ground == 1u);
                palette_id = select(PAL_BRICK, PAL_GROUND, p.is_ground == 1u);
            } else {
                visible = false;
            }
        } else {
            visible = false;
        }
    } else if (instance_idx < ENTITY_OFFSET) {
        // Block instance
        let block_idx = instance_idx - BLOCK_OFFSET;
        if (block_idx < BLOCK_COUNT) {
            let b = blocks[block_idx];
            if ((b.flags & 2u) == 0u && b.pos.x > 0.0) {
                world_pos = b.pos;
                sprite_id = select(SPR_BRICK, SPR_QUESTION, b.kind == 1u);
                palette_id = select(PAL_BRICK, PAL_QUESTION, b.kind == 1u);
            } else {
                visible = false;
            }
        } else {
            visible = false;
        }
    } else {
        // Entity instance
        let ent_idx = instance_idx - ENTITY_OFFSET;
        if (ent_idx < ENTITY_COUNT) {
            let e = entities[ent_idx];
            if ((e.flags & FLAG_ALIVE) != 0u) {
                world_pos = e.pos;
                flags = e.flags;

                let is_player = (e.flags & FLAG_PLAYER) != 0u;
                let on_ground = (e.flags & FLAG_GROUND) != 0u;

                switch (e.kind) {
                    case KIND_MARIO: {
                        if (!on_ground) {
                            sprite_id = SPR_MARIO_JUMP;
                        } else if (abs(e.vel.x) > 0.3) {
                            sprite_id = select(SPR_MARIO_WALK1, SPR_MARIO_WALK2, (e.timer / 8u) % 2u == 1u);
                        } else {
                            sprite_id = SPR_MARIO_STAND;
                        }
                        palette_id = select(select(PAL_MARIO, PAL_LUIGI, e.state == 1u), PAL_PLAYER, is_player);
                    }
                    case KIND_GOOMBA: {
                        sprite_id = SPR_GOOMBA;
                        palette_id = PAL_GOOMBA;
                    }
                    case KIND_KOOPA: {
                        sprite_id = SPR_KOOPA;
                        palette_id = PAL_KOOPA;
                    }
                    case KIND_COIN: {
                        sprite_id = SPR_COIN;
                        palette_id = PAL_COIN;
                    }
                    default: {
                        visible = false;
                    }
                }
            } else {
                visible = false;
            }
        } else {
            visible = false;
        }
    }

    // Hide invisible instances by placing off-screen
    if (!visible) {
        out.pos = vec4<f32>(-10.0, -10.0, 0.0, 1.0);
        out.uv = vec2<f32>(0.0, 0.0);
        out.sprite_id = 0u;
        out.palette_id = 0u;
        out.flags = 0u;
        return out;
    }

    // Calculate screen position
    let pixel_pos = world_pos + local_pos * size;
    let ndc = (pixel_pos / u.resolution) * 2.0 - 1.0;

    out.pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = local_uv;
    out.sprite_id = sprite_id;
    out.palette_id = palette_id;
    out.flags = flags;

    return out;
}

//=============================================================================
// FRAGMENT SHADER - Sprite sampling
//=============================================================================

fn get_sprite_pixel(sprite_id: u32, x: u32, y: u32, flip: bool) -> u32 {
    let px = select(x, 7u - x, flip);
    let idx = y * 8u + px;
    let word_idx = sprite_id * 4u + idx / 16u;
    let bit_idx = (idx % 16u) * 2u;
    return (sprites[word_idx] >> bit_idx) & 3u;
}

fn get_color(palette_id: u32, color_idx: u32) -> vec4<f32> {
    if (color_idx == 0u) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0); // Transparent
    }
    let packed = palettes[palette_id * 4u + color_idx];
    return vec4<f32>(
        f32((packed >> 16u) & 0xFFu) / 255.0,
        f32((packed >> 8u) & 0xFFu) / 255.0,
        f32(packed & 0xFFu) / 255.0,
        1.0
    );
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Sample sprite at UV position (8x8 sprite)
    let sprite_x = u32(in.uv.x * 8.0) % 8u;
    let sprite_y = u32(in.uv.y * 8.0) % 8u;

    let flip = (in.flags & FLAG_FLIP) != 0u;
    let color_idx = get_sprite_pixel(in.sprite_id, sprite_x, sprite_y, flip);
    let color = get_color(in.palette_id, color_idx);

    // Discard transparent pixels
    if (color.a < 0.5) {
        discard;
    }

    return color;
}
