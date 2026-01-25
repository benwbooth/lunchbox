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
    Dead,
}

/// Character type (visual variant)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CharacterType {
    Mario,
    Luigi,
    Toad,
    Princess,
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
    /// Is this Mario big (powered up)?
    pub is_big: bool,
    /// Is Mario alive?
    pub alive: bool,
    /// Invincibility frames after getting hit
    pub invincible_timer: u8,
    /// Death animation timer
    pub death_timer: u8,
    /// AI state
    pub ai_jump_cooldown: u8,
    pub ai_direction_timer: u8,
    /// Character type (Mario, Luigi, Toad, Princess)
    pub character_type: CharacterType,
}

impl Mario {
    pub fn new(x: f64, y: f64, id: u32) -> Self {
        // Randomly pick a character type
        let character_type = match (js_sys::Math::random() * 4.0) as u32 {
            0 => CharacterType::Mario,
            1 => CharacterType::Luigi,
            2 => CharacterType::Toad,
            _ => CharacterType::Princess,
        };

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
            is_big: false,
            alive: true,
            invincible_timer: 0,
            death_timer: 0,
            ai_jump_cooldown: 0,
            ai_direction_timer: 0,
            character_type,
        }
    }

    /// Get sprite size based on big/small state (8px sprites)
    pub fn sprite_height(&self) -> f64 {
        if self.is_big { 16.0 } else { 8.0 }
    }

    /// Get the hitbox for collision detection
    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        let height = self.sprite_height();
        (self.pos.x, self.pos.y, 8.0, height)
    }

    /// Get the feet hitbox for stomp detection
    pub fn feet_hitbox(&self) -> (f64, f64, f64, f64) {
        let height = self.sprite_height();
        (self.pos.x + 1.0, self.pos.y + height - 3.0, 6.0, 3.0)
    }

    /// Get the head hitbox for being stomped
    pub fn head_hitbox(&self) -> (f64, f64, f64, f64) {
        (self.pos.x + 1.0, self.pos.y, 6.0, 4.0)
    }

    /// Get body hitbox for side collisions (excludes head/feet)
    pub fn body_hitbox(&self) -> (f64, f64, f64, f64) {
        let height = self.sprite_height();
        (self.pos.x, self.pos.y + 4.0, 8.0, height - 7.0)
    }
}

/// A Goomba enemy
#[derive(Clone, Debug)]
pub struct Goomba {
    pub pos: Vec2,
    pub vel: Vec2,
    pub facing_right: bool,
    pub alive: bool,
    pub on_ground: bool,
    pub squish_timer: u8,
    pub walk_frame: u8,
    pub walk_timer: u8,
}

impl Goomba {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(-0.5, 0.0),
            facing_right: false,
            alive: true,
            on_ground: false,
            squish_timer: 0,
            walk_frame: 0,
            walk_timer: 0,
        }
    }

    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        (self.pos.x, self.pos.y, 8.0, 8.0)
    }

    pub fn head_hitbox(&self) -> (f64, f64, f64, f64) {
        (self.pos.x + 1.0, self.pos.y, 6.0, 4.0)
    }
}

/// Koopa state
#[derive(Clone, Debug, PartialEq)]
pub enum KoopaState {
    Walking,
    Shell,
    ShellMoving,
}

/// A Koopa enemy (turtle)
#[derive(Clone, Debug)]
pub struct Koopa {
    pub pos: Vec2,
    pub vel: Vec2,
    pub facing_right: bool,
    pub alive: bool,
    pub on_ground: bool,
    pub state: KoopaState,
    pub shell_timer: u8,
    pub walk_frame: u8,
    pub walk_timer: u8,
}

impl Koopa {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(-0.5, 0.0),
            facing_right: false,
            alive: true,
            on_ground: false,
            state: KoopaState::Walking,
            shell_timer: 0,
            walk_frame: 0,
            walk_timer: 0,
        }
    }

    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        match self.state {
            KoopaState::Walking => (self.pos.x, self.pos.y, 8.0, 12.0), // Taller when walking
            _ => (self.pos.x, self.pos.y, 8.0, 8.0), // Shell is shorter
        }
    }

    pub fn head_hitbox(&self) -> (f64, f64, f64, f64) {
        (self.pos.x + 1.0, self.pos.y, 6.0, 4.0)
    }
}

/// A mushroom power-up
#[derive(Clone, Debug)]
pub struct Mushroom {
    pub pos: Vec2,
    pub vel: Vec2,
    pub active: bool,
    /// Rising animation from block
    pub rising: bool,
    pub rise_progress: f64,
    pub origin_y: f64,
}

impl Mushroom {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(1.5, 0.0),
            active: true,
            rising: true,
            rise_progress: 0.0,
            origin_y: y,
        }
    }

    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        (self.pos.x, self.pos.y, 8.0, 8.0)
    }
}

/// Brick debris particle
#[derive(Clone, Debug)]
pub struct Debris {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rotation: f64,
    pub rotation_speed: f64,
    pub alive: bool,
}

impl Debris {
    pub fn new(x: f64, y: f64, vel_x: f64, vel_y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(vel_x, vel_y),
            rotation: 0.0,
            rotation_speed: (js_sys::Math::random() - 0.5) * 0.5,
            alive: true,
        }
    }
}

/// Block type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockType {
    Brick,
    Question,
    QuestionEmpty, // After being hit
    Ground,
}

/// A block in the world (brick or question)
#[derive(Clone, Debug)]
pub struct Block {
    pub x: i32, // Tile coordinates
    pub y: i32,
    pub block_type: BlockType,
    /// Animation for being bumped
    pub bump_offset: f64,
    /// Whether this block has been hit (for question blocks)
    pub hit: bool,
}

impl Block {
    pub fn new(x: i32, y: i32, block_type: BlockType) -> Self {
        Self {
            x,
            y,
            block_type,
            bump_offset: 0.0,
            hit: false,
        }
    }

    pub fn hitbox(&self, tile_size: i32) -> (f64, f64, f64, f64) {
        (
            (self.x * tile_size) as f64,
            (self.y * tile_size) as f64 + self.bump_offset,
            tile_size as f64,
            tile_size as f64,
        )
    }
}

/// A platform (row of ground blocks)
#[derive(Clone, Debug)]
pub struct Platform {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub is_ground: bool,
}

impl Platform {
    pub fn new(x: i32, y: i32, width: i32, is_ground: bool) -> Self {
        Self { x, y, width, is_ground }
    }

    pub fn hitbox(&self, tile_size: i32) -> (f64, f64, f64, f64) {
        (
            (self.x * tile_size) as f64,
            (self.y * tile_size) as f64,
            (self.width * tile_size) as f64,
            tile_size as f64,
        )
    }
}

/// A coin collectible
#[derive(Clone, Debug)]
pub struct Coin {
    pub pos: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    pub collected: bool,
    pub bounce_timer: u8,
}

impl Coin {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(0.0, -4.0), // Pop up when spawned
            on_ground: false,
            collected: false,
            bounce_timer: 0,
        }
    }

    pub fn hitbox(&self) -> (f64, f64, f64, f64) {
        (self.pos.x, self.pos.y, 8.0, 8.0)
    }
}

/// The complete game world state
#[derive(Clone)]
pub struct GameWorld {
    pub platforms: Vec<Platform>,
    pub blocks: Vec<Block>,
    pub marios: Vec<Mario>,
    pub goombas: Vec<Goomba>,
    pub koopas: Vec<Koopa>,
    pub mushrooms: Vec<Mushroom>,
    pub coins: Vec<Coin>,
    pub debris: Vec<Debris>,
    pub width: i32,
    pub height: i32,
    pub tile_size: i32,
    pub player_mario_id: Option<u32>,
    pub next_mario_id: u32,
    pub spawn_timer: u32,
}

impl GameWorld {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            platforms: Vec::new(),
            blocks: Vec::new(),
            marios: Vec::new(),
            goombas: Vec::new(),
            koopas: Vec::new(),
            mushrooms: Vec::new(),
            coins: Vec::new(),
            debris: Vec::new(),
            width,
            height,
            tile_size: 16,
            player_mario_id: None,
            next_mario_id: 0,
            spawn_timer: 0,
        }
    }

    pub fn player_mario(&self) -> Option<&Mario> {
        self.player_mario_id.and_then(|id| {
            self.marios.iter().find(|m| m.id == id && m.alive)
        })
    }

    pub fn player_mario_mut(&mut self) -> Option<&mut Mario> {
        let id = self.player_mario_id?;
        self.marios.iter_mut().find(|m| m.id == id && m.alive)
    }

    pub fn set_player(&mut self, id: u32) {
        if let Some(old_id) = self.player_mario_id {
            if let Some(mario) = self.marios.iter_mut().find(|m| m.id == old_id) {
                mario.is_player = false;
            }
        }
        if let Some(mario) = self.marios.iter_mut().find(|m| m.id == id) {
            mario.is_player = true;
            self.player_mario_id = Some(id);
        }
    }

    pub fn next_id(&mut self) -> u32 {
        let id = self.next_mario_id;
        self.next_mario_id += 1;
        id
    }

    /// Find block at tile position
    pub fn get_block_at(&self, tx: i32, ty: i32) -> Option<&Block> {
        self.blocks.iter().find(|b| b.x == tx && b.y == ty)
    }

    /// Find block index at tile position
    pub fn get_block_index_at(&self, tx: i32, ty: i32) -> Option<usize> {
        self.blocks.iter().position(|b| b.x == tx && b.y == ty)
    }
}
