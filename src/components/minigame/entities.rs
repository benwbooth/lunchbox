//! Entity definitions for the Mario mini-game
#![allow(dead_code)]

/// Position and velocity in world coordinates (pixels)
#[derive(Clone, Debug)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Mario's current animation state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarioState {
    Standing,
    Walking,
    Jumping,
}

/// A Mario character (player or AI controlled)
#[derive(Clone, Debug)]
pub struct Mario {
    pub pos: Vec2,
    pub vel: Vec2,
    pub state: MarioState,
    pub facing_right: bool,
    pub on_ground: bool,
    pub is_player: bool,
    pub id: u32,
    pub walk_frame: u8,
    pub walk_timer: u8,
    /// AI state
    pub ai_target: Option<(f64, f64)>,
    pub ai_jump_cooldown: u8,
    pub ai_direction_timer: u8,
}

impl Mario {
    pub fn new(x: f64, y: f64, id: u32) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::zero(),
            state: MarioState::Standing,
            facing_right: true,
            on_ground: false,
            is_player: false,
            id,
            walk_frame: 0,
            walk_timer: 0,
            ai_target: None,
            ai_jump_cooldown: 0,
            ai_direction_timer: 0,
        }
    }

    /// Get the hitbox for collision detection
    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        // Mario is 8 pixels wide/tall at base scale, but we render at 2x
        let size = 16.0;
        (self.pos.x, self.pos.y, size, size)
    }

    /// Get the feet hitbox for stomp detection
    pub fn feet_hitbox(&self) -> (f64, f64, f64, f64) {
        let size = 16.0;
        (self.pos.x + 2.0, self.pos.y + size - 4.0, size - 4.0, 4.0)
    }
}

/// A Goomba enemy
#[derive(Clone, Debug)]
pub struct Goomba {
    pub pos: Vec2,
    pub vel: Vec2,
    pub facing_right: bool,
    pub alive: bool,
    pub squish_timer: u8,
    pub walk_frame: u8,
    pub walk_timer: u8,
}

impl Goomba {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(-0.5, 0.0), // Start moving left
            facing_right: false,
            alive: true,
            squish_timer: 0,
            walk_frame: 0,
            walk_timer: 0,
        }
    }

    /// Get the hitbox for collision detection
    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        let size = 16.0;
        (self.pos.x, self.pos.y, size, size)
    }

    /// Get the top hitbox for stomp detection
    pub fn head_hitbox(&self) -> (f64, f64, f64, f64) {
        let size = 16.0;
        (self.pos.x + 2.0, self.pos.y, size - 4.0, 6.0)
    }
}

/// A platform that entities can stand on
#[derive(Clone, Debug)]
pub struct Platform {
    /// Position in tile coordinates
    pub x: i32,
    pub y: i32,
    /// Width in tiles
    pub width: i32,
    /// Whether this is ground (uses ground tiles) or floating (uses brick tiles)
    pub is_ground: bool,
}

impl Platform {
    pub fn new(x: i32, y: i32, width: i32, is_ground: bool) -> Self {
        Self { x, y, width, is_ground }
    }

    /// Get the hitbox in pixel coordinates
    pub fn hitbox(&self, tile_size: i32) -> (f64, f64, f64, f64) {
        (
            (self.x * tile_size) as f64,
            (self.y * tile_size) as f64,
            (self.width * tile_size) as f64,
            tile_size as f64,
        )
    }
}

/// The complete game world state
#[derive(Clone)]
pub struct GameWorld {
    pub platforms: Vec<Platform>,
    pub marios: Vec<Mario>,
    pub goombas: Vec<Goomba>,
    pub width: i32,
    pub height: i32,
    pub tile_size: i32,
    pub player_mario_id: Option<u32>,
    pub next_mario_id: u32,
    pub goomba_spawn_timer: u32,
}

impl GameWorld {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            platforms: Vec::new(),
            marios: Vec::new(),
            goombas: Vec::new(),
            width,
            height,
            tile_size: 16,
            player_mario_id: None,
            next_mario_id: 0,
            goomba_spawn_timer: 0,
        }
    }

    /// Get the player-controlled Mario, if any
    pub fn player_mario(&self) -> Option<&Mario> {
        self.player_mario_id.and_then(|id| {
            self.marios.iter().find(|m| m.id == id)
        })
    }

    /// Get mutable reference to player-controlled Mario
    pub fn player_mario_mut(&mut self) -> Option<&mut Mario> {
        let id = self.player_mario_id?;
        self.marios.iter_mut().find(|m| m.id == id)
    }

    /// Set which Mario is player-controlled
    pub fn set_player(&mut self, id: u32) {
        // Remove player status from previous
        if let Some(old_id) = self.player_mario_id {
            if let Some(mario) = self.marios.iter_mut().find(|m| m.id == old_id) {
                mario.is_player = false;
            }
        }
        // Set new player
        if let Some(mario) = self.marios.iter_mut().find(|m| m.id == id) {
            mario.is_player = true;
            self.player_mario_id = Some(id);
        }
    }

    /// Get next Mario ID and increment counter
    pub fn next_id(&mut self) -> u32 {
        let id = self.next_mario_id;
        self.next_mario_id += 1;
        id
    }
}
