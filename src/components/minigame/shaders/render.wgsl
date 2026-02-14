// Mario Minigame - Instanced Render Shader
// Efficient instanced rendering - vertex shader handles positioning, fragment just samples sprites

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
    // Dynamic grid dimensions (must match compute shader)
    grid_width: u32,
    grid_height: u32,
    grid_size: u32,
    egrid_width: u32,
    egrid_height: u32,
    egrid_cells: u32,
    egrid_size: u32,
    block_count: u32,
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

// Bindings - read-only for rendering
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> entities: array<Entity, 256>;
@group(0) @binding(2) var<storage, read> blocks: array<Block>;  // Runtime sized
@group(0) @binding(3) var<storage, read> sprites: array<u32, 256>;
@group(0) @binding(4) var<storage, read> palettes: array<u32, 48>;

//=============================================================================
// CONSTANTS
//=============================================================================

const TILE: f32 = 8.0;

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
const FLAG_BIG: u32 = 8u;
const FLAG_PLAYER: u32 = 16u;
const FLAG_DYING: u32 = 32u;

//=============================================================================
// SHARED SPRITE FUNCTIONS
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

//=============================================================================
// BACKGROUND PASS - Fullscreen triangle
//=============================================================================

struct BgVertexOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_background(@builtin(vertex_index) vertex_idx: u32) -> BgVertexOut {
    var out: BgVertexOut;
    // Fullscreen triangle (covers entire screen with 3 vertices)
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    out.pos = vec4<f32>(positions[vertex_idx], 0.0, 1.0);
    return out;
}

@fragment
fn fs_background(in: BgVertexOut) -> @location(0) vec4<f32> {
    // Black background
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

//=============================================================================
// BLOCK PASS - Instanced quad rendering
//=============================================================================

struct BlockVertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) sprite_id: u32,
    @location(2) @interpolate(flat) palette_id: u32,
};

@vertex
fn vs_block(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> BlockVertexOut {
    var out: BlockVertexOut;

    let b = blocks[instance_idx];

    // Check if block should be skipped (destroyed or offscreen)
    let is_destroyed = (b.flags & 2u) != 0u;
    let is_offscreen = b.pos.x < -16.0 || b.pos.y < -16.0 || b.pos.x > u.resolution.x + 16.0 || b.pos.y > u.resolution.y + 16.0;

    if (is_destroyed || is_offscreen) {
        // Degenerate triangle - will be culled
        out.pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.uv = vec2<f32>(0.0);
        out.sprite_id = 0u;
        out.palette_id = 0u;
        return out;
    }

    // Quad vertices (two triangles: 0-1-2, 2-1-3)
    // vertex_idx: 0=TL, 1=TR, 2=BL, 3=BL, 4=TR, 5=BR
    var quad_offsets = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),  // TL
        vec2<f32>(1.0, 0.0),  // TR
        vec2<f32>(0.0, 1.0),  // BL
        vec2<f32>(0.0, 1.0),  // BL
        vec2<f32>(1.0, 0.0),  // TR
        vec2<f32>(1.0, 1.0)   // BR
    );

    let offset = quad_offsets[vertex_idx];
    let pixel_pos = b.pos + offset * TILE;

    // Convert pixel position to NDC
    let ndc = (pixel_pos / u.resolution) * 2.0 - 1.0;
    out.pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);  // Flip Y for screen coords

    // UV for sprite sampling (0-1 range within 8x8 tile)
    out.uv = offset;

    // Select sprite and palette based on block kind
    if (b.kind == 3u) {
        // Ground block
        out.sprite_id = SPR_GROUND;
        out.palette_id = PAL_GROUND;
    } else if (b.kind == 2u) {
        // Empty question block
        out.sprite_id = SPR_QUESTION_EMPTY;
        out.palette_id = PAL_BRICK;
    } else if (b.kind == 1u) {
        // Question block
        out.sprite_id = SPR_QUESTION;
        out.palette_id = PAL_QUESTION;
    } else {
        // Brick
        out.sprite_id = SPR_BRICK;
        out.palette_id = PAL_BRICK;
    }

    return out;
}

@fragment
fn fs_block(in: BlockVertexOut) -> @location(0) vec4<f32> {
    // Convert UV (0-1) to pixel coords (0-7)
    let px = u32(in.uv.x * 7.999);
    let py = u32(in.uv.y * 7.999);

    let color_idx = get_sprite_pixel(in.sprite_id, px, py, false);
    let color = get_color(in.palette_id, color_idx);

    // Discard transparent pixels
    if (color.a < 0.5) {
        discard;
    }

    return color;
}

//=============================================================================
// ENTITY PASS - Instanced quad rendering
//=============================================================================

struct EntityVertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) sprite_id: u32,
    @location(2) @interpolate(flat) palette_id: u32,
    @location(3) @interpolate(flat) flip: u32,
};

@vertex
fn vs_entity(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> EntityVertexOut {
    var out: EntityVertexOut;

    let e = entities[instance_idx];

    // Check if entity should be skipped (dead and not dying)
    let is_visible = (e.flags & (FLAG_ALIVE | FLAG_DYING)) != 0u;

    if (!is_visible) {
        // Degenerate triangle - will be culled
        out.pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.uv = vec2<f32>(0.0);
        out.sprite_id = 0u;
        out.palette_id = 0u;
        out.flip = 0u;
        return out;
    }

    // Quad vertices (two triangles: 0-1-2, 2-1-3)
    var quad_offsets = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),  // TL
        vec2<f32>(1.0, 0.0),  // TR
        vec2<f32>(0.0, 1.0),  // BL
        vec2<f32>(0.0, 1.0),  // BL
        vec2<f32>(1.0, 0.0),  // TR
        vec2<f32>(1.0, 1.0)   // BR
    );

    let offset = quad_offsets[vertex_idx];
    let pixel_pos = e.pos + offset * TILE;

    // Convert pixel position to NDC
    let ndc = (pixel_pos / u.resolution) * 2.0 - 1.0;
    out.pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);  // Flip Y for screen coords

    // UV for sprite sampling
    out.uv = offset;
    out.flip = select(0u, 1u, (e.flags & FLAG_FLIP) != 0u);

    // Select sprite and palette based on entity kind and state
    let is_player = (e.flags & FLAG_PLAYER) != 0u;
    let on_ground = (e.flags & FLAG_GROUND) != 0u;

    switch (e.kind) {
        case KIND_MARIO: {
            if (!on_ground) {
                out.sprite_id = SPR_MARIO_JUMP;
            } else if (abs(e.vel.x) > 0.3) {
                out.sprite_id = select(SPR_MARIO_WALK1, SPR_MARIO_WALK2, (e.timer / 8u) % 2u == 1u);
            } else {
                out.sprite_id = SPR_MARIO_STAND;
            }
            out.palette_id = select(select(PAL_MARIO, PAL_LUIGI, e.state == 1u), PAL_PLAYER, is_player);
        }
        case KIND_GOOMBA: {
            out.sprite_id = SPR_GOOMBA;
            out.palette_id = PAL_GOOMBA;
        }
        case KIND_KOOPA: {
            if (e.state >= KOOPA_SHELL) {
                out.sprite_id = SPR_KOOPA_SHELL;
            } else {
                out.sprite_id = SPR_KOOPA;
            }
            out.palette_id = PAL_KOOPA;
        }
        case KIND_COIN: {
            out.sprite_id = SPR_COIN;
            out.palette_id = PAL_COIN;
        }
        case KIND_MUSHROOM: {
            out.sprite_id = SPR_MUSHROOM;
            out.palette_id = PAL_MARIO;
        }
        case KIND_DEBRIS: {
            out.sprite_id = SPR_DEBRIS;
            out.palette_id = PAL_BRICK;
        }
        default: {
            out.sprite_id = SPR_MARIO_STAND;
            out.palette_id = PAL_MARIO;
        }
    }

    return out;
}

@fragment
fn fs_entity(in: EntityVertexOut) -> @location(0) vec4<f32> {
    // Convert UV (0-1) to pixel coords (0-7)
    let px = u32(in.uv.x * 7.999);
    let py = u32(in.uv.y * 7.999);

    let color_idx = get_sprite_pixel(in.sprite_id, px, py, in.flip != 0u);
    let color = get_color(in.palette_id, color_idx);

    // Discard transparent pixels
    if (color.a < 0.5) {
        discard;
    }

    return color;
}
