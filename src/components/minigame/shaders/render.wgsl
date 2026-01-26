// Mario Minigame - Instanced Render Shader
// Efficient sprite rendering with proper brick tiling

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
const PLATFORM_COUNT: u32 = 512u;

const PLATFORM_OFFSET: u32 = 0u;
const BLOCK_OFFSET: u32 = 512u;
const ENTITY_OFFSET: u32 = 1024u;
const TOTAL_INSTANCES: u32 = 1152u;

// Entity kinds
const KIND_MARIO: u32 = 0u;
const KIND_GOOMBA: u32 = 1u;
const KIND_KOOPA: u32 = 2u;
const KIND_COIN: u32 = 3u;
const KIND_MUSHROOM: u32 = 4u;
const KIND_DEBRIS: u32 = 5u;

// Koopa states
const KOOPA_WALK: u32 = 0u;
const KOOPA_SHELL: u32 = 1u;
const KOOPA_SHELL_MOVING: u32 = 2u;

// Sprite indices (must match gpu.rs sprite_list order)
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
const SPR_MUSHROOM: u32 = 10u;
const SPR_MARIO_DEAD: u32 = 11u;
const SPR_QUESTION_EMPTY: u32 = 12u;
const SPR_KOOPA_SHELL: u32 = 13u;
const SPR_DEBRIS: u32 = 14u;

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
const FLAG_DYING: u32 = 32u;

//=============================================================================
// VERTEX SHADER
//=============================================================================

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) sprite_id: u32,
    @location(2) @interpolate(flat) palette_id: u32,
    @location(3) @interpolate(flat) flags: u32,
    @location(4) pixel_size: vec2<f32>,  // Size in pixels for tiling
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> VertexOut {
    var out: VertexOut;

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

    var world_pos: vec2<f32>;
    var size: vec2<f32> = vec2<f32>(TILE, TILE);
    var sprite_id: u32 = 0u;
    var palette_id: u32 = 0u;
    var flags: u32 = 0u;
    var visible: bool = true;

    if (instance_idx < BLOCK_OFFSET) {
        // Platform instance
        let plat_idx = instance_idx;
        if (plat_idx < PLATFORM_COUNT) {
            let p = platforms[plat_idx];
            if (p.width > 0.0) {
                world_pos = vec2<f32>(p.x * TILE, p.y * TILE);
                size = vec2<f32>(p.width * TILE, TILE);
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
            if ((b.flags & 2u) == 0u && (b.pos.x >= 0.0 || b.pos.y >= 0.0)) {
                world_pos = b.pos;
                if (b.kind == 2u) {
                    // Empty question block
                    sprite_id = SPR_QUESTION_EMPTY;
                    palette_id = PAL_BRICK; // Use brick palette for empty blocks
                } else if (b.kind == 1u) {
                    sprite_id = SPR_QUESTION;
                    palette_id = PAL_QUESTION;
                } else {
                    sprite_id = SPR_BRICK;
                    palette_id = PAL_BRICK;
                }
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
            // Show alive OR dying entities
            if ((e.flags & (FLAG_ALIVE | FLAG_DYING)) != 0u) {
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
                        // state 0 = Mario, state 1 = Luigi
                        palette_id = select(select(PAL_MARIO, PAL_LUIGI, e.state == 1u), PAL_PLAYER, is_player);
                    }
                    case KIND_GOOMBA: {
                        sprite_id = SPR_GOOMBA;
                        palette_id = PAL_GOOMBA;
                    }
                    case KIND_KOOPA: {
                        if (e.state >= KOOPA_SHELL) {
                            sprite_id = SPR_KOOPA_SHELL;
                        } else {
                            sprite_id = SPR_KOOPA;
                        }
                        palette_id = PAL_KOOPA;
                    }
                    case KIND_COIN: {
                        sprite_id = SPR_COIN;
                        palette_id = PAL_COIN;
                    }
                    case KIND_MUSHROOM: {
                        sprite_id = SPR_MUSHROOM;
                        palette_id = PAL_MARIO; // Red mushroom
                    }
                    case KIND_DEBRIS: {
                        sprite_id = SPR_DEBRIS;
                        palette_id = PAL_BRICK;
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

    if (!visible) {
        out.pos = vec4<f32>(-10.0, -10.0, 0.0, 1.0);
        out.uv = vec2<f32>(0.0, 0.0);
        out.sprite_id = 0u;
        out.palette_id = 0u;
        out.flags = 0u;
        out.pixel_size = vec2<f32>(8.0, 8.0);
        return out;
    }

    let pixel_pos = world_pos + local_pos * size;
    let ndc = (pixel_pos / u.resolution) * 2.0 - 1.0;

    out.pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = local_uv;
    out.sprite_id = sprite_id;
    out.palette_id = palette_id;
    out.flags = flags;
    out.pixel_size = size;  // Pass actual size for tiling

    return out;
}

//=============================================================================
// FRAGMENT SHADER - With proper tiling for wide platforms
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
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
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
    // Calculate actual pixel position within the quad
    let pixel_in_quad = in.uv * in.pixel_size;

    // Tile the 8x8 sprite by taking modulo 8
    let sprite_x = u32(pixel_in_quad.x) % 8u;
    let sprite_y = u32(pixel_in_quad.y) % 8u;

    let flip = (in.flags & FLAG_FLIP) != 0u;
    let color_idx = get_sprite_pixel(in.sprite_id, sprite_x, sprite_y, flip);
    let color = get_color(in.palette_id, color_idx);

    if (color.a < 0.5) {
        discard;
    }

    return color;
}
