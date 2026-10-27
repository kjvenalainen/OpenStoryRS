#[derive(Default)]
struct GameplayInput {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    sit: bool,
    jump_buffer: u8,
    jump_down_ticks: u8,
}

struct ScrollingNoticeState {
    texture: GpuTexture,
    width: f32,
    x: f32,
}

struct ServerNoticePopup {
    texture: GpuTexture,
    panel: SpriteRect,
    button: SpriteRect,
}

struct EventListEntry {
    event: openstory_protocol::ServerEventInfo,
    peak_duration: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RpsGamePhase {
    Waiting,
    Selecting,
    Result,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RpsGameButton {
    Start,
    Rock,
    Paper,
    Scissor,
    Continue,
    Retry,
    Exit,
}

struct RpsGameRuntime {
    phase: RpsGamePhase,
    player_choice: Option<i8>,
    opponent_choice: Option<i8>,
    result: Option<i8>,
    wins: i16,
    hovered: Option<RpsGameButton>,
    texture: Option<GpuTexture>,
}

impl RpsGameRuntime {
    fn new() -> Self {
        Self {
            phase: RpsGamePhase::Waiting,
            player_choice: None,
            opponent_choice: None,
            result: None,
            wins: 0,
            hovered: None,
            texture: None,
        }
    }
}

struct AvatarMegaphoneUi {
    animation: MapleTvAnimation,
    text_texture: GpuTexture,
    avatar_draws: Vec<(usize, SpriteRect)>,
    expires_at: Instant,
}

struct MonsterCarnivalUi {
    state: monster_carnival::State,
    texture: Option<GpuTexture>,
    close: monster_carnival::HitRect,
    side: monster_carnival::HitRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OptionControl {
    GraphicsQuality,
    EffectsQuality,
    Music,
    MuteMusic,
    SoundEffects,
    MuteEffects,
    HpWarning,
    MpWarning,
    MouseSpeed,
    ScreenShake,
    Windowed,
    Resolution800,
    Resolution1024,
    AllowWhisper,
    AllowChatInvite,
    AllowPartyInvite,
    AllowGuildChat,
    AllowAllianceChat,
    AllowFamilyInvite,
    AllowFriendChat,
    AllowTradeRequest,
    AllowExpeditionInvite,
    AllowGuildInvite,
    AllowAllianceInvite,
    AllowFollow,
}

impl OptionControl {
    fn next(self) -> Self {
        match self {
            Self::GraphicsQuality => Self::EffectsQuality,
            Self::EffectsQuality => Self::Music,
            Self::Music => Self::MuteMusic,
            Self::MuteMusic => Self::SoundEffects,
            Self::SoundEffects => Self::MuteEffects,
            Self::MuteEffects => Self::HpWarning,
            Self::HpWarning => Self::MpWarning,
            Self::MpWarning => Self::MouseSpeed,
            Self::MouseSpeed => Self::ScreenShake,
            Self::ScreenShake => Self::Windowed,
            Self::Windowed => Self::Resolution800,
            Self::Resolution800 => Self::Resolution1024,
            Self::Resolution1024 => Self::AllowWhisper,
            Self::AllowWhisper => Self::AllowChatInvite,
            Self::AllowChatInvite => Self::AllowPartyInvite,
            Self::AllowPartyInvite => Self::AllowGuildChat,
            Self::AllowGuildChat => Self::AllowAllianceChat,
            Self::AllowAllianceChat => Self::AllowFamilyInvite,
            Self::AllowFamilyInvite => Self::AllowFriendChat,
            Self::AllowFriendChat => Self::AllowTradeRequest,
            Self::AllowTradeRequest => Self::AllowExpeditionInvite,
            Self::AllowExpeditionInvite => Self::AllowGuildInvite,
            Self::AllowGuildInvite => Self::AllowAllianceInvite,
            Self::AllowAllianceInvite => Self::AllowFollow,
            Self::AllowFollow => Self::GraphicsQuality,
        }
    }

    fn track_y(self) -> Option<usize> {
        match self {
            Self::GraphicsQuality => Some(56),
            Self::EffectsQuality => Some(86),
            Self::Music => Some(116),
            Self::MuteMusic => None,
            Self::SoundEffects => Some(146),
            Self::MuteEffects => None,
            Self::HpWarning => Some(176),
            Self::MpWarning => Some(206),
            Self::MouseSpeed => Some(236),
            Self::ScreenShake => None,
            Self::Windowed | Self::Resolution800 | Self::Resolution1024 => None,
            Self::AllowWhisper
            | Self::AllowChatInvite
            | Self::AllowPartyInvite
            | Self::AllowGuildChat
            | Self::AllowAllianceChat
            | Self::AllowFamilyInvite
            | Self::AllowFriendChat
            | Self::AllowTradeRequest
            | Self::AllowExpeditionInvite
            | Self::AllowGuildInvite
            | Self::AllowAllianceInvite
            | Self::AllowFollow => None,
        }
    }

    fn is_toggle(self) -> bool {
        self == Self::ScreenShake || self.track_y().is_none()
    }
}

#[derive(Clone, Debug)]
struct SkillUiEntry {
    id: i32,
    name: String,
    level: i32,
    master_level: i32,
    max_level: i32,
    required_weapon: Option<i32>,
    passive: bool,
    attack: bool,
    magic_damage: bool,
    magic_damage_multiplier: f64,
    magic_mastery: f64,
    element: Option<char>,
    attack_count: u8,
    bullet_count: u8,
    mob_count: u8,
    damage_multiplier: f64,
    horizontal_range_multiplier: f64,
    attack_range: Option<[i16; 4]>,
    fixed_damage: Option<i32>,
    mp_cost: u16,
    hp_cost: u16,
    bullet_cost: u16,
    requirements: Vec<(i32, i32)>,
}

#[derive(Clone, Debug)]
struct KeyConfigAction {
    label: String,
    kind: u8,
    action: i32,
    assignable: bool,
}

#[derive(Clone, Copy)]
struct ActiveCharacterBuff {
    value: i16,
    skill_id: i32,
    duration_ms: i32,
    applied_at: Instant,
    expires_at: Option<Instant>,
}

#[derive(Clone, Copy)]
struct PendingSkillHitSound {
    skill_id: i32,
    position: (i16, i16),
    play_at: Instant,
}

#[derive(Clone, Copy, Debug, Default)]
struct ForcedEquipmentStats {
    weapon_attack: Option<i16>,
    weapon_defense: Option<i16>,
    magic_attack: Option<i16>,
    magic_defense: Option<i16>,
    accuracy: Option<i16>,
    avoidability: Option<i16>,
    speed: Option<u8>,
    jump: Option<u8>,
}

struct MapSceneData {
    map_id: i32,
    underwater: bool,
    footholds: Option<FootholdTree>,
    spawn: (f64, f64),
    camera_offset: (f32, f32),
    camera_bounds: Option<MapCameraBounds>,
    seats: Vec<(i16, i16)>,
    ladders: Vec<MapLadder>,
    world_draw_indices: Vec<usize>,
    map_backgrounds: Vec<MapBackgroundAsset>,
    layer_insert_indices: [usize; 8],
    foreground_start_index: usize,
    map_art_animations: Vec<MapArtAnimationAsset>,
    avatar_frames: Vec<AvatarFrameAsset>,
    minimap: Option<minimap::MiniMapData>,
    portals: Vec<MapPortalAsset>,
    regular_portal_frames: Vec<SpriteFrameAsset>,
    hidden_portal_frames: Vec<SpriteFrameAsset>,
    door_frames: Vec<SpriteFrameAsset>,
}

#[derive(Clone, Copy, Debug)]
struct MapCameraBounds {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl MapCameraBounds {
    fn offset_for(self, world_x: f64, world_y: f64) -> (f32, f32) {
        let mut x = 512.0 - world_x as f32;
        let mut y = GAMEPLAY_CAMERA_CENTER_Y - world_y as f32;
        let width = 1024.0;
        let height = 768.0;
        if self.right - self.left < width {
            x = -self.left;
        } else {
            x = x.clamp(width - self.right, -self.left);
        }
        if self.bottom - self.top < height {
            y = height - self.bottom;
        } else {
            y = y.clamp(height - self.bottom, -self.top);
        }
        (x, y)
    }
}

#[derive(Clone, Copy, Debug)]
struct MapLadder {
    x: i16,
    y1: i16,
    y2: i16,
    is_ladder: bool,
}

impl MapLadder {
    fn in_range(self, x: i16, y: i16, upwards: bool) -> bool {
        let vertical = if upwards {
            y.saturating_sub(5)
        } else {
            y.saturating_add(5)
        };
        (i32::from(x) - i32::from(self.x)).abs() <= 10 && (self.y1..=self.y2).contains(&vertical)
    }

    fn fell_off(self, y: i16, downwards: bool) -> bool {
        let beyond_end = if downwards {
            y.saturating_add(5) > self.y2
        } else {
            y.saturating_sub(5) > self.y2
        };
        beyond_end || y.saturating_add(5) < self.y1
    }
}

struct MapPortalAsset {
    portal_type: u8,
    name: String,
    target_name: String,
    target_map_id: i32,
    x: f64,
    y: f64,
}

struct SpriteFrameAsset {
    delay_ms: u16,
    sprites: Range<usize>,
}

struct MapArtAnimationAsset {
    frames: Vec<SpriteFrameAsset>,
    environment_name: Option<String>,
    hold_last: bool,
}

struct MapBackgroundAsset {
    sprites: Range<usize>,
    environment_name: Option<String>,
    horizontal_tiled: bool,
    vertical_tiled: bool,
    tile_width: f32,
    tile_height: f32,
    tile_count_x: usize,
    tile_count_y: usize,
    horizontal_mobile: bool,
    vertical_mobile: bool,
    horizontal_speed: f32,
    vertical_speed: f32,
    horizontal_parallax: f32,
    vertical_parallax: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AvatarMotion {
    Stand,
    Walk,
    Alert,
    Crouch,
    Jump,
    Swim,
    Ladder,
    Rope,
    Sit,
    Attack(u8),
}

const FACE_EXPRESSION_NAMES: [&str; 24] = [
    "default",
    "blink",
    "hit",
    "smile",
    "troubled",
    "cry",
    "angry",
    "bewildered",
    "stunned",
    "blaze",
    "bowing",
    "cheers",
    "chu",
    "dam",
    "despair",
    "glitter",
    "hot",
    "hum",
    "love",
    "oops",
    "pain",
    "shine",
    "vomit",
    "wink",
];

fn face_expression_index(action: i32) -> Option<u8> {
    let index = match action {
        1..=23 => action,
        99..=121 => action - 98,
        _ => return None,
    };
    u8::try_from(index).ok()
}

fn avatar_motion_for_stance(stance: u8) -> AvatarMotion {
    match stance & !1 {
        2 => AvatarMotion::Walk,
        4 | 18 => AvatarMotion::Stand,
        6 => AvatarMotion::Jump,
        8 => AvatarMotion::Alert,
        10 => AvatarMotion::Crouch,
        12 => AvatarMotion::Swim,
        14 => AvatarMotion::Ladder,
        16 => AvatarMotion::Rope,
        20 => AvatarMotion::Sit,
        _ => AvatarMotion::Stand,
    }
}

fn first_avatar_frame(frames: &[AvatarFrameRuntime], motion: AvatarMotion) -> Option<usize> {
    frames
        .iter()
        .position(|frame| frame.motion == motion)
        .or_else(|| {
            let fallback = match motion {
                AvatarMotion::Stand => AvatarMotion::Walk,
                AvatarMotion::Sit => AvatarMotion::Stand,
                AvatarMotion::Alert | AvatarMotion::Crouch | AvatarMotion::Swim => {
                    AvatarMotion::Stand
                }
                AvatarMotion::Walk
                | AvatarMotion::Jump
                | AvatarMotion::Ladder
                | AvatarMotion::Rope => AvatarMotion::Stand,
                AvatarMotion::Attack(_) => AvatarMotion::Stand,
            };
            frames.iter().position(|frame| frame.motion == fallback)
        })
        .or_else(|| (!frames.is_empty()).then_some(0))
}

struct AvatarFrameAsset {
    motion: AvatarMotion,
    delay_ms: u16,
    sprites: Range<usize>,
    face_sprite: Option<usize>,
    face_offset: (f32, f32),
}

struct AvatarFrameRuntime {
    motion: AvatarMotion,
    delay: Duration,
    face_offset: (f32, f32),
    sprites: Vec<AvatarSpriteRuntime>,
}

struct AvatarSpriteRuntime {
    texture_index: usize,
    draw_index: Option<usize>,
    rect: SpriteRect,
    is_face: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MountMotion {
    Stand,
    Walk,
    Jump,
    Rope,
    Ladder,
    Prone,
    Fly,
}

struct MountFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
    navel: (f32, f32),
}

struct MountFrameAsset {
    delay_ms: u16,
    sprites: Range<usize>,
    navel: (f32, f32),
}

struct MountAnimationRuntime {
    frames: Vec<MountFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct MountLookRuntime {
    animations: HashMap<MountMotion, MountAnimationRuntime>,
}

impl MountLookRuntime {
    fn motion_for_avatar(motion: AvatarMotion) -> MountMotion {
        match motion {
            AvatarMotion::Walk => MountMotion::Walk,
            AvatarMotion::Jump => MountMotion::Jump,
            AvatarMotion::Swim => MountMotion::Fly,
            AvatarMotion::Rope => MountMotion::Rope,
            AvatarMotion::Ladder => MountMotion::Ladder,
            AvatarMotion::Crouch => MountMotion::Prone,
            AvatarMotion::Stand
            | AvatarMotion::Alert
            | AvatarMotion::Sit
            | AvatarMotion::Attack(_) => MountMotion::Stand,
        }
    }

    fn current_animation(&self, motion: MountMotion) -> Option<&MountAnimationRuntime> {
        self.animations
            .get(&motion)
            .filter(|animation| !animation.frames.is_empty())
            .or_else(|| self.animations.get(&MountMotion::Stand))
    }

    fn current_frame(&self, motion: MountMotion) -> Option<&MountFrameRuntime> {
        let animation = self.current_animation(motion)?;
        animation.frames.get(animation.frame_index)
    }

    fn seat_offset(&self, motion: MountMotion) -> (f32, f32) {
        self.current_frame(motion)
            .map_or((0.0, -16.0), |frame| (frame.navel.0, frame.navel.1 + 14.0))
    }

    fn advance(&mut self, motion: MountMotion, elapsed: Duration) {
        let active_motion = if self.animations.contains_key(&motion) {
            motion
        } else {
            MountMotion::Stand
        };
        let Some(animation) = self.animations.get_mut(&active_motion) else {
            return;
        };
        if animation.frames.len() < 2 {
            return;
        }
        animation.elapsed = animation.elapsed.saturating_add(elapsed);
        loop {
            let Some(frame) = animation.frames.get(animation.frame_index) else {
                animation.frame_index = 0;
                break;
            };
            if animation.elapsed < frame.delay {
                break;
            }
            animation.elapsed -= frame.delay;
            animation.frame_index = (animation.frame_index + 1) % animation.frames.len();
        }
    }
}

struct LocalDeathFrameAsset {
    delay_ms: u16,
    sprites: Range<usize>,
}

struct LocalDeathArtAsset {
    fall_frames: Vec<LocalDeathFrameAsset>,
    landed_sprites: Range<usize>,
    ghost_frames: Vec<LocalDeathFrameAsset>,
}

struct LocalDeathFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

struct LocalDeathArtRuntime {
    fall_frames: Vec<LocalDeathFrameRuntime>,
    landed_sprites: Vec<(usize, SpriteRect)>,
    ghost_frames: Vec<LocalDeathFrameRuntime>,
    tomb_y_offset: f32,
    tomb_landed: bool,
    tomb_frame_index: usize,
    tomb_elapsed: Duration,
    ghost_frame_index: usize,
    ghost_elapsed: Duration,
    ghost_ticks: u64,
}

impl LocalDeathArtRuntime {
    fn reset(&mut self) {
        self.tomb_y_offset = -420.0;
        self.tomb_landed = false;
        self.tomb_frame_index = 0;
        self.tomb_elapsed = Duration::ZERO;
        self.ghost_frame_index = 0;
        self.ghost_elapsed = Duration::ZERO;
        self.ghost_ticks = 0;
    }

    fn advance(&mut self, updates: u8) {
        for _ in 0..updates {
            if !self.tomb_landed {
                self.tomb_y_offset += 9.0;
                self.tomb_elapsed += Duration::from_millis(8);
                if self.tomb_elapsed >= Duration::from_millis(30) {
                    self.tomb_elapsed = Duration::ZERO;
                    if !self.fall_frames.is_empty() {
                        self.tomb_frame_index = (self.tomb_frame_index + 1) % self.fall_frames.len();
                    }
                }
                if self.tomb_y_offset >= 0.0 {
                    self.tomb_y_offset = 0.0;
                    self.tomb_landed = true;
                }
            }
            if let Some(frame) = self.ghost_frames.get(self.ghost_frame_index) {
                self.ghost_ticks = self.ghost_ticks.wrapping_add(1);
                self.ghost_elapsed += Duration::from_millis(8);
                if self.ghost_elapsed >= frame.delay {
                    self.ghost_elapsed = Duration::ZERO;
                    self.ghost_frame_index = (self.ghost_frame_index + 1) % self.ghost_frames.len();
                }
            }
        }
    }

    fn retain_timeline_from(&mut self, previous: &Self) {
        self.tomb_y_offset = previous.tomb_y_offset;
        self.tomb_landed = previous.tomb_landed;
        self.tomb_frame_index = if self.fall_frames.is_empty() {
            0
        } else {
            previous.tomb_frame_index % self.fall_frames.len()
        };
        self.tomb_elapsed = previous.tomb_elapsed;
        self.ghost_frame_index = if self.ghost_frames.is_empty() {
            0
        } else {
            previous.ghost_frame_index % self.ghost_frames.len()
        };
        self.ghost_elapsed = previous.ghost_elapsed;
        self.ghost_ticks = previous.ghost_ticks;
    }
}

#[derive(Clone)]
struct FaceExpressionFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

#[derive(Clone)]
struct FaceExpressionRuntime {
    frames: Vec<FaceExpressionFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

fn avatar_frames_runtime(
    assets: Vec<AvatarFrameAsset>,
    draws: &[(usize, SpriteRect)],
) -> Vec<AvatarFrameRuntime> {
    assets
        .into_iter()
        .map(|frame| AvatarFrameRuntime {
            motion: frame.motion,
            delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
            face_offset: frame.face_offset,
            sprites: frame
                .sprites
                .filter_map(|draw_index| {
                    draws
                        .get(draw_index)
                        .map(|(texture_index, rect)| AvatarSpriteRuntime {
                            texture_index: *texture_index,
                            draw_index: Some(draw_index),
                            rect: *rect,
                            is_face: frame.face_sprite == Some(draw_index),
                        })
                })
                .collect(),
        })
        .collect()
}

fn uploaded_avatar_frames_runtime(
    assets: Vec<AvatarFrameAsset>,
    draws: &[(usize, SpriteRect)],
) -> Vec<AvatarFrameRuntime> {
    assets
        .into_iter()
        .map(|frame| AvatarFrameRuntime {
            motion: frame.motion,
            delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
            face_offset: frame.face_offset,
            sprites: frame
                .sprites
                .filter_map(|index| {
                    draws
                        .get(index)
                        .map(|(texture_index, rect)| AvatarSpriteRuntime {
                            texture_index: *texture_index,
                            draw_index: None,
                            rect: *rect,
                            is_face: frame.face_sprite == Some(index),
                        })
                })
                .collect(),
        })
        .collect()
}

fn local_death_art_runtime(
    assets: LocalDeathArtAsset,
    draws: &[(usize, SpriteRect)],
) -> Option<LocalDeathArtRuntime> {
    let convert_frames = |frames: Vec<LocalDeathFrameAsset>| {
        frames
            .into_iter()
            .filter_map(|frame| {
                let sprites = frame
                    .sprites
                    .filter_map(|index| draws.get(index).copied())
                    .collect::<Vec<_>>();
                (!sprites.is_empty()).then_some(LocalDeathFrameRuntime {
                    delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                    sprites,
                })
            })
            .collect::<Vec<_>>()
    };
    let fall_frames = convert_frames(assets.fall_frames);
    let ghost_frames = convert_frames(assets.ghost_frames);
    let landed_sprites = assets
        .landed_sprites
        .filter_map(|index| draws.get(index).copied())
        .collect::<Vec<_>>();
    if fall_frames.is_empty() && ghost_frames.is_empty() && landed_sprites.is_empty() {
        return None;
    }
    Some(LocalDeathArtRuntime {
        tomb_y_offset: -420.0,
        tomb_landed: false,
        fall_frames,
        landed_sprites,
        ghost_frames,
        tomb_frame_index: 0,
        tomb_elapsed: Duration::ZERO,
        ghost_frame_index: 0,
        ghost_elapsed: Duration::ZERO,
        ghost_ticks: 0,
    })
}

fn advance_face_expression(expression: &mut Option<FaceExpressionRuntime>, updates: u8) {
    let Some(expression_state) = expression.as_mut() else {
        return;
    };
    expression_state.elapsed = expression_state
        .elapsed
        .saturating_add(Duration::from_millis(8 * u64::from(updates)));
    let mut finished = false;
    loop {
        let Some(frame) = expression_state.frames.get(expression_state.frame_index) else {
            finished = true;
            break;
        };
        if expression_state.elapsed < frame.delay {
            break;
        }
        expression_state.elapsed -= frame.delay;
        if expression_state.frame_index + 1 >= expression_state.frames.len() {
            finished = true;
            break;
        }
        expression_state.frame_index += 1;
    }
    if finished {
        *expression = None;
    }
}

fn projectile_frames_runtime(
    assets: Vec<ProjectileFrameAsset>,
    draws: &[(usize, SpriteRect)],
) -> Vec<ProjectileFrameRuntime> {
    assets
        .into_iter()
        .filter_map(|frame| {
            let sprites = frame
                .sprites
                .filter_map(|index| draws.get(index).copied())
                .collect::<Vec<_>>();
            (!sprites.is_empty()).then_some(ProjectileFrameRuntime {
                delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                sprites,
            })
        })
        .collect()
}

fn reactor_frame_runtime(
    asset: &ReactorFrameAsset,
    draws: &[(usize, SpriteRect)],
) -> Option<ReactorFrameRuntime> {
    let sprites = asset
        .sprites
        .clone()
        .filter_map(|index| draws.get(index).copied())
        .collect::<Vec<_>>();
    (!sprites.is_empty()).then_some(ReactorFrameRuntime {
        delay: Duration::from_millis(u64::from(asset.delay_ms.max(1))),
        sprites,
    })
}

fn portal_frame_runtime(
    asset: &SpriteFrameAsset,
    draws: &[(usize, SpriteRect)],
) -> PortalFrameRuntime {
    PortalFrameRuntime {
        delay: Duration::from_millis(u64::from(asset.delay_ms.max(1))),
        sprites: asset
            .sprites
            .clone()
            .filter_map(|index| draws.get(index).copied())
            .collect(),
    }
}

fn mob_frames_runtime(
    assets: Vec<MobFrameAsset>,
    draws: &[(usize, SpriteRect)],
) -> HashMap<MobMotion, Vec<MobFrameRuntime>> {
    let mut animations: HashMap<MobMotion, Vec<MobFrameRuntime>> = HashMap::new();
    for frame in assets {
        let sprites = frame
            .sprites
            .filter_map(|index| draws.get(index).copied())
            .collect::<Vec<_>>();
        if !sprites.is_empty() {
            animations
                .entry(frame.motion)
                .or_default()
                .push(MobFrameRuntime {
                    delay: Duration::from_millis(u64::from(frame.delay_ms.max(1))),
                    sprites,
                });
        }
    }
    animations
}

struct MapSceneRuntime {
    map_id: i32,
    underwater: bool,
    camera_offset: (f32, f32),
    camera_bounds: Option<MapCameraBounds>,
    minimap: Option<minimap::MiniMapData>,
    minimap_presentation: minimap::Presentation,
    footholds: Option<FootholdTree>,
    map_seats: Vec<(i16, i16)>,
    map_ladders: Vec<MapLadder>,
    local_ladder: Option<MapLadder>,
    climb_cooldown: u8,
    player: PhysicsBody,
    world_draw_indices: Vec<usize>,
    map_backgrounds: Vec<MapBackgroundRuntime>,
    map_art_animations: Vec<MapArtAnimationRuntime>,
    map_art_animation_draw_indices: HashSet<usize>,
    active_map_art_draw_indices: HashSet<usize>,
    inactive_environment_draw_indices: HashSet<usize>,
    avatar_frames: Vec<AvatarFrameRuntime>,
    avatar_indices: HashSet<usize>,
    active_avatar_frame: Option<usize>,
    local_death_art: Option<LocalDeathArtRuntime>,
    local_mount: Option<MountLookRuntime>,
    desired_avatar_motion: AvatarMotion,
    attack_stance: Option<u8>,
    attack_count: u32,
    attack_random_state: u32,
    avatar_elapsed: Duration,
    facing_right: bool,
    last_movement: Option<openstory_protocol::PlayerMovement>,
    remote_players: HashMap<i32, RemotePlayerState>,
    local_item_aura: Option<PersistentItemAuraRuntime>,
    local_face_expression: Option<FaceExpressionRuntime>,
    local_speech_bubble_remaining: Option<Duration>,
    local_chalkboard: Option<String>,
    npcs: HashMap<i32, MapNpcRuntime>,
    hired_merchants: HashMap<i32, MapHiredMerchantRuntime>,
    scriptable_npc_ids: HashSet<i32>,
    drops: HashMap<i32, MapDropRuntime>,
    mobs: HashMap<i32, MapMobRuntime>,
    mists: HashMap<i32, openstory_protocol::MapMistSpawn>,
    summons: HashMap<i32, MapSummonRuntime>,
    dragons: HashMap<i32, MapDragonRuntime>,
    pets: HashMap<(i32, u8), MapPetRuntime>,
    pet_loot_cooldown: u8,
    character_effects: Vec<MapCharacterSkillEffect>,
    screen_effect: Option<MapScreenEffect>,
    reactors: HashMap<i32, MapReactorRuntime>,
    doors: HashMap<i32, MapDoorRuntime>,
    portals: Vec<MapPortalRuntime>,
    regular_portal_frames: Vec<PortalFrameRuntime>,
    hidden_portal_frames: Vec<PortalFrameRuntime>,
    door_frames: Vec<PortalFrameRuntime>,
    portal_frame_indices: HashSet<usize>,
    regular_portal_frame_index: usize,
    regular_portal_elapsed: Duration,
    hidden_portal_frame_index: usize,
    hidden_portal_elapsed: Duration,
    portal_cooldown: u8,
    teleport_cooldown: u8,
    touch_damage_cooldown: Duration,
    local_invincibility_remaining: Duration,
    idle_regen_elapsed: Duration,
    regen_hit_lockout_remaining: Duration,
    local_chair: Option<LocalChairRuntime>,
    local_map_seat: Option<(i16, i16)>,
    damage_numbers: Vec<MobDamageNumber>,
    pending_mob_attack_impacts: Vec<PendingMobAttackImpact>,
    quest_clear_effect: Option<QuestClearEffectRuntime>,
    projectiles: Vec<MapProjectileRuntime>,
    layer_insert_indices: [usize; 8],
    foreground_start_index: usize,
}

struct QuestClearEffectRuntime {
    frames: Vec<NpcQuestMarkFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct MapArtAnimationRuntime {
    frames: Vec<SpriteFrameAsset>,
    environment_name: Option<String>,
    hold_last: bool,
    frame_index: usize,
    elapsed: Duration,
}

fn prepare_map_art_animations(
    animations: Vec<MapArtAnimationAsset>,
) -> (Vec<MapArtAnimationRuntime>, HashSet<usize>, HashSet<usize>) {
    let mut animated_draw_indices = HashSet::new();
    let mut active_draw_indices = HashSet::new();
    let runtimes = animations
        .into_iter()
        .filter_map(|animation| {
            if animation.frames.len() < 2 {
                return None;
            }
            for frame in &animation.frames {
                animated_draw_indices.extend(frame.sprites.clone());
            }
            if let Some(frame) = animation.frames.first() {
                active_draw_indices.extend(frame.sprites.clone());
            }
            Some(MapArtAnimationRuntime {
                frames: animation.frames,
                environment_name: animation.environment_name,
                hold_last: animation.hold_last,
                frame_index: 0,
                elapsed: Duration::ZERO,
            })
        })
        .collect();
    (runtimes, animated_draw_indices, active_draw_indices)
}

struct MapBackgroundRuntime {
    sprites: Range<usize>,
    environment_name: Option<String>,
    active: bool,
    horizontal_tiled: bool,
    vertical_tiled: bool,
    tile_width: f32,
    tile_height: f32,
    tile_count_x: usize,
    tile_count_y: usize,
    horizontal_mobile: bool,
    vertical_mobile: bool,
    horizontal_speed: f32,
    vertical_speed: f32,
    horizontal_parallax: f32,
    vertical_parallax: f32,
    horizontal_offset: f32,
    vertical_offset: f32,
}

#[derive(Clone)]
struct CharacterSkillEffectAnimation {
    z: i8,
    frames: Vec<ProjectileFrameRuntime>,
}

struct CharacterSkillEffectAsset {
    z: i8,
    frames: Vec<ProjectileFrameAsset>,
}

#[derive(Clone)]
struct SkillHitEffectAnimation {
    pos: i8,
    z: i8,
    frames: Vec<ProjectileFrameRuntime>,
}

struct SkillHitEffectAsset {
    pos: i8,
    z: i8,
    frames: Vec<ProjectileFrameAsset>,
}

struct MapCharacterSkillEffect {
    character_id: Option<i32>,
    point_position: Option<(f64, f64)>,
    z: i8,
    speed_factor: f32,
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct MapScreenEffect {
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct MapMobHitEffect {
    z: i8,
    flip_x: bool,
    anchor_x: f32,
    anchor_y: f32,
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct PendingMobAttackImpact {
    object_id: i32,
    remaining: Duration,
    hit_effects: Arc<Vec<SkillHitEffectAnimation>>,
    hit_effect_flip: bool,
    damage: Option<(openstory_protocol::MobDamage, bool, f32)>,
}

struct MapProjectileRuntime {
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    frame_elapsed: Duration,
    x: f64,
    y: f64,
    target_x: f64,
    target_y: f64,
    facing_left: bool,
}

#[derive(Clone)]
struct ProjectileFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

#[derive(Clone, Default)]
struct PersistentItemAuraFrames {
    back: Vec<ProjectileFrameRuntime>,
    front: Vec<ProjectileFrameRuntime>,
    split: bool,
}

#[derive(Clone)]
struct PersistentItemAuraRuntime {
    frames: PersistentItemAuraFrames,
    back_index: usize,
    back_elapsed: Duration,
    front_index: usize,
    front_elapsed: Duration,
}

impl PersistentItemAuraRuntime {
    fn new(frames: PersistentItemAuraFrames) -> Option<Self> {
        (!frames.back.is_empty() || !frames.front.is_empty()).then_some(Self {
            frames,
            back_index: 0,
            back_elapsed: Duration::ZERO,
            front_index: 0,
            front_elapsed: Duration::ZERO,
        })
    }

    fn advance(&mut self, elapsed: Duration) {
        advance_looping_frames(
            &self.frames.back,
            &mut self.back_index,
            &mut self.back_elapsed,
            elapsed,
        );
        advance_looping_frames(
            &self.frames.front,
            &mut self.front_index,
            &mut self.front_elapsed,
            elapsed,
        );
    }

    fn below(&self) -> Option<&ProjectileFrameRuntime> {
        if self.frames.split {
            self.frames.back.get(self.back_index)
        } else {
            self.frames.front.get(self.front_index)
        }
    }

    fn above(&self) -> Option<&ProjectileFrameRuntime> {
        self.frames
            .split
            .then(|| self.frames.front.get(self.front_index))
            .flatten()
    }
}

fn advance_looping_frames(
    frames: &[ProjectileFrameRuntime],
    frame_index: &mut usize,
    elapsed: &mut Duration,
    delta: Duration,
) {
    if frames.is_empty() {
        return;
    }
    *elapsed = elapsed.saturating_add(delta);
    while let Some(frame) = frames.get(*frame_index) {
        if *elapsed < frame.delay {
            break;
        }
        *elapsed -= frame.delay;
        *frame_index = (*frame_index + 1) % frames.len();
    }
}

struct ChairAnimationRuntime {
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct LocalChairRuntime {
    behind: ChairAnimationRuntime,
    front: ChairAnimationRuntime,
    sit_offset_y: f32,
}

impl LocalChairRuntime {
    fn empty() -> Self {
        Self {
            behind: ChairAnimationRuntime {
                frames: Vec::new(),
                frame_index: 0,
                elapsed: Duration::ZERO,
            },
            front: ChairAnimationRuntime {
                frames: Vec::new(),
                frame_index: 0,
                elapsed: Duration::ZERO,
            },
            sit_offset_y: 0.0,
        }
    }
}

struct ProjectileFrameAsset {
    delay_ms: u16,
    sprites: Range<usize>,
}

struct LocalChairAsset {
    behind: Vec<ProjectileFrameAsset>,
    front: Vec<ProjectileFrameAsset>,
    sit_offset_y: f32,
}

#[derive(Default)]
struct PersistentItemAuraAsset {
    back: Vec<ProjectileFrameAsset>,
    front: Vec<ProjectileFrameAsset>,
    split: bool,
}

struct MapPortalRuntime {
    portal_type: u8,
    name: String,
    target_name: String,
    target_map_id: i32,
    x: f64,
    y: f64,
    touched: bool,
}

#[derive(Clone)]
struct PortalFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

struct MapDoorRuntime {
    x: f64,
    y: f64,
    frame_index: usize,
    elapsed: Duration,
}

enum PortalWarp {
    SameMap { x: f64, y: f64 },
    ChangeMap { map_id: i32, portal_name: String },
}

struct MapReactorRuntime {
    state: i8,
    x: f64,
    y: f64,
    foothold_layer: u8,
    width: f64,
    height: f64,
    hittable: bool,
    idle_frame: ReactorFrameRuntime,
    hit_animations: HashMap<i8, Vec<ReactorFrameRuntime>>,
    active_frames: Option<Vec<ReactorFrameRuntime>>,
    frame_index: usize,
    frame_elapsed: Duration,
    dead: bool,
}

#[derive(Clone)]
struct ReactorFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

struct ReactorFrameAsset {
    delay_ms: u16,
    sprites: Range<usize>,
}

struct ReactorArtAssets {
    idle: ReactorFrameAsset,
    hits: HashMap<i8, Vec<ReactorFrameAsset>>,
    hittable: bool,
}

struct MobDamageNumber {
    world_x: f64,
    world_y: f64,
    digits: Vec<u8>,
    miss: bool,
    critical: bool,
    heal: bool,
    player_damage: bool,
    lifetime: Duration,
    rise_speed: f32,
    elapsed: Duration,
}

struct ChatLine {
    text: String,
    color: [u8; 3],
}

struct StatusMessageRuntime {
    text: String,
    color: [u8; 3],
    created_at: Instant,
    texture: Option<GpuTexture>,
}

#[derive(Clone, Copy)]
enum CashItemComposeKind {
    Megaphone,
    MapleTv,
    AvatarMegaphone,
    PetNameTag,
    PersonalShop,
    HiredMerchant,
}

impl CashItemComposeKind {
    const fn label(self, item_id: i32) -> &'static str {
        match self {
            Self::Megaphone if (item_id / 1_000) % 10 == 6 => "Item Megaphone",
            Self::Megaphone => "Megaphone",
            Self::MapleTv => "Maple TV",
            Self::AvatarMegaphone => "Avatar Megaphone",
            Self::PetNameTag => "Pet Name Tag",
            Self::PersonalShop => "Personal Shop",
            Self::HiredMerchant => "Hired Merchant",
        }
    }

    const fn max_chars(self) -> usize {
        match self {
            Self::PetNameTag => 12,
            Self::PersonalShop | Self::HiredMerchant => 30,
            _ => 60,
        }
    }

    const fn max_lines(self, item_id: i32) -> usize {
        match self {
            Self::Megaphone if (item_id / 1_000) % 10 == 7 => 3,
            Self::MapleTv => 5,
            Self::AvatarMegaphone => 4,
            _ => 1,
        }
    }

    const fn has_recipient_field(self, item_id: i32) -> bool {
        match self {
            Self::MapleTv => item_id != 5_075_001 && item_id != 5_075_004,
            _ => false,
        }
    }

    const fn uses_item_attachment(self, item_id: i32) -> bool {
        matches!(self, Self::Megaphone) && (item_id / 1_000) % 10 == 6
    }

    const fn has_whisper_toggle(self, item_id: i32) -> bool {
        matches!(self, Self::Megaphone) && matches!((item_id / 1_000) % 10, 2 | 6 | 7)
    }

    const fn is_item_megaphone(self, item_id: i32) -> bool {
        matches!(self, Self::Megaphone) && (item_id / 1_000) % 10 == 6
    }
}

#[derive(Clone)]
struct CashItemCompose {
    slot: i16,
    item_id: i32,
    kind: CashItemComposeKind,
    lines: Vec<String>,
    recipient: String,
    recipient_prompt_pending: bool,
    whisper: bool,
    attached_item: Option<(openstory_protocol::InventoryType, u16, i32)>,
    awaiting_attachment: bool,
}

impl CashItemCompose {
    fn input_max_chars(&self) -> usize {
        if self.recipient_prompt_pending {
            12
        } else {
            self.kind.max_chars()
        }
    }
}

struct WhisperConversation {
    target: String,
    lines: VecDeque<ChatLine>,
    input: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChatTarget {
    Public,
    Buddy,
    Guild,
    Alliance,
    Party,
}

impl ChatTarget {
    fn next(self) -> Self {
        match self {
            Self::Public => Self::Buddy,
            Self::Buddy => Self::Guild,
            Self::Guild => Self::Alliance,
            Self::Alliance => Self::Party,
            Self::Party => Self::Public,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Public => "Public",
            Self::Buddy => "Buddy",
            Self::Guild => "Guild",
            Self::Alliance => "Alliance",
            Self::Party => "Party",
        }
    }

    const fn multi_chat_type(self) -> Option<openstory_protocol::MultiChatType> {
        match self {
            Self::Public => None,
            Self::Buddy => Some(openstory_protocol::MultiChatType::Buddy),
            Self::Guild => Some(openstory_protocol::MultiChatType::Guild),
            Self::Alliance => Some(openstory_protocol::MultiChatType::Alliance),
            Self::Party => Some(openstory_protocol::MultiChatType::Party),
        }
    }
}

enum ChatCommand {
    Whisper { target: String, message: String },
    OpenWhisper(String),
    Find { target: String },
    PartyMessage(String),
    Trade(TradeCommand),
    Party(PartyCommand),
    Buddy(BuddyCommand),
    Alliance(AllianceCommand),
    Guild(GuildCommand),
}

enum TradeCommand {
    Invite(String),
    Accept,
    Decline,
}

enum PartyCommand {
    Create,
    Leave,
    Join(i32),
    Invite(String),
    Expel(String),
    Leader(String),
    Accept,
    Deny,
    List,
}

enum BuddyCommand {
    Add(String),
    ChangeGroup { target: String, group: String },
    Filter(Option<String>),
    Remove(String),
    Accept,
    Decline,
    List,
}

enum AllianceCommand {
    Invite(String),
    Expel(String),
    Leave,
    Notice(String),
    RankTitles([String; 5]),
    Accept,
    Deny,
    List,
}

enum GuildCommand {
    Create(String),
    Invite(String),
    Leave,
    Expel(String),
    MemberRank {
        target: String,
        rank: u8,
    },
    RankTitles([String; 5]),
    Emblem {
        background: i16,
        background_color: i8,
        logo: i16,
        logo_color: i8,
    },
    Notice(String),
    List,
}

#[derive(Clone)]
struct GuildMarkChoice {
    id: i16,
    category: String,
    label: String,
    colors: Vec<i8>,
}

#[derive(Clone)]
enum GuildPanelMode {
    Overview,
    Create {
        name: String,
        notice: Option<String>,
    },
    RankTitles {
        titles: [String; 5],
        selected: usize,
        notice: Option<String>,
    },
    Emblem {
        emblem: openstory_protocol::GuildEmblem,
        selected: usize,
        backgrounds: Vec<GuildMarkChoice>,
        marks: Vec<GuildMarkChoice>,
    },
}

fn parse_chat_command(message: &str) -> Result<Option<ChatCommand>, &'static str> {
    let mut parts = message.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(None);
    };
    match command.to_ascii_lowercase().as_str() {
        "/w" | "/whisper" => {
            let Some(target) = parts.next() else {
                return Err("Usage: /w <character> [message]");
            };
            let message = parts.collect::<Vec<_>>().join(" ");
            if message.is_empty() {
                return Ok(Some(ChatCommand::OpenWhisper(target.to_owned())));
            }
            Ok(Some(ChatCommand::Whisper {
                target: target.to_owned(),
                message,
            }))
        }
        "/find" => {
            let Some(target) = parts.next() else {
                return Err("Usage: /find <character>");
            };
            Ok(Some(ChatCommand::Find {
                target: target.to_owned(),
            }))
        }
        "/pt" => {
            let message = parts.collect::<Vec<_>>().join(" ");
            if message.is_empty() {
                return Err("Usage: /pt <message>");
            }
            Ok(Some(ChatCommand::PartyMessage(message)))
        }
        "/trade" => {
            let Some(action) = parts.next() else {
                return Err("Usage: /trade <character>|accept|decline");
            };
            let command = match action.to_ascii_lowercase().as_str() {
                "accept" => TradeCommand::Accept,
                "decline" | "deny" => TradeCommand::Decline,
                "invite" => {
                    let Some(target) = parts.next() else {
                        return Err("Usage: /trade <character>|accept|decline");
                    };
                    TradeCommand::Invite(target.to_owned())
                }
                _ => TradeCommand::Invite(action.to_owned()),
            };
            Ok(Some(ChatCommand::Trade(command)))
        }
        "/party" => {
            let Some(action) = parts.next() else {
                return Err("Usage: /party create|leave|invite|join|expel|leader|accept|deny|list");
            };
            let command = match action.to_ascii_lowercase().as_str() {
                "create" => PartyCommand::Create,
                "leave" => PartyCommand::Leave,
                "join" => {
                    let Some(party_id) = parts.next().and_then(|value| value.parse().ok()) else {
                        return Err("Usage: /party join <party id>");
                    };
                    PartyCommand::Join(party_id)
                }
                "invite" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /party invite <character>");
                    };
                    PartyCommand::Invite(name.to_owned())
                }
                "expel" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /party expel <character>");
                    };
                    PartyCommand::Expel(name.to_owned())
                }
                "leader" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /party leader <character>");
                    };
                    PartyCommand::Leader(name.to_owned())
                }
                "accept" => PartyCommand::Accept,
                "deny" => PartyCommand::Deny,
                "list" => PartyCommand::List,
                _ => {
                    return Err(
                        "Usage: /party create|leave|invite|join|expel|leader|accept|deny|list",
                    );
                }
            };
            Ok(Some(ChatCommand::Party(command)))
        }
        "/buddy" => {
            let Some(action) = parts.next() else {
                return Err("Usage: /buddy add|group|filter|remove|accept|dismiss|list");
            };
            let command = match action.to_ascii_lowercase().as_str() {
                "add" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /buddy add <character>");
                    };
                    BuddyCommand::Add(name.to_owned())
                }
                "group" => {
                    let Some(target) = parts.next() else {
                        return Err("Usage: /buddy group <character> <group name>");
                    };
                    let group = parts.collect::<Vec<_>>().join(" ");
                    if group.is_empty() {
                        return Err("Usage: /buddy group <character> <group name>");
                    }
                    if group.encode_utf16().count() > 16 {
                        return Err("Buddy group names are limited to 16 UTF-16 code units.");
                    }
                    BuddyCommand::ChangeGroup {
                        target: target.to_owned(),
                        group,
                    }
                }
                "filter" => {
                    let filter = parts.collect::<Vec<_>>().join(" ");
                    if filter.is_empty() {
                        return Err("Usage: /buddy filter all|<group name>");
                    }
                    BuddyCommand::Filter(if filter.eq_ignore_ascii_case("all") {
                        None
                    } else {
                        Some(filter)
                    })
                }
                "remove" | "delete" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /buddy remove <character>");
                    };
                    BuddyCommand::Remove(name.to_owned())
                }
                "accept" => BuddyCommand::Accept,
                "decline" | "deny" | "dismiss" => BuddyCommand::Decline,
                "list" => BuddyCommand::List,
                _ => return Err("Usage: /buddy add|group|filter|remove|accept|dismiss|list"),
            };
            Ok(Some(ChatCommand::Buddy(command)))
        }
        "/alliance" => {
            let Some(action) = parts.next() else {
                return Err(
                    "Usage: /alliance list|invite <guild>|expel <guild>|leave|notice <text>|ranks <r1>|<r2>|<r3>|<r4>|<r5>|accept|deny",
                );
            };
            let command = match action.to_ascii_lowercase().as_str() {
                "invite" => {
                    let Some(guild) = parts.next() else {
                        return Err("Usage: /alliance invite <guild>");
                    };
                    AllianceCommand::Invite(guild.to_owned())
                }
                "expel" => {
                    let Some(guild) = parts.next() else {
                        return Err("Usage: /alliance expel <guild name or id>");
                    };
                    AllianceCommand::Expel(guild.to_owned())
                }
                "leave" => AllianceCommand::Leave,
                "notice" => {
                    let notice = parts.collect::<Vec<_>>().join(" ");
                    if notice.is_empty() {
                        return Err("Usage: /alliance notice <text>");
                    }
                    AllianceCommand::Notice(notice)
                }
                "ranks" => {
                    let rank_text = parts.collect::<Vec<_>>().join(" ");
                    let titles = rank_text.split('|').map(str::trim).collect::<Vec<_>>();
                    if titles.len() != 5 {
                        return Err(
                            "Usage: /alliance ranks <rank 1>|<rank 2>|<rank 3>|<rank 4>|<rank 5>",
                        );
                    }
                    AllianceCommand::RankTitles(std::array::from_fn(|index| {
                        titles[index].to_owned()
                    }))
                }
                "accept" => AllianceCommand::Accept,
                "deny" | "decline" => AllianceCommand::Deny,
                "list" => AllianceCommand::List,
                _ => {
                    return Err(
                        "Usage: /alliance list|invite <guild>|expel <guild>|leave|notice <text>|ranks <r1>|<r2>|<r3>|<r4>|<r5>|accept|deny",
                    );
                }
            };
            Ok(Some(ChatCommand::Alliance(command)))
        }
        "/guild" => {
            let Some(action) = parts.next() else {
                return Err(
                    "Usage: /guild list|create|invite|leave|expel|rank|ranks|emblem|notice",
                );
            };
            let command = match action.to_ascii_lowercase().as_str() {
                "list" => GuildCommand::List,
                "create" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /guild create <3-12 letter name>");
                    };
                    if !(3..=12).contains(&name.len())
                        || !name.bytes().all(|byte| byte.is_ascii_alphabetic())
                    {
                        return Err("Guild names must be 3–12 English letters.");
                    }
                    GuildCommand::Create(name.to_owned())
                }
                "invite" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /guild invite <character>");
                    };
                    GuildCommand::Invite(name.to_owned())
                }
                "leave" => GuildCommand::Leave,
                "expel" => {
                    let Some(name) = parts.next() else {
                        return Err("Usage: /guild expel <member>");
                    };
                    GuildCommand::Expel(name.to_owned())
                }
                "rank" => {
                    let Some(target) = parts.next() else {
                        return Err("Usage: /guild rank <member> <1-5>");
                    };
                    let Some(rank) = parts.next().and_then(|value| value.parse::<u8>().ok()) else {
                        return Err("Usage: /guild rank <member> <1-5>");
                    };
                    if !(1..=5).contains(&rank) {
                        return Err("Guild rank must be between 1 and 5.");
                    }
                    GuildCommand::MemberRank {
                        target: target.to_owned(),
                        rank,
                    }
                }
                "ranks" => {
                    let rank_text = parts.collect::<Vec<_>>().join(" ");
                    let titles = rank_text.split('|').map(str::trim).collect::<Vec<_>>();
                    if titles.len() != 5 {
                        return Err(
                            "Usage: /guild ranks <rank 1>|<rank 2>|<rank 3>|<rank 4>|<rank 5>",
                        );
                    }
                    GuildCommand::RankTitles(std::array::from_fn(|index| titles[index].to_owned()))
                }
                "emblem" => {
                    let Some(background) = parts.next().and_then(|value| value.parse().ok()) else {
                        return Err("Usage: /guild emblem <bg> <bg color> <logo> <logo color>");
                    };
                    let Some(background_color) = parts.next().and_then(|value| value.parse().ok())
                    else {
                        return Err("Usage: /guild emblem <bg> <bg color> <logo> <logo color>");
                    };
                    let Some(logo) = parts.next().and_then(|value| value.parse().ok()) else {
                        return Err("Usage: /guild emblem <bg> <bg color> <logo> <logo color>");
                    };
                    let Some(logo_color) = parts.next().and_then(|value| value.parse().ok()) else {
                        return Err("Usage: /guild emblem <bg> <bg color> <logo> <logo color>");
                    };
                    GuildCommand::Emblem {
                        background,
                        background_color,
                        logo,
                        logo_color,
                    }
                }
                "notice" => {
                    let notice = parts.collect::<Vec<_>>().join(" ");
                    if notice.is_empty() {
                        return Err("Usage: /guild notice <text>");
                    }
                    GuildCommand::Notice(notice)
                }
                _ => {
                    return Err(
                        "Usage: /guild list|create|invite|leave|expel|rank|ranks|emblem|notice",
                    );
                }
            };
            Ok(Some(ChatCommand::Guild(command)))
        }
        _ => Ok(None),
    }
}

fn public_chat_color(chat_type: i8, gm: bool) -> [u8; 3] {
    if gm {
        return [255, 255, 255];
    }
    match chat_type {
        2 => [255, 105, 105],
        3 => [132, 190, 255],
        4 => [255, 222, 112],
        5 => [255, 160, 215],
        6 => [150, 220, 255],
        7 => [135, 230, 150],
        8 => [255, 180, 90],
        9 => [185, 240, 170],
        _ => [238, 232, 214],
    }
}

struct ChatBalloonPart {
    width: usize,
    height: usize,
    origin_x: i32,
    origin_y: i32,
    pixels: Arc<[u8]>,
}

struct ChatBalloonSkin {
    parts: HashMap<&'static str, ChatBalloonPart>,
}

#[derive(Clone, Copy)]
struct DamageGlyph {
    uv: [f32; 4],
    width: f32,
    height: f32,
    origin_x: f32,
    origin_y: f32,
}

struct MapNpcRuntime {
    npc_id: i32,
    maple_tv_anchors: Option<MapleTvAnchors>,
    scripted: bool,
    x: f64,
    y: f64,
    foothold: u16,
    body: PhysicsBody,
    flipped: bool,
    animations: HashMap<NpcMotion, Vec<NpcFrameRuntime>>,
    animation_order: Vec<NpcMotion>,
    imitated_look_sprites: Option<Vec<(usize, SpriteRect)>>,
    random_state: u32,
    motion: NpcMotion,
    frame_index: usize,
    elapsed: Duration,
    quest_mark: NpcQuestMark,
    quest_mark_frames: Vec<NpcQuestMarkFrameRuntime>,
    quest_mark_frame_index: usize,
    quest_mark_elapsed: Duration,
}

#[derive(Clone, Copy)]
struct MapleTvAnchors {
    message_x: i16,
    message_y: i16,
    ad_x: i16,
    ad_y: i16,
}

struct MapleTvBroadcastRuntime {
    type_id: u8,
    has_partner: bool,
    expires_at: Instant,
    sender_avatar_draws: Vec<(usize, SpriteRect)>,
    partner_avatar_draws: Vec<(usize, SpriteRect)>,
}

struct MapleTvAnimation {
    frames: Vec<MapleTvAnimationFrame>,
    frame_index: usize,
    elapsed: Duration,
}

struct MapleTvAnimationFrame {
    texture: GpuTexture,
    delay: Duration,
    origin: (i16, i16),
}

impl MapleTvAnimation {
    fn empty() -> Self {
        Self {
            frames: Vec::new(),
            frame_index: 0,
            elapsed: Duration::ZERO,
        }
    }

    fn current(&self) -> Option<&MapleTvAnimationFrame> {
        self.frames.get(self.frame_index)
    }

    /// Advances a one-shot reel and returns true when it reaches its last frame.
    fn advance_once(&mut self, elapsed: Duration) -> bool {
        if self.frames.len() < 2 {
            return false;
        }
        self.elapsed = self.elapsed.saturating_add(elapsed);
        loop {
            let delay = self.frames[self.frame_index].delay;
            if self.elapsed < delay {
                return false;
            }
            self.elapsed = self.elapsed.saturating_sub(delay);
            if self.frame_index + 1 == self.frames.len() {
                self.frame_index = 0;
                self.elapsed = Duration::ZERO;
                return true;
            }
            self.frame_index += 1;
        }
    }

    fn advance_loop(&mut self, elapsed: Duration) {
        if self.frames.len() < 2 {
            return;
        }
        let cycle_nanos = self.frames.iter().fold(0_u128, |total, frame| {
            total.saturating_add(frame.delay.as_nanos())
        });
        if cycle_nanos == 0 {
            return;
        }
        let mut phase_nanos =
            self.elapsed.as_nanos().saturating_add(elapsed.as_nanos()) % cycle_nanos;
        self.frame_index = 0;
        for (index, frame) in self.frames.iter().enumerate() {
            let delay_nanos = frame.delay.as_nanos();
            if phase_nanos < delay_nanos {
                self.frame_index = index;
                break;
            }
            phase_nanos -= delay_nanos;
            self.frame_index = (index + 1) % self.frames.len();
        }
        self.elapsed = Duration::new(
            (phase_nanos / 1_000_000_000) as u64,
            (phase_nanos % 1_000_000_000) as u32,
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum NpcMotion {
    Stand,
    Move,
    Say,
    Other(String),
}

impl NpcMotion {
    fn from_state_name(name: String) -> Self {
        match name.as_str() {
            "stand" => Self::Stand,
            "move" => Self::Move,
            "say" => Self::Say,
            _ => Self::Other(name),
        }
    }

    fn is_walking(&self) -> bool {
        match self {
            Self::Move => true,
            Self::Other(name) => name == "walk",
            Self::Stand | Self::Say => false,
        }
    }

    fn state_name(&self) -> &str {
        match self {
            Self::Stand => "stand",
            Self::Move => "move",
            Self::Say => "say",
            Self::Other(name) => name,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum NpcQuestMark {
    #[default]
    None,
    Available,
    InProgress,
    Complete,
}

impl MapNpcRuntime {
    fn frames(&self) -> &[NpcFrameRuntime] {
        self.animations
            .get(&self.motion)
            .or_else(|| self.animations.get(&NpcMotion::Stand))
            .or_else(|| {
                self.animation_order
                    .first()
                    .and_then(|motion| self.animations.get(motion))
            })
            .map_or(&[], Vec::as_slice)
    }

    fn set_motion(&mut self, motion: NpcMotion) {
        if self.motion != motion {
            let walking = motion.is_walking();
            self.motion = motion;
            self.frame_index = 0;
            self.elapsed = Duration::ZERO;
            if walking {
                self.body.horizontal_speed = if self.flipped { 1.0 } else { -1.0 };
                self.body.flags.turn_at_edges = true;
            } else {
                self.body.horizontal_speed = 0.0;
                self.body.flags.turn_at_edges = false;
            }
        }
    }

    fn next_random_motion(&mut self) -> Option<NpcMotion> {
        if self.animation_order.is_empty() {
            return None;
        }
        // Keep the legacy random state changes deterministic per spawned NPC. This
        // avoids syncing or platform-specific RNG while still varying the stance.
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.random_state = value.max(1);
        self.animation_order
            .get(value as usize % self.animation_order.len())
            .cloned()
    }
}

struct NpcFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

struct MapHiredMerchantRuntime {
    object_id: i32,
    x: f64,
    y: f64,
    owner_name: String,
    description: String,
    skin: u8,
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
    sign: Option<(usize, SpriteRect)>,
    label: Option<(usize, SpriteRect)>,
    visitors: i8,
    visitor_capacity: i8,
}

struct NpcQuestMarkFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

struct NpcFrameAsset {
    motion: NpcMotion,
    delay_ms: u16,
    sprites: Range<usize>,
}

struct NpcQuestMarkFrameAsset {
    delay_ms: u16,
    sprites: Range<usize>,
}

#[derive(Clone, Copy)]
struct NpcTraits {
    scripted: bool,
}

type NpcInlineIcons = HashMap<char, (u32, u32, Arc<[u8]>)>;
static EMPTY_NPC_INLINE_ICONS: std::sync::LazyLock<NpcInlineIcons> =
    std::sync::LazyLock::new(NpcInlineIcons::new);
const UI_GLYPH_CACHE_ENTRIES: usize = 8192;
const UI_GLYPH_CACHE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct UiGlyphKey {
    character: char,
    size_bits: u32,
}

#[derive(Clone)]
struct UiGlyphRaster {
    metrics: fontdue::Metrics,
    coverage: Arc<[u8]>,
}

#[derive(Default)]
struct UiGlyphCache {
    values: HashMap<UiGlyphKey, UiGlyphRaster>,
    insertion_order: VecDeque<UiGlyphKey>,
    cached_bytes: usize,
}

static UI_GLYPH_CACHE: LazyLock<Mutex<UiGlyphCache>> =
    LazyLock::new(|| Mutex::new(UiGlyphCache::default()));

/// The client currently loads a single UI font. Reuse glyph coverage across the
/// many software-rendered panels, with fixed-size keys and bounded retained memory.
fn rasterize_ui_glyph(
    font: &fontdue::Font,
    character: char,
    size: f32,
) -> (fontdue::Metrics, Arc<[u8]>) {
    let key = UiGlyphKey {
        character,
        size_bits: size.to_bits(),
    };
    if let Some(glyph) = UI_GLYPH_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .values
        .get(&key)
        .cloned()
    {
        return (glyph.metrics, glyph.coverage);
    }

    let (metrics, coverage): (_, Vec<u8>) = font.rasterize(character, size);
    let coverage: Arc<[u8]> = coverage.into();
    if coverage.len() <= UI_GLYPH_CACHE_BYTES {
        let mut cache = UI_GLYPH_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(glyph) = cache.values.get(&key) {
            return (glyph.metrics, glyph.coverage.clone());
        }
        while cache.values.len() >= UI_GLYPH_CACHE_ENTRIES
            || cache.cached_bytes.saturating_add(coverage.len()) > UI_GLYPH_CACHE_BYTES
        {
            let Some(evicted) = cache.insertion_order.pop_front() else {
                break;
            };
            if let Some(glyph) = cache.values.remove(&evicted) {
                cache.cached_bytes = cache.cached_bytes.saturating_sub(glyph.coverage.len());
            }
        }
        if cache.values.len() < UI_GLYPH_CACHE_ENTRIES
            && cache.cached_bytes.saturating_add(coverage.len()) <= UI_GLYPH_CACHE_BYTES
        {
            cache.insertion_order.push_back(key);
            cache.cached_bytes += coverage.len();
            cache.values.insert(
                key,
                UiGlyphRaster {
                    metrics,
                    coverage: coverage.clone(),
                },
            );
        }
    }
    (metrics, coverage)
}

struct NpcDialogueUi {
    state: openstory_protocol::NpcDialogue,
    text: String,
    visible_text_chars: usize,
    reveal_elapsed: Duration,
    text_scroll: usize,
    text_max_scroll: usize,
    inline_icons: NpcInlineIcons,
    selections: Vec<(i32, String)>,
    selected_selection: usize,
    first_visible_selection: usize,
    input: String,
    texture: GpuTexture,
    panel: SpriteRect,
    buttons: Vec<NpcDialogueButton>,
}

struct NpcShopUi {
    state: openstory_protocol::NpcShop,
    mode: NpcShopMode,
    selected: usize,
    first_visible: usize,
    quantity: i16,
    notice: Option<String>,
    confirmation: Option<NpcShopConfirmation>,
    texture: GpuTexture,
}

struct StorageUi {
    state: openstory_protocol::StorageSnapshot,
    tab: openstory_protocol::InventoryType,
    storage_scroll: usize,
    inventory_scroll: usize,
    meso_input: String,
    meso_focused: bool,
    notice: Option<String>,
    close_when_transfer_finishes: bool,
}

struct HiredMerchantRoomUi {
    state: openstory_protocol::HiredMerchantRoom,
    selected_item: Option<usize>,
    item_scroll: usize,
    chat_input: String,
    chat_focused: bool,
    stock_source: Option<HiredMerchantStockSource>,
    stock_price: String,
    notice: Option<String>,
}

#[derive(Clone, Copy)]
struct HiredMerchantStockSource {
    inventory_type: openstory_protocol::InventoryType,
    slot: u16,
    item_id: i32,
    count: u16,
}

struct TradeRoomUi {
    self_slot: u8,
    partner: Option<openstory_protocol::TradeParticipant>,
    own_items: Vec<Option<TradeRoomItem>>,
    partner_items: Vec<Option<TradeRoomItem>>,
    own_meso: i32,
    partner_meso: i32,
    own_confirmed: bool,
    partner_confirmed: bool,
    chat_lines: VecDeque<String>,
    chat_input: String,
    chat_focused: bool,
    meso_input: String,
    meso_focused: bool,
    pending_item: Option<TradeStockSource>,
    quantity_input: String,
    notice: Option<String>,
}

#[derive(Clone)]
struct TradeRoomItem {
    item_id: i32,
    quantity: i16,
}

struct CharacterInfoUi {
    name: String,
    info: openstory_protocol::CharacterInfoReply,
    fame_pending: bool,
}

#[derive(Clone, Copy)]
struct TradeStockSource {
    inventory_type: openstory_protocol::InventoryType,
    inventory_slot: u16,
    trade_slot: u8,
    item_id: i32,
    max_quantity: i16,
}

struct PendingTradeInvite {
    inviter: String,
    room_id: i32,
}

struct OwlSearchUi {
    slot: i16,
    item_id: i32,
    query: String,
    top_item_ids: Vec<i32>,
    top_items_loaded: bool,
    results: Vec<openstory_protocol::OwlShopResult>,
    result_item_id: Option<i32>,
    selected_result: Option<usize>,
    result_offset: usize,
    notice: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InventoryDropStep {
    ConfirmUntradable,
    Quantity,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InventoryQuantityAction {
    Drop,
    StorageStore,
    StorageTake,
}

struct InventoryDropPrompt {
    action: InventoryQuantityAction,
    inventory_type: openstory_protocol::InventoryType,
    slot: u16,
    item_id: i32,
    max_quantity: u16,
    quantity: String,
    step: InventoryDropStep,
    notice: Option<String>,
}

struct PendingStorageRestore {
    inventory_type: openstory_protocol::InventoryType,
    item_id: i32,
    full_quantity: u16,
    keep_quantity: u16,
    before_counts: HashMap<u16, u16>,
}

#[derive(Clone, Copy)]
struct NpcShopConfirmation {
    mode: NpcShopMode,
    slot: i16,
    item_id: i32,
    quantity: i16,
    recharge: bool,
}

#[derive(Clone, Debug)]
struct CashShopProduct {
    sn: i32,
    item_id: i32,
    price: i32,
    count: u16,
    period: i32,
    gender: i32,
    priority: i32,
}

struct CashShopUi {
    entered: bool,
    view: CashShopView,
    category: usize,
    page: usize,
    selected_sn: Option<i32>,
    query: String,
    search_focused: bool,
    coupon_code: String,
    coupon_focused: bool,
    coupon_open: bool,
    coupon_pending: bool,
    currency: usize,
    balances: [i32; 3],
    locker: Vec<openstory_protocol::CashShopItem>,
    locker_slots: u16,
    character_slots: u16,
    pending_withdrawal: Option<i64>,
    confirmation: bool,
    notice: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CashShopView {
    #[default]
    Catalog,
    Locker,
}

struct GuildBbsUi {
    page: usize,
    total_threads: usize,
    notice: Option<openstory_protocol::BbsThread>,
    threads: Vec<openstory_protocol::BbsThread>,
    thread: Option<openstory_protocol::BbsThreadView>,
    draft: Option<GuildBbsDraft>,
    reply_input: String,
    reply_focused: bool,
    confirm_delete: bool,
    feedback: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GuildBbsDraftFocus {
    Title,
    Content,
}

struct GuildBbsDraft {
    title: String,
    content: String,
    is_notice: bool,
    focus: GuildBbsDraftFocus,
}

#[derive(Default)]
struct FamilyUiState {
    info: Option<openstory_protocol::FamilyInfo>,
    chart: Option<openstory_protocol::FamilyChart>,
    privileges: Vec<openstory_protocol::FamilyPrivilege>,
    selected_ability: usize,
    selected_member: Option<i32>,
    notice: Option<String>,
    input: Option<FamilyInput>,
    confirm_leave: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FamilyInputKind {
    InviteJunior,
    EditPrecepts,
}

struct FamilyInput {
    kind: FamilyInputKind,
    text: String,
    key_text_for_ime_dedup: Option<String>,
}

impl CashShopUi {
    fn new() -> Self {
        Self {
            entered: false,
            view: CashShopView::Catalog,
            category: 0,
            page: 0,
            selected_sn: None,
            query: String::new(),
            search_focused: false,
            coupon_code: String::new(),
            coupon_focused: false,
            coupon_open: false,
            coupon_pending: false,
            currency: 0,
            balances: [0; 3],
            locker: Vec::new(),
            locker_slots: 0,
            character_slots: 0,
            pending_withdrawal: None,
            confirmation: false,
            notice: Some("Connecting to the Cash Shop…".to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NpcShopMode {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum QuestLogTab {
    #[default]
    Active,
    Available,
    Completed,
}

#[derive(Clone, Copy)]
struct NpcDialogueButton {
    rect: SpriteRect,
    action: NpcDialogueAction,
}

#[derive(Clone, Copy)]
enum NpcDialogueAction {
    More(i8),
    Select(i32),
    SubmitText,
    SubmitNumber,
    AdjustNumber(i32),
}

struct MapDropRuntime {
    x: f64,
    y: f64,
    destination_x: f64,
    destination_y: f64,
    body: PhysicsBody,
    frames: Vec<DropFrameRuntime>,
    frame_index: usize,
    frame_elapsed: Duration,
    motion: DropMotion,
}

struct DropFrameRuntime {
    delay: Duration,
    sprite: (usize, SpriteRect),
}

struct DropFrameAsset {
    delay_ms: u16,
    sprite_index: usize,
}

enum DropMotion {
    Falling {
        elapsed: Duration,
    },
    Floating {
        phase: f64,
    },
    PickedUp {
        mode: i8,
        looter_id: Option<i32>,
        elapsed: Duration,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MobMotion {
    Move,
    Stand,
    Jump,
    Hit,
    Die,
    Attack(u8),
    Skill(u8),
}

fn mob_motion_for_stance(stance: u8) -> MobMotion {
    match stance & !1 {
        0 | 2 => MobMotion::Move,
        6 => MobMotion::Jump,
        8 => MobMotion::Hit,
        10 => MobMotion::Die,
        _ => MobMotion::Stand,
    }
}

fn mob_motion_for_action(action: i8) -> Option<MobMotion> {
    let action = u8::try_from(action).ok()?;
    match action {
        24..=41 => Some(MobMotion::Attack((action - 24) / 2 + 1)),
        42..=59 => Some(MobMotion::Skill((action - 42) / 2 + 1)),
        _ => None,
    }
}

fn first_mob_motion(
    animations: &HashMap<MobMotion, Vec<MobFrameRuntime>>,
    desired: MobMotion,
) -> Option<MobMotion> {
    if animations
        .get(&desired)
        .is_some_and(|frames| !frames.is_empty())
    {
        return Some(desired);
    }
    [
        MobMotion::Stand,
        MobMotion::Move,
        MobMotion::Jump,
        MobMotion::Hit,
    ]
    .into_iter()
    .find(|motion| {
        animations
            .get(motion)
            .is_some_and(|frames| !frames.is_empty())
    })
}

struct MobFrameRuntime {
    delay: Duration,
    sprites: Vec<(usize, SpriteRect)>,
}

struct MobFrameAsset {
    motion: MobMotion,
    delay_ms: u16,
    sprites: Range<usize>,
}

struct MapMobRuntime {
    mob_id: i32,
    x: f64,
    y: f64,
    body: PhysicsBody,
    foothold: u16,
    control_mode: i8,
    server_flip: bool,
    no_flip: bool,
    faces_right: bool,
    traits: MobTraits,
    motion: MobMotion,
    server_stance: u8,
    frame_index: usize,
    elapsed: Duration,
    animations: HashMap<MobMotion, Vec<MobFrameRuntime>>,
    skill_actions: HashMap<(u8, u8), (u8, i8)>,
    hit_effects: Vec<MapMobHitEffect>,
    dying: bool,
    fade_remaining: Option<Duration>,
    fade_in_remaining: Option<Duration>,
    hp_percent: Option<u8>,
    hp_bar_remaining: Duration,
    statuses: HashSet<(openstory_protocol::MobStatusMaskGroup, u32)>,
    control_acked: bool,
    ai_counter: u16,
    random_state: u32,
    fly_direction: i8,
}

struct MapSummonRuntime {
    owner_id: i32,
    skill_id: i32,
    skill_level: i8,
    x: f64,
    y: f64,
    body: PhysicsBody,
    stance: u8,
    movement_type: u8,
    attacks: bool,
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct MapDragonRuntime {
    x: f64,
    y: f64,
    body: PhysicsBody,
    flipped: bool,
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

struct MapPetRuntime {
    owner_id: i32,
    slot: u8,
    item_id: i32,
    unique_id: i32,
    name_label: Option<(usize, SpriteRect)>,
    x: f64,
    y: f64,
    body: PhysicsBody,
    loot_target: Option<(f64, f64)>,
    stance: u8,
    animations: HashMap<u8, Vec<ProjectileFrameRuntime>>,
    frame_index: usize,
    elapsed: Duration,
    command_remaining: Duration,
    interaction: Option<PetInteractionRuntime>,
    speech_bubble_remaining: Option<Duration>,
}

struct PetInteractionRuntime {
    frames: Vec<ProjectileFrameRuntime>,
    frame_index: usize,
    elapsed: Duration,
}

impl MapPetRuntime {
    fn state(&self) -> u8 {
        let state = self.stance / 2;
        if state < 8 { state } else { 1 }
    }

    fn frames(&self) -> &[ProjectileFrameRuntime] {
        if let Some(interaction) = self.interaction.as_ref() {
            return &interaction.frames;
        }
        self.animations
            .get(&self.state())
            .or_else(|| self.animations.get(&1))
            .map_or(&[], Vec::as_slice)
    }

    fn frame_index(&self) -> usize {
        self.interaction
            .as_ref()
            .map_or(self.frame_index, |interaction| interaction.frame_index)
    }

    fn set_stance(&mut self, stance: u8) {
        let previous_state = self.state();
        self.stance = stance;
        if self.state() != previous_state {
            self.frame_index = 0;
            self.elapsed = Duration::ZERO;
        }
    }

    fn set_local_motion(&mut self, state: u8, flipped: bool) {
        self.set_stance(state.saturating_mul(2) + u8::from(flipped));
    }

    fn play_interaction(&mut self, frames: Vec<ProjectileFrameRuntime>) -> bool {
        if frames.is_empty() {
            return false;
        }
        self.command_remaining = Duration::from_millis(8 * 240);
        self.body.horizontal_force = 0.0;
        self.body.vertical_force = 0.0;
        self.body.mode = PhysicsMode::Normal;
        self.body.flags.no_gravity = false;
        self.interaction = Some(PetInteractionRuntime {
            frames,
            frame_index: 0,
            elapsed: Duration::ZERO,
        });
        true
    }

    fn play_command(&mut self, animation: i8) {
        self.command_remaining = Duration::from_millis(8 * 240);
        self.interaction = None;
        self.body.horizontal_force = 0.0;
        self.body.vertical_force = 0.0;
        self.body.mode = PhysicsMode::Normal;
        self.body.flags.no_gravity = false;
        let state = if animation == 0 { 3 } else { 2 };
        self.set_local_motion(state, self.stance & 1 == 1);
    }

    fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
        self.body.previous_x = x;
        self.body.previous_y = y;
        self.body.x = x;
        self.body.y = y;
    }

    fn follow_local(&mut self, owner: (f64, f64), footholds: Option<&FootholdTree>) {
        const WALK_FORCE: f64 = 0.35;
        const FLY_FORCE: f64 = 0.2;

        if !self.command_remaining.is_zero() {
            self.body.horizontal_force = 0.0;
            self.body.vertical_force = 0.0;
            self.body.mode = PhysicsMode::Normal;
            self.body.flags.no_gravity = false;
            return;
        }

        let owner_delta_x = owner.0 - self.body.x;
        let owner_delta_y = owner.1 - self.body.y;
        let owner_distance = owner_delta_x.hypot(owner_delta_y);
        let current_state = self.state();
        self.body.horizontal_force = 0.0;
        self.body.vertical_force = 0.0;

        match current_state {
            0 | 1 => {
                if owner_distance > 150.0 {
                    self.set_position(owner.0, owner.1);
                    self.loot_target = None;
                } else {
                    let destination = self.loot_target.unwrap_or(owner);
                    let delta_x = destination.0 - self.body.x;
                    let delta_y = destination.1 - self.body.y;
                    let deadzone = if self.loot_target.is_some() {
                        8.0
                    } else {
                        50.0
                    };
                    if delta_x > deadzone {
                        self.body.horizontal_force = WALK_FORCE;
                        self.set_local_motion(0, true);
                    } else if delta_x < -deadzone {
                        self.body.horizontal_force = -WALK_FORCE;
                        self.set_local_motion(0, false);
                    } else {
                        self.set_local_motion(1, self.stance & 1 == 1);
                    }
                    if self.body.on_ground && -delta_y > 40.0 && delta_x.abs() < 70.0 {
                        self.body.vertical_speed = -5.5;
                    }
                    if !self.body.on_ground {
                        self.set_local_motion(2, self.stance & 1 == 1);
                    }
                }
                self.body.mode = PhysicsMode::Normal;
                self.body.flags.no_gravity = false;
            }
            2 => {
                if self.body.on_ground {
                    self.set_local_motion(1, self.stance & 1 == 1);
                } else if owner_distance > 150.0 {
                    self.set_position(owner.0, owner.1);
                } else if owner_delta_x > 50.0 {
                    self.body.horizontal_force = WALK_FORCE;
                    self.set_local_motion(2, true);
                } else if owner_delta_x < -50.0 {
                    self.body.horizontal_force = -WALK_FORCE;
                    self.set_local_motion(2, false);
                }
                self.body.mode = PhysicsMode::Normal;
                self.body.flags.no_gravity = false;
            }
            5 => {
                if owner_distance > 250.0 {
                    self.set_position(owner.0, owner.1);
                } else {
                    if owner_delta_x > 50.0 {
                        self.body.horizontal_force = FLY_FORCE;
                        self.set_local_motion(5, true);
                    } else if owner_delta_x < -50.0 {
                        self.body.horizontal_force = -FLY_FORCE;
                        self.set_local_motion(5, false);
                    }
                    if owner_delta_y > 50.0 {
                        self.body.vertical_force = FLY_FORCE;
                    } else if owner_delta_y < -50.0 {
                        self.body.vertical_force = -FLY_FORCE;
                    }
                }
                self.body.mode = PhysicsMode::Flying;
                self.body.flags.no_gravity = false;
            }
            6 => {
                self.set_position(owner.0, owner.1);
                self.body.horizontal_speed = 0.0;
                self.body.vertical_speed = 0.0;
                self.body.mode = PhysicsMode::Normal;
                self.body.flags.no_gravity = true;
            }
            _ => {
                self.body.mode = PhysicsMode::Normal;
                self.body.flags.no_gravity = false;
            }
        }

        if let Some(footholds) = footholds {
            PhysicsEngine.move_body(footholds, &mut self.body);
            self.x = self.body.x;
            self.y = self.body.y;
        }
    }

    fn advance(&mut self, updates: u8) {
        let elapsed = Duration::from_millis(8 * u64::from(updates));
        self.speech_bubble_remaining = self.speech_bubble_remaining.and_then(|remaining| {
            let remaining = remaining.saturating_sub(elapsed);
            (!remaining.is_zero()).then_some(remaining)
        });
        let was_commanding = !self.command_remaining.is_zero();
        self.command_remaining = self.command_remaining.saturating_sub(elapsed);
        if was_commanding && self.command_remaining.is_zero() {
            self.interaction = None;
            self.set_local_motion(1, self.stance & 1 == 1);
        }

        if let Some(interaction) = self.interaction.as_mut() {
            advance_pet_frame_sequence(
                &interaction.frames,
                &mut interaction.frame_index,
                &mut interaction.elapsed,
                elapsed,
            );
        } else {
            let frames = self
                .animations
                .get(&self.state())
                .or_else(|| self.animations.get(&1));
            if let Some(frames) = frames {
                advance_pet_frame_sequence(
                    frames,
                    &mut self.frame_index,
                    &mut self.elapsed,
                    elapsed,
                );
            }
        }
    }
}

fn advance_pet_frame_sequence(
    frames: &[ProjectileFrameRuntime],
    frame_index: &mut usize,
    elapsed: &mut Duration,
    delta: Duration,
) {
    if frames.len() < 2 {
        return;
    }
    *elapsed = elapsed.saturating_add(delta);
    while let Some(delay) = frames.get(*frame_index).map(|frame| frame.delay) {
        if *elapsed < delay {
            break;
        }
        *elapsed -= delay;
        *frame_index = (*frame_index + 1) % frames.len();
    }
}

#[derive(Clone, Copy)]
struct MobTraits {
    can_move: bool,
    can_jump: bool,
    can_fly: bool,
    body_attack: bool,
    boss: bool,
    speed: f64,
    fly_speed: f64,
    physical_attack: i32,
    level: i32,
    physical_defense: f64,
    magic_defense: f64,
    elemental_attributes: [u8; 6],
    avoid: f64,
}

#[derive(Clone, Copy)]
struct MobTouchAttack {
    object_id: i32,
    mob_id: i32,
    damage: i32,
    origin_x: f64,
}

#[derive(Clone, Copy)]
struct BasicAttackStats {
    weapon_type: i32,
    attack_type: openstory_protocol::AttackType,
    projectile_id: Option<i32>,
    attack_class: u8,
    speed: i8,
    level: i32,
    minimum_damage: f64,
    maximum_damage: f64,
    intelligence: i64,
    luck: i64,
    magic_total: i64,
    magic_damage: bool,
    element: Option<char>,
    fixed_damage: Option<i32>,
    accuracy: i32,
    critical: f64,
}

impl MapMobRuntime {
    fn is_frozen(&self) -> bool {
        self.statuses
            .contains(&(openstory_protocol::MobStatusMaskGroup::Second, 0x100))
    }

    fn is_immobilized(&self) -> bool {
        [0x80, 0x100, 0x20000].into_iter().any(|flag| {
            self.statuses
                .contains(&(openstory_protocol::MobStatusMaskGroup::Second, flag))
        })
    }

    fn status_tint(&self) -> [f32; 3] {
        let group = openstory_protocol::MobStatusMaskGroup::Second;
        if self.statuses.contains(&(group, 0x100)) {
            [0.5, 0.7, 1.0]
        } else if self.statuses.contains(&(group, 0x200)) {
            [0.5, 1.0, 0.5]
        } else if self.statuses.contains(&(group, 0x80)) {
            [1.0, 1.0, 0.5]
        } else {
            [1.0, 1.0, 1.0]
        }
    }

    fn random(&mut self) -> u32 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.random_state = value.max(1);
        self.random_state
    }

    fn choose_next_move(&mut self) {
        if !self.traits.can_move {
            self.set_motion(MobMotion::Stand);
        } else {
            match self.motion {
                MobMotion::Hit | MobMotion::Stand => {
                    self.set_motion(MobMotion::Move);
                    self.server_flip = self.random() & 1 != 0;
                }
                MobMotion::Move | MobMotion::Jump => {
                    if self.traits.can_jump && self.body.on_ground && self.random() % 4 == 0 {
                        self.set_motion(MobMotion::Jump);
                    } else {
                        match self.random() % 3 {
                            0 => self.set_motion(MobMotion::Stand),
                            1 => {
                                self.set_motion(MobMotion::Move);
                                self.server_flip = false;
                            }
                            _ => {
                                self.set_motion(MobMotion::Move);
                                self.server_flip = true;
                            }
                        }
                    }
                }
                MobMotion::Attack(_) | MobMotion::Skill(_) => {
                    self.set_motion(MobMotion::Stand);
                }
                MobMotion::Die => {}
            }
            if self.motion == MobMotion::Move && self.traits.can_fly {
                self.fly_direction = (self.random() % 3) as i8 - 1;
            }
        }
        self.refresh_server_stance();
    }

    fn set_motion(&mut self, desired: MobMotion) {
        let Some(motion) = first_mob_motion(&self.animations, desired) else {
            return;
        };
        if self.motion != motion {
            self.motion = motion;
            self.frame_index = 0;
            self.elapsed = Duration::ZERO;
        }
    }

    fn refresh_server_stance(&mut self) {
        let stance = match self.motion {
            MobMotion::Move => 2,
            MobMotion::Stand => 4,
            MobMotion::Jump => 6,
            MobMotion::Hit => 8,
            MobMotion::Die => 10,
            MobMotion::Attack(_) | MobMotion::Skill(_) => 4,
        };
        self.server_stance = stance + u8::from(!self.server_flip);
    }

    fn set_action_motion(&mut self, desired: MobMotion) -> bool {
        if !self
            .animations
            .get(&desired)
            .is_some_and(|frames| !frames.is_empty())
        {
            return false;
        }
        self.motion = desired;
        self.frame_index = 0;
        self.elapsed = Duration::ZERO;
        true
    }
}

fn advance_mob_animation(mob: &mut MapMobRuntime, elapsed: Duration) -> bool {
    let Some(frame_count) = mob.animations.get(&mob.motion).map(Vec::len) else {
        return false;
    };
    if frame_count == 0 {
        return false;
    }
    mob.elapsed = mob.elapsed.saturating_add(elapsed);
    let mut cycle_ended = false;
    loop {
        let Some(delay) = mob
            .animations
            .get(&mob.motion)
            .and_then(|frames| frames.get(mob.frame_index))
            .map(|frame| frame.delay)
        else {
            return cycle_ended;
        };
        if mob.elapsed < delay {
            break;
        }
        mob.elapsed -= delay;
        if mob.frame_index + 1 == frame_count {
            if mob.dying && mob.motion == MobMotion::Die {
                return true;
            }
            if matches!(mob.motion, MobMotion::Attack(_) | MobMotion::Skill(_)) {
                let desired = if mob.is_immobilized() {
                    MobMotion::Stand
                } else {
                    mob_motion_for_stance(mob.server_stance)
                };
                mob.set_motion(desired);
                return true;
            }
            mob.frame_index = 0;
            cycle_ended = true;
        } else {
            mob.frame_index += 1;
        }
    }
    cycle_ended
}

fn mob_movement_packet(object_id: i32, mob: &MapMobRuntime, nibble: i8) -> Vec<u8> {
    openstory_protocol::move_mob(
        object_id,
        nibble,
        openstory_protocol::PlayerMovement {
            x: rounded_i16(mob.body.x),
            y: rounded_i16(mob.body.y),
            last_x: rounded_i16(mob.body.previous_x),
            last_y: rounded_i16(mob.body.previous_y),
            foothold: mob.body.foothold_id,
            stance: mob.server_stance,
            duration: 1,
        },
    )
}

struct RemotePlayerState {
    name: String,
    guild_name: String,
    _guild_mark: openstory_protocol::GuildMark,
    level: u8,
    job: u16,
    look: openstory_protocol::CharacterLook,
    x: f64,
    y: f64,
    recoil_offset_x: f64,
    recoil_velocity_x: f64,
    target_x: f64,
    target_y: f64,
    previous_x: f64,
    previous_y: f64,
    foothold: u16,
    stance: u8,
    dead: bool,
    death_art: Option<LocalDeathArtRuntime>,
    facing_left: bool,
    hidden: bool,
    mount: Option<MountLookRuntime>,
    attack_speed: i8,
    attack_stance: Option<u8>,
    speech_bubble_remaining: Option<Duration>,
    chalkboard: Option<String>,
    invincibility_remaining: Duration,
    mini_room: Option<openstory_protocol::CharacterMiniRoom>,
    face_expression: Option<FaceExpressionRuntime>,
    item_aura: Option<PersistentItemAuraRuntime>,
    avatar_frames: Vec<AvatarFrameRuntime>,
    active_avatar_frame: Option<usize>,
    desired_avatar_motion: AvatarMotion,
    avatar_elapsed: Duration,
    name_label: Option<(usize, SpriteRect)>,
    guild_label: Option<(usize, SpriteRect)>,
}

impl RemotePlayerState {
    fn snap_to_ground_if_needed(&mut self, footholds: Option<&FootholdTree>) {
        let Some(footholds) = footholds else {
            return;
        };
        let grounded_here = footholds
            .get(self.foothold)
            .and_then(|foothold| foothold.ground_y(self.target_x))
            .is_some_and(|ground_y| (ground_y - self.target_y).abs() <= 2.0);
        if grounded_here {
            return;
        }
        let Some((foothold, ground_y)) = footholds.find_below(self.target_x, self.target_y) else {
            return;
        };
        self.foothold = foothold;
        self.target_y = ground_y;
        self.x = self.target_x;
        self.y = ground_y;
        self.recoil_velocity_x = 0.0;
    }
}

impl MapSceneRuntime {
    fn set_player_traction(&mut self, traction: f64) {
        self.player.traction = traction;
    }

    fn spawn_door(&mut self, owner_id: i32, x: i16, y: i16) {
        self.doors.insert(
            owner_id,
            MapDoorRuntime {
                x: f64::from(x),
                y: f64::from(y),
                frame_index: 0,
                elapsed: Duration::ZERO,
            },
        );
    }

    fn remove_door(&mut self, owner_id: i32) {
        self.doors.remove(&owner_id);
    }

    fn new(scene: MapSceneData, draws: &mut [(usize, SpriteRect)]) -> Self {
        let avatar_frames = avatar_frames_runtime(scene.avatar_frames, draws);
        let portal_frame_indices = scene
            .regular_portal_frames
            .iter()
            .chain(&scene.hidden_portal_frames)
            .chain(&scene.door_frames)
            .flat_map(|frame| frame.sprites.clone())
            .collect();
        let regular_portal_frames = scene
            .regular_portal_frames
            .iter()
            .map(|frame| portal_frame_runtime(frame, draws))
            .collect();
        let hidden_portal_frames = scene
            .hidden_portal_frames
            .iter()
            .map(|frame| portal_frame_runtime(frame, draws))
            .collect();
        let door_frames = scene
            .door_frames
            .iter()
            .map(|frame| portal_frame_runtime(frame, draws))
            .collect();
        let portals = scene
            .portals
            .into_iter()
            .map(|portal| MapPortalRuntime {
                portal_type: portal.portal_type,
                name: portal.name,
                target_name: portal.target_name,
                target_map_id: portal.target_map_id,
                x: portal.x,
                y: portal.y,
                touched: false,
            })
            .collect();
        let avatar_indices = avatar_frames
            .iter()
            .flat_map(|frame| frame.sprites.iter().filter_map(|sprite| sprite.draw_index))
            .collect();
        let active_avatar_frame = avatar_frames
            .iter()
            .position(|frame| frame.motion == AvatarMotion::Stand)
            .or_else(|| (!avatar_frames.is_empty()).then_some(0));
        let minimap_presentation = scene
            .minimap
            .as_ref()
            .map_or(minimap::Presentation::Collapsed, |_| {
                minimap::Presentation::Normal
            });
        let (map_art_animations, map_art_animation_draw_indices, active_map_art_draw_indices) =
            prepare_map_art_animations(scene.map_art_animations);
        let map_backgrounds = scene
            .map_backgrounds
            .into_iter()
            .map(|background| MapBackgroundRuntime {
                sprites: background.sprites,
                environment_name: background.environment_name,
                active: true,
                horizontal_tiled: background.horizontal_tiled,
                vertical_tiled: background.vertical_tiled,
                tile_width: background.tile_width,
                tile_height: background.tile_height,
                tile_count_x: background.tile_count_x,
                tile_count_y: background.tile_count_y,
                horizontal_mobile: background.horizontal_mobile,
                vertical_mobile: background.vertical_mobile,
                horizontal_speed: background.horizontal_speed,
                vertical_speed: background.vertical_speed,
                horizontal_parallax: background.horizontal_parallax,
                vertical_parallax: background.vertical_parallax,
                horizontal_offset: 0.0,
                vertical_offset: 0.0,
            })
            .collect();
        let runtime = Self {
            map_id: scene.map_id,
            underwater: scene.underwater,
            camera_offset: scene.camera_offset,
            camera_bounds: scene.camera_bounds,
            minimap: scene.minimap,
            minimap_presentation,
            footholds: scene.footholds,
            map_seats: scene.seats,
            map_ladders: scene.ladders,
            local_ladder: None,
            climb_cooldown: 0,
            player: PhysicsBody::new(scene.spawn.0, scene.spawn.1),
            world_draw_indices: scene.world_draw_indices,
            map_backgrounds,
            map_art_animations,
            map_art_animation_draw_indices,
            active_map_art_draw_indices,
            inactive_environment_draw_indices: HashSet::new(),
            avatar_frames,
            avatar_indices,
            active_avatar_frame,
            local_death_art: None,
            local_mount: None,
            desired_avatar_motion: AvatarMotion::Stand,
            attack_stance: None,
            attack_count: 0,
            attack_random_state: ((scene.spawn.0 as i64 as u32).rotate_left(13)
                ^ scene.spawn.1 as i64 as u32
                ^ 0xC0FF_EE11)
                .max(1),
            avatar_elapsed: Duration::ZERO,
            facing_right: true,
            last_movement: None,
            remote_players: HashMap::new(),
            local_item_aura: None,
            local_face_expression: None,
            local_speech_bubble_remaining: None,
            local_chalkboard: None,
            npcs: HashMap::new(),
            hired_merchants: HashMap::new(),
            scriptable_npc_ids: HashSet::new(),
            drops: HashMap::new(),
            mobs: HashMap::new(),
            mists: HashMap::new(),
            summons: HashMap::new(),
            dragons: HashMap::new(),
            pets: HashMap::new(),
            pet_loot_cooldown: 0,
            character_effects: Vec::with_capacity(8),
            screen_effect: None,
            reactors: HashMap::new(),
            doors: HashMap::new(),
            portals,
            regular_portal_frames,
            hidden_portal_frames,
            door_frames,
            portal_frame_indices,
            regular_portal_frame_index: 0,
            regular_portal_elapsed: Duration::ZERO,
            hidden_portal_frame_index: 0,
            hidden_portal_elapsed: Duration::ZERO,
            portal_cooldown: 0,
            teleport_cooldown: 0,
            touch_damage_cooldown: Duration::ZERO,
            local_invincibility_remaining: Duration::ZERO,
            idle_regen_elapsed: Duration::ZERO,
            regen_hit_lockout_remaining: Duration::ZERO,
            local_chair: None,
            local_map_seat: None,
            damage_numbers: Vec::with_capacity(64),
            pending_mob_attack_impacts: Vec::with_capacity(64),
            quest_clear_effect: None,
            projectiles: Vec::with_capacity(8),
            layer_insert_indices: scene.layer_insert_indices,
            foreground_start_index: scene.foreground_start_index,
        };
        runtime.refresh_avatar_draws(draws);
        runtime
    }

    fn is_locally_seated(&self) -> bool {
        self.local_chair.is_some() || self.local_map_seat.is_some()
    }

    fn update_camera(&mut self) -> (f32, f32) {
        let old = self.camera_offset;
        if let Some(bounds) = self.camera_bounds {
            self.camera_offset = bounds.offset_for(self.player.x, self.player.y);
        } else {
            self.camera_offset = (
                512.0 - self.player.x as f32,
                GAMEPLAY_CAMERA_CENTER_Y - self.player.y as f32,
            );
        }
        (self.camera_offset.0 - old.0, self.camera_offset.1 - old.1)
    }

    fn advance_climb_cooldown(&mut self, updates: u8) {
        self.climb_cooldown = self.climb_cooldown.saturating_sub(updates);
    }

    fn sit_on_nearby_map_seat(&mut self, draws: &mut [(usize, SpriteRect)]) -> bool {
        if self.is_locally_seated() || self.local_ladder.is_some() || self.attack_stance.is_some() {
            return false;
        }
        let Some((x, y)) = nearby_map_seat(&self.map_seats, &self.player) else {
            return false;
        };
        self.local_map_seat = Some((x, y));
        place_player_on_map_seat((x, y), &mut self.player, self.footholds.as_ref());
        self.last_movement = None;
        self.desired_avatar_motion = AvatarMotion::Sit;
        self.active_avatar_frame = first_avatar_frame(&self.avatar_frames, AvatarMotion::Sit);
        self.avatar_elapsed = Duration::ZERO;
        self.refresh_avatar_draws(draws);
        true
    }

    fn advance_map_art_animations(&mut self, updates: u8) {
        if updates == 0 {
            return;
        }
        let elapsed = Duration::from_millis(8 * u64::from(updates));
        for animation in &mut self.map_art_animations {
            if animation.environment_name.as_ref().is_some_and(|name| {
                self.map_backgrounds.iter().any(|background| {
                    background.environment_name.as_ref() == Some(name) && !background.active
                })
            }) {
                continue;
            }
            animation.elapsed = animation.elapsed.saturating_add(elapsed);
            let mut changed = false;
            loop {
                if animation.hold_last && animation.frame_index + 1 == animation.frames.len() {
                    animation.elapsed = Duration::ZERO;
                    break;
                }
                let Some(frame) = animation.frames.get(animation.frame_index) else {
                    break;
                };
                let delay = Duration::from_millis(u64::from(frame.delay_ms.max(1)));
                if animation.elapsed < delay {
                    break;
                }
                animation.elapsed -= delay;
                animation.frame_index = if animation.frame_index + 1 == animation.frames.len() {
                    0
                } else {
                    animation.frame_index + 1
                };
                changed = true;
            }
            if changed {
                if let Some(frame) = animation.frames.get(animation.frame_index) {
                    for sprite_index in animation
                        .frames
                        .iter()
                        .flat_map(|frame| frame.sprites.clone())
                    {
                        self.active_map_art_draw_indices.remove(&sprite_index);
                    }
                    self.active_map_art_draw_indices
                        .extend(frame.sprites.clone());
                }
            }
        }
    }

    fn toggle_environment(&mut self, name: &str, mode: i32) {
        let active = mode != 0;
        let sprite_ranges = self
            .map_backgrounds
            .iter_mut()
            .filter(|background| background.environment_name.as_deref() == Some(name))
            .map(|background| {
                background.active = active;
                background.sprites.clone()
            })
            .collect::<Vec<_>>();
        if sprite_ranges.is_empty() {
            return;
        }
        for range in sprite_ranges {
            for index in range {
                if active {
                    self.inactive_environment_draw_indices.remove(&index);
                } else {
                    self.inactive_environment_draw_indices.insert(index);
                }
            }
        }
        for animation in &mut self.map_art_animations {
            if animation.environment_name.as_deref() != Some(name) {
                continue;
            }
            for sprite_index in animation
                .frames
                .iter()
                .flat_map(|frame| frame.sprites.clone())
            {
                self.active_map_art_draw_indices.remove(&sprite_index);
            }
            animation.elapsed = Duration::ZERO;
            animation.frame_index = 0;
            if active {
                if let Some(frame) = animation.frames.first() {
                    self.active_map_art_draw_indices
                        .extend(frame.sprites.clone());
                }
            }
        }
    }

    fn advance_portals(
        &mut self,
        up_pressed: bool,
        can_use_portal: bool,
        updates: u8,
    ) -> Option<PortalWarp> {
        for _ in 0..updates {
            advance_portal_animation(
                &self.regular_portal_frames,
                &mut self.regular_portal_frame_index,
                &mut self.regular_portal_elapsed,
            );
            advance_portal_animation(
                &self.hidden_portal_frames,
                &mut self.hidden_portal_frame_index,
                &mut self.hidden_portal_elapsed,
            );
            for door in self.doors.values_mut() {
                advance_portal_animation(&self.door_frames, &mut door.frame_index, &mut door.elapsed);
            }
            self.portal_cooldown = self.portal_cooldown.saturating_sub(1);
            self.teleport_cooldown = self.teleport_cooldown.saturating_sub(1);
            for portal in &mut self.portals {
                portal.touched = matches!(portal.portal_type, 3 | 10)
                    && portal_bounds_contains(portal, self.player.x, self.player.y);
            }
            if !up_pressed || !can_use_portal || self.portal_cooldown != 0 {
                continue;
            }
            let Some(portal) = self
                .portals
                .iter()
                .find(|portal| portal_bounds_contains(portal, self.player.x, self.player.y))
            else {
                continue;
            };
            if portal.target_map_id >= 999_999_999 {
                continue;
            }
            if portal.target_map_id == self.map_id {
                let Some(target) = self
                    .portals
                    .iter()
                    .find(|target| target.name == portal.target_name)
                else {
                    continue;
                };
                self.portal_cooldown = 48;
                return Some(PortalWarp::SameMap {
                    x: target.x,
                    y: target.y - 30.0,
                });
            }
            self.portal_cooldown = 48;
            return Some(PortalWarp::ChangeMap {
                map_id: portal.target_map_id,
                portal_name: portal.name.clone(),
            });
        }
        None
    }

    fn apply_use_movement(
        &mut self,
        skill_id: i32,
        teleport_range: i16,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
    ) -> bool {
        if skill_id == 4_111_006 {
            if self.player.on_ground {
                return false;
            }
            let direction = if left {
                -1.0
            } else if right {
                1.0
            } else if self.facing_right {
                1.0
            } else {
                -1.0
            };
            self.player.horizontal_speed = direction * 7.5;
            if self.player.vertical_speed > 0.0 {
                self.player.vertical_speed = -2.0;
            }
            return true;
        }
        if !is_teleport_skill(skill_id) || self.teleport_cooldown > 0 {
            return false;
        }
        let target = {
            let Some(footholds) = self.footholds.as_ref() else {
                return false;
            };
            let current_x = rounded_i16(self.player.x);
            let current_y = rounded_i16(self.player.y);
            let border = footholds.bounds().bottom;
            if up || down {
                let ground = if up {
                    footholds
                        .find_above(f64::from(current_x), f64::from(current_y.saturating_sub(1)))
                        .map(|(_, ground)| ground)
                } else {
                    footholds
                        .find_below(f64::from(current_x), f64::from(current_y.saturating_add(3)))
                        .map(|(_, ground)| ground)
                };
                let Some(ground) = ground else {
                    return false;
                };
                let target_y = (ground
                    .trunc()
                    .clamp(f64::from(i16::MIN), f64::from(i16::MAX))
                    as i16)
                    .saturating_sub(1);
                let delta_y = if up {
                    i32::from(current_y) - i32::from(target_y)
                } else {
                    i32::from(target_y) - i32::from(current_y)
                };
                (target_y < border && delta_y > 0 && delta_y <= i32::from(teleport_range))
                    .then_some((current_x, target_y))
            } else {
                let direction = if left {
                    -1_i32
                } else if right {
                    1_i32
                } else if self.facing_right {
                    1_i32
                } else {
                    -1_i32
                };
                let bounds = footholds.bounds();
                let base_ground = legacy_y_below(footholds, current_x, current_y.saturating_sub(6));
                let mut target_x = current_x;
                let mut target_y = base_ground;
                let mut step = 8_i32;
                while step <= i32::from(teleport_range) {
                    let candidate_x = i32::from(current_x) + direction * step;
                    let candidate_x =
                        candidate_x.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
                    if candidate_x <= bounds.left_wall || candidate_x >= bounds.right_wall {
                        break;
                    }
                    let ground =
                        legacy_y_below(footholds, candidate_x, current_y.saturating_sub(6));
                    if ground >= border || (i32::from(ground) - i32::from(base_ground)).abs() > 35 {
                        break;
                    }
                    target_x = candidate_x;
                    target_y = ground;
                    step += 8;
                }
                (target_x != current_x || target_y != current_y).then_some((target_x, target_y))
            }
        };
        let Some((x, y)) = target else {
            return false;
        };
        if x == rounded_i16(self.player.x) && y == rounded_i16(self.player.y) {
            return false;
        }
        self.respawn_at_portal(f64::from(x), f64::from(y));
        self.teleport_cooldown = 44;
        true
    }

    fn respawn_at_portal(&mut self, x: f64, spawn_y: f64) {
        self.local_ladder = None;
        self.climb_cooldown = 0;
        let (y, foothold_id) = if let Some(footholds) = self.footholds.as_ref() {
            footholds
                .find_below(x, spawn_y)
                .map(|(id, ground_y)| (ground_y - 1.0, id))
                .unwrap_or_else(|| {
                    (
                        f64::from(footholds.y_below(rounded_i16(x), rounded_i16(spawn_y))),
                        0,
                    )
                })
        } else {
            (spawn_y, 0)
        };
        self.player.x = x;
        self.player.y = y;
        self.player.previous_x = x;
        self.player.previous_y = y;
        self.player.horizontal_speed = 0.0;
        self.player.vertical_speed = 0.0;
        self.player.foothold_id = foothold_id;
        self.player.foothold_slope = self
            .footholds
            .as_ref()
            .and_then(|footholds| footholds.get(foothold_id))
            .map_or(0.0, |foothold| foothold.slope());
        self.player.ground_below = y + 1.0;
        self.player.on_ground = foothold_id != 0;
        self.player.mode = PhysicsMode::Normal;
        self.last_movement = None;
    }

    fn set_facing(&mut self, right: bool, draws: &mut [(usize, SpriteRect)]) {
        if self.facing_right == right {
            return;
        }
        self.facing_right = right;
        self.refresh_avatar_draws(draws);
    }

    fn start_attack(&mut self, stance: u8, draws: &mut [(usize, SpriteRect)]) {
        self.attack_stance = Some(stance);
        self.attack_count = self.attack_count.wrapping_add(1);
        self.desired_avatar_motion = AvatarMotion::Attack(stance);
        self.active_avatar_frame =
            first_avatar_frame(&self.avatar_frames, self.desired_avatar_motion);
        self.avatar_elapsed = Duration::ZERO;
        self.refresh_avatar_draws(draws);
    }

    fn next_attack_random(&mut self) -> f64 {
        let mut value = self.attack_random_state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.attack_random_state = value.max(1);
        f64::from(self.attack_random_state) / f64::from(u32::MAX)
    }

    fn advance_avatar_animation(
        &mut self,
        elapsed: Duration,
        desired: AvatarMotion,
        fallback: AvatarMotion,
        draws: &mut [(usize, SpriteRect)],
    ) {
        if self.avatar_frames.is_empty() {
            return;
        }
        if self.desired_avatar_motion != desired || self.active_avatar_frame.is_none() {
            self.desired_avatar_motion = desired;
            self.active_avatar_frame = self.first_frame_for_motion(desired);
            self.avatar_elapsed = Duration::ZERO;
            self.refresh_avatar_draws(draws);
            return;
        }

        self.avatar_elapsed = self.avatar_elapsed.saturating_add(elapsed);
        let mut changed = false;
        loop {
            let Some(active) = self.active_avatar_frame else {
                break;
            };
            let delay = self.avatar_frames[active].delay;
            if self.avatar_elapsed < delay {
                break;
            }
            self.avatar_elapsed -= delay;
            let motion = self.avatar_frames[active].motion;
            let next = if matches!(motion, AvatarMotion::Attack(_)) {
                (active + 1..self.avatar_frames.len())
                    .find(|candidate| self.avatar_frames[*candidate].motion == motion)
            } else {
                (1..=self.avatar_frames.len())
                    .map(|offset| (active + offset) % self.avatar_frames.len())
                    .find(|candidate| self.avatar_frames[*candidate].motion == motion)
            };
            let Some(next) = next else {
                if matches!(motion, AvatarMotion::Attack(_)) {
                    self.attack_stance = None;
                    self.desired_avatar_motion = fallback;
                    self.active_avatar_frame = self.first_frame_for_motion(fallback);
                    self.avatar_elapsed = Duration::ZERO;
                    changed = true;
                }
                break;
            };
            self.active_avatar_frame = Some(next);
            changed = true;
        }
        if changed {
            self.refresh_avatar_draws(draws);
        }
    }

    fn advance_local_death_art(&mut self, updates: u8) {
        if let Some(art) = self.local_death_art.as_mut() {
            art.advance(updates);
        }
    }

    fn apply_remote_movement(&mut self, movement: openstory_protocol::RemoteCharacterMovement) {
        let footholds = self.footholds.as_ref();
        let Some(player) = self.remote_players.get_mut(&movement.id) else {
            return;
        };
        for fragment in movement.fragments {
            if fragment.kind == openstory_protocol::MovementKind::None {
                continue;
            }
            player.previous_x = player.target_x;
            player.previous_y = player.target_y;
            player.target_x = f64::from(fragment.x);
            player.target_y = f64::from(fragment.y);
            let dx = player.target_x - player.x;
            let dy = player.target_y - player.y;
            if fragment.kind == openstory_protocol::MovementKind::Teleport
                || dx * dx + dy * dy > 200.0 * 200.0
            {
                player.x = player.target_x;
                player.y = player.target_y;
                player.recoil_offset_x = 0.0;
                player.recoil_velocity_x = 0.0;
            }
            if fragment.foothold != 0 {
                player.foothold = fragment.foothold;
            }
            let was_dead = player.dead;
            player.dead = fragment.stance & !1 == 18;
            if player.dead && !was_dead {
                player.attack_stance = None;
                if let Some(art) = player.death_art.as_mut() {
                    art.reset();
                }
            }
            player.stance = fragment.stance;
            player.facing_left = fragment.stance & 1 != 0;
            player.attack_stance = None;
            if player.dead {
                player.snap_to_ground_if_needed(footholds);
            }
        }
    }

    fn apply_remote_attack(
        &mut self,
        attack: openstory_protocol::RemoteCharacterAttack,
        projectile_frames: Option<Vec<ProjectileFrameRuntime>>,
        skill_hit_effects: Vec<SkillHitEffectAnimation>,
        hit_delays: Vec<Duration>,
    ) {
        let hit_effect_flip = attack.to_left;
        let skill_hit_effects = Arc::new(skill_hit_effects);
        let source_position = self
            .remote_players
            .get(&attack.attacker_id)
            .map(|player| (player.x, player.y));
        if let Some(player) = self.remote_players.get_mut(&attack.attacker_id) {
            player.facing_left = attack.to_left;
            player.attack_speed = attack.speed;
            player.attack_stance = Some(attack.stance);
            player.desired_avatar_motion = AvatarMotion::Attack(attack.stance);
            player.active_avatar_frame =
                first_avatar_frame(&player.avatar_frames, player.desired_avatar_motion);
            player.avatar_elapsed = Duration::ZERO;
        }
        if let (Some(frames), Some((source_x, source_y))) = (projectile_frames, source_position) {
            let targets = attack
                .targets
                .iter()
                .filter_map(|target| {
                    self.mobs
                        .get(&target.object_id)
                        .map(|mob| (mob.x, mob.y - 24.0))
                })
                .take(15)
                .collect::<Vec<_>>();
            if targets.is_empty() {
                self.launch_projectile_from(
                    frames,
                    source_x,
                    source_y,
                    source_x + if attack.to_left { -400.0 } else { 400.0 },
                    source_y - 26.0,
                );
            } else {
                for (target_x, target_y) in targets {
                    self.launch_projectile_from(
                        frames.clone(),
                        source_x,
                        source_y,
                        target_x,
                        target_y,
                    );
                }
            }
        }
        for target in attack.targets {
            let mut stack_offset = 0.0;
            for (hit_index, damage) in target.damage_lines.into_iter().enumerate() {
                let mob_damage = openstory_protocol::MobDamage {
                    object_id: target.object_id,
                    damage: damage.damage,
                };
                self.schedule_mob_attack_impact(
                    target.object_id,
                    Arc::clone(&skill_hit_effects),
                    hit_effect_flip,
                    Some((mob_damage, damage.critical, stack_offset)),
                    hit_delays.get(hit_index).copied().unwrap_or_default(),
                );
                stack_offset += if damage.critical { 36.0 } else { 30.0 };
            }
        }
    }

    fn advance_background_positions(
        &mut self,
        camera_delta_x: f32,
        camera_delta_y: f32,
        updates: u8,
        draws: &mut [(usize, SpriteRect)],
    ) {
        let elapsed_updates = f32::from(updates);
        for background in &mut self.map_backgrounds {
            let raw_delta_x = if background.horizontal_mobile {
                camera_delta_x
                    + if background.active {
                        background.horizontal_speed * elapsed_updates
                    } else {
                        0.0
                    }
            } else {
                camera_delta_x * background.horizontal_parallax / 100.0
            };
            let raw_delta_y = if background.vertical_mobile {
                camera_delta_y
                    + if background.active {
                        background.vertical_speed * elapsed_updates
                    } else {
                        0.0
                    }
            } else {
                camera_delta_y * background.vertical_parallax / 100.0
            };
            let delta_x = if background.horizontal_tiled {
                let interval = background.tile_width.max(1.0);
                let offset = (background.horizontal_offset + raw_delta_x).rem_euclid(interval);
                let delta = offset - background.horizontal_offset;
                background.horizontal_offset = offset;
                delta
            } else {
                background.horizontal_offset += raw_delta_x;
                raw_delta_x
            };
            let delta_y = if background.vertical_tiled {
                let interval = background.tile_height.max(1.0);
                let offset = (background.vertical_offset + raw_delta_y).rem_euclid(interval);
                let delta = offset - background.vertical_offset;
                background.vertical_offset = offset;
                delta
            } else {
                background.vertical_offset += raw_delta_y;
                raw_delta_y
            };
            for index in background.sprites.clone() {
                if let Some((_, rect)) = draws.get_mut(index) {
                    rect.x += delta_x;
                    rect.y += delta_y;
                    if background.horizontal_tiled {
                        let period = background.tile_width * background.tile_count_x as f32;
                        let left_bound = -background.tile_width;
                        let right_bound = 1024.0 + background.tile_width;
                        if period > 0.0 && rect.x + rect.width <= left_bound {
                            let wraps =
                                ((left_bound - (rect.x + rect.width)) / period).floor() + 1.0;
                            rect.x += wraps * period;
                        } else if period > 0.0 && rect.x >= right_bound {
                            let wraps = ((rect.x - right_bound) / period).floor() + 1.0;
                            rect.x -= wraps * period;
                        }
                    }
                    if background.vertical_tiled {
                        let period = background.tile_height * background.tile_count_y as f32;
                        let top_bound = -background.tile_height;
                        let bottom_bound = 768.0 + background.tile_height;
                        if period > 0.0 && rect.y + rect.height <= top_bound {
                            let wraps =
                                ((top_bound - (rect.y + rect.height)) / period).floor() + 1.0;
                            rect.y += wraps * period;
                        } else if period > 0.0 && rect.y >= bottom_bound {
                            let wraps = ((rect.y - bottom_bound) / period).floor() + 1.0;
                            rect.y -= wraps * period;
                        }
                    }
                }
            }
        }
    }

    fn advance_remote_players(&mut self, updates: u8) {
        advance_face_expression(&mut self.local_face_expression, updates);
        let aura_elapsed = Duration::from_millis(8 * u64::from(updates));
        if let Some(mount) = self.local_mount.as_mut() {
            mount.advance(
                MountLookRuntime::motion_for_avatar(self.desired_avatar_motion),
                aura_elapsed,
            );
        }
        if let Some(aura) = self.local_item_aura.as_mut() {
            aura.advance(aura_elapsed);
        }
        for player in self.remote_players.values_mut() {
            advance_face_expression(&mut player.face_expression, updates);
            if player.dead
                && let Some(art) = player.death_art.as_mut()
            {
                art.advance(updates);
            }
            if let Some(aura) = player.item_aura.as_mut() {
                aura.advance(aura_elapsed);
            }
            player.invincibility_remaining = player
                .invincibility_remaining
                .saturating_sub(Duration::from_millis(8 * u64::from(updates)));
            player.speech_bubble_remaining = player.speech_bubble_remaining.and_then(|remaining| {
                let remaining =
                    remaining.saturating_sub(Duration::from_millis(8 * u64::from(updates)));
                (!remaining.is_zero()).then_some(remaining)
            });
            for _ in 0..updates {
                player.x += (player.target_x - player.x) * 0.18;
                player.y += (player.target_y - player.y) * 0.18;
                if player.recoil_velocity_x.abs() >= 0.05 {
                    player.recoil_offset_x += player.recoil_velocity_x;
                    player.recoil_velocity_x *= 0.82;
                } else {
                    player.recoil_velocity_x = 0.0;
                    player.recoil_offset_x *= 0.72;
                    if player.recoil_offset_x.abs() < 0.1 {
                        player.recoil_offset_x = 0.0;
                    }
                }
            }
            let desired = player
                .attack_stance
                .map(AvatarMotion::Attack)
                .unwrap_or_else(|| avatar_motion_for_stance(player.stance));
            if let Some(mount) = player.mount.as_mut() {
                mount.advance(
                    MountLookRuntime::motion_for_avatar(desired),
                    Duration::from_millis(8 * u64::from(updates)),
                );
            }
            if desired != player.desired_avatar_motion || player.active_avatar_frame.is_none() {
                player.desired_avatar_motion = desired;
                player.active_avatar_frame = first_avatar_frame(&player.avatar_frames, desired);
                player.avatar_elapsed = Duration::ZERO;
                continue;
            }
            let speed_factor = if player.attack_stance.is_some() {
                (1.7 - f32::from(player.attack_speed) / 10.0).clamp(0.2, 3.0)
            } else {
                1.0
            };
            player.avatar_elapsed = player
                .avatar_elapsed
                .saturating_add(Duration::from_secs_f32(
                    0.008 * f32::from(updates) * speed_factor,
                ));
            loop {
                let Some(active) = player.active_avatar_frame else {
                    break;
                };
                let delay = player.avatar_frames[active].delay;
                if player.avatar_elapsed < delay {
                    break;
                }
                player.avatar_elapsed -= delay;
                let motion = player.avatar_frames[active].motion;
                let next = match motion {
                    AvatarMotion::Attack(_) => (active + 1..player.avatar_frames.len())
                        .find(|candidate| player.avatar_frames[*candidate].motion == motion),
                    _ => (1..=player.avatar_frames.len())
                        .map(|offset| (active + offset) % player.avatar_frames.len())
                        .find(|candidate| player.avatar_frames[*candidate].motion == motion),
                };
                let Some(next) = next else {
                    if matches!(motion, AvatarMotion::Attack(_)) {
                        player.attack_stance = None;
                        player.desired_avatar_motion = avatar_motion_for_stance(player.stance);
                        player.active_avatar_frame =
                            first_avatar_frame(&player.avatar_frames, player.desired_avatar_motion);
                        player.avatar_elapsed = Duration::ZERO;
                    }
                    break;
                };
                player.active_avatar_frame = Some(next);
            }
        }
    }

    fn advance_npcs(&mut self, updates: u8) {
        let footholds = self.footholds.as_ref();
        for npc in self.npcs.values_mut() {
            for _ in 0..updates {
                if let Some(footholds) = footholds {
                    PhysicsEngine.move_body(footholds, &mut npc.body);
                } else {
                    npc.body.previous_x = npc.body.x;
                    npc.body.previous_y = npc.body.y;
                    npc.body.x += npc.body.horizontal_speed;
                    npc.body.y += npc.body.vertical_speed;
                }
                if npc.motion.is_walking() && !npc.body.flags.turn_at_edges {
                    npc.flipped = !npc.flipped;
                    npc.body.horizontal_speed = if npc.flipped { 1.0 } else { -1.0 };
                    npc.body.flags.turn_at_edges = true;
                }
                npc.x = npc.body.x;
                npc.y = npc.body.y;
                npc.foothold = npc.body.foothold_id;

                if !npc.frames().is_empty() {
                    npc.elapsed = npc.elapsed.saturating_add(Duration::from_millis(8));
                    loop {
                        let frame_count = npc.frames().len();
                        let Some(delay) =
                            npc.frames().get(npc.frame_index).map(|frame| frame.delay)
                        else {
                            break;
                        };
                        if npc.elapsed < delay {
                            break;
                        }
                        npc.elapsed -= delay;
                        if npc.frame_index + 1 < frame_count {
                            npc.frame_index += 1;
                        } else {
                            // The legacy NPC animation is one-shot: after a
                            // clip completes it chooses another authored NX state.
                            npc.frame_index = 0;
                            if let Some(next) = npc.next_random_motion() {
                                npc.set_motion(next);
                            }
                        }
                    }
                }
                if npc.quest_mark_frames.len() >= 2 {
                    npc.quest_mark_elapsed = npc
                        .quest_mark_elapsed
                        .saturating_add(Duration::from_millis(8));
                    loop {
                        let Some(delay) = npc
                            .quest_mark_frames
                            .get(npc.quest_mark_frame_index)
                            .map(|frame| frame.delay)
                        else {
                            break;
                        };
                        if npc.quest_mark_elapsed < delay {
                            break;
                        }
                        npc.quest_mark_elapsed -= delay;
                        npc.quest_mark_frame_index =
                            (npc.quest_mark_frame_index + 1) % npc.quest_mark_frames.len();
                    }
                }
            }
        }
    }

    fn advance_hired_merchants(&mut self, updates: u8) {
        let elapsed = Duration::from_millis(8 * u64::from(updates));
        for merchant in self.hired_merchants.values_mut() {
            if merchant.frames.len() < 2 {
                continue;
            }
            merchant.elapsed = merchant.elapsed.saturating_add(elapsed);
            loop {
                let Some(delay) = merchant
                    .frames
                    .get(merchant.frame_index)
                    .map(|frame| frame.delay)
                else {
                    break;
                };
                if merchant.elapsed < delay {
                    break;
                }
                merchant.elapsed -= delay;
                merchant.frame_index = (merchant.frame_index + 1) % merchant.frames.len();
            }
        }
    }

    fn advance_quest_clear_effect(&mut self, updates: u8) {
        let mut finished = false;
        if let Some(effect) = self.quest_clear_effect.as_mut() {
            for _ in 0..updates {
                effect.elapsed = effect.elapsed.saturating_add(Duration::from_millis(8));
                loop {
                    let Some(frame) = effect.frames.get(effect.frame_index) else {
                        finished = true;
                        break;
                    };
                    if effect.elapsed < frame.delay {
                        break;
                    }
                    effect.elapsed -= frame.delay;
                    if effect.frame_index + 1 >= effect.frames.len() {
                        finished = true;
                        break;
                    }
                    effect.frame_index += 1;
                }
                if finished {
                    break;
                }
            }
        }
        if finished {
            self.quest_clear_effect = None;
        }
    }

    fn apply_npc_action(&mut self, action: openstory_protocol::NpcAction) {
        let Some(action_code) = action.action else {
            return;
        };
        let motion = match action_code {
            1 => NpcMotion::Move,
            2 => NpcMotion::Say,
            _ => NpcMotion::Stand,
        };
        if let Some(npc) = self.npcs.get_mut(&action.object_id) {
            npc.set_motion(motion);
        }
    }

    fn set_remote_chair(&mut self, character_id: i32, item_id: i32) {
        let Some(player) = self.remote_players.get_mut(&character_id) else {
            return;
        };
        player.stance = if item_id > 0 { 20 } else { 0 };
        player.attack_stance = None;
        player.desired_avatar_motion = if item_id > 0 {
            AvatarMotion::Sit
        } else {
            AvatarMotion::Stand
        };
        player.active_avatar_frame =
            first_avatar_frame(&player.avatar_frames, player.desired_avatar_motion);
        player.avatar_elapsed = Duration::ZERO;
    }

    fn mark_npcs_scriptable(&mut self, npc_ids: &[i32]) {
        self.scriptable_npc_ids.extend(npc_ids.iter().copied());
        for npc in self.npcs.values_mut() {
            if self.scriptable_npc_ids.contains(&npc.npc_id) {
                npc.scripted = true;
            }
        }
    }

    fn scripted_npc_at(&self, point: Point) -> Option<i32> {
        let camera_bias_x = self.camera_offset.0 + self.player.x as f32 - 512.0;
        let camera_bias_y = self.camera_offset.1 + self.player.y as f32 - GAMEPLAY_CAMERA_CENTER_Y;
        self.npcs
            .iter()
            .filter(|(_, npc)| npc.scripted)
            .filter_map(|(object_id, npc)| {
                let frame = npc.frames().get(npc.frame_index)?;
                let offset_x = (npc.x - self.player.x) as f32 + camera_bias_x;
                let offset_y = (npc.y - self.player.y) as f32 + camera_bias_y;
                frame
                    .sprites
                    .iter()
                    .any(|(_, rect)| {
                        let x = if npc.flipped {
                            1024.0 - (rect.x + rect.width) + offset_x
                        } else {
                            rect.x + offset_x
                        };
                        rect_contains(
                            SpriteRect {
                                x,
                                y: rect.y + offset_y,
                                width: rect.width,
                                height: rect.height,
                                blend_mode: rect.blend_mode,
                            },
                            point,
                        )
                    })
                    .then_some(*object_id)
            })
            .next()
    }

    fn hired_merchant_at(&self, point: Point) -> Option<i32> {
        let camera_bias_x = self.camera_offset.0 + self.player.x as f32 - 512.0;
        let camera_bias_y = self.camera_offset.1 + self.player.y as f32 - GAMEPLAY_CAMERA_CENTER_Y;
        self.hired_merchants
            .iter()
            .filter_map(|(&owner_id, merchant)| {
                let x = 512.0 + (merchant.x - self.player.x) as f32 + camera_bias_x;
                let y =
                    GAMEPLAY_CAMERA_CENTER_Y + (merchant.y - self.player.y) as f32 + camera_bias_y;
                (point.x >= x - 28.0 && point.x <= x + 28.0 && point.y >= y - 85.0 && point.y <= y)
                    .then_some((owner_id, (point.x - x).abs() + (point.y - y).abs()))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(owner_id, _)| owner_id)
    }

    fn personal_shop_at(&self, point: Point) -> Option<i32> {
        let camera_bias_x = self.camera_offset.0 + self.player.x as f32 - 512.0;
        let camera_bias_y = self.camera_offset.1 + self.player.y as f32 - GAMEPLAY_CAMERA_CENTER_Y;
        self.remote_players
            .values()
            .filter_map(|player| {
                let room = player.mini_room.as_ref()?;
                if room.room_type != 4 {
                    return None;
                }
                let x = 512.0 + player.x as f32 + player.recoil_offset_x as f32
                    - self.player.x as f32
                    + camera_bias_x;
                let y = GAMEPLAY_CAMERA_CENTER_Y + player.y as f32 - self.player.y as f32
                    + camera_bias_y;
                (point.x >= x - 30.0 && point.x <= x + 30.0 && point.y >= y - 70.0 && point.y <= y)
                    .then_some((room.object_id, (point.x - x).abs() + (point.y - y).abs()))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(object_id, _)| object_id)
    }

    fn remote_character_at(&self, point: Point) -> Option<i32> {
        let camera_bias_x = self.camera_offset.0 + self.player.x as f32 - 512.0;
        let camera_bias_y = self.camera_offset.1 + self.player.y as f32 - GAMEPLAY_CAMERA_CENTER_Y;
        self.remote_players
            .iter()
            .filter_map(|(&character_id, player)| {
                let x = 512.0 + player.x as f32 + player.recoil_offset_x as f32
                    - self.player.x as f32
                    + camera_bias_x;
                let y = GAMEPLAY_CAMERA_CENTER_Y + player.y as f32 - self.player.y as f32
                    + camera_bias_y;
                (point.x >= x - 30.0 && point.x <= x + 30.0 && point.y >= y - 70.0 && point.y <= y)
                    .then_some((character_id, (point.x - x).abs() + (point.y - y).abs()))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(character_id, _)| character_id)
    }

    fn refresh_existing_drop(&mut self, spawn: openstory_protocol::MapDropSpawn) -> bool {
        // Legacy MapDrops reactivates an existing object without rebuilding its
        // motion, icon, or ownership state. Active map drops need no further work.
        self.drops.contains_key(&spawn.object_id)
    }

    fn spawn_drop(
        &mut self,
        spawn: openstory_protocol::MapDropSpawn,
        frames: Vec<DropFrameRuntime>,
    ) {
        let mut body = PhysicsBody::new(f64::from(spawn.source_x), f64::from(spawn.source_y) - 4.0);
        body.horizontal_speed = (f64::from(spawn.destination_x) - f64::from(spawn.source_x)) / 48.0;
        body.vertical_speed = -5.0;
        if spawn.mode == 2 {
            body.x = f64::from(spawn.destination_x);
            body.y = f64::from(spawn.destination_y) - 4.0;
            body.previous_x = body.x;
            body.previous_y = body.y;
            body.mode = PhysicsMode::Fixated;
            body.horizontal_speed = 0.0;
            body.vertical_speed = 0.0;
        }
        let mut drop = MapDropRuntime {
            x: f64::from(spawn.source_x),
            y: f64::from(spawn.source_y) - 4.0,
            destination_x: f64::from(spawn.destination_x),
            destination_y: f64::from(spawn.destination_y) - 4.0,
            body,
            frames,
            frame_index: 0,
            frame_elapsed: Duration::ZERO,
            motion: DropMotion::Falling {
                elapsed: Duration::ZERO,
            },
        };
        if spawn.mode == 2 {
            drop.x = drop.destination_x;
            drop.y = drop.destination_y;
            drop.motion = DropMotion::Floating { phase: 0.0 };
        } else if spawn.mode == 3 {
            drop.motion = DropMotion::PickedUp {
                mode: 0,
                looter_id: None,
                elapsed: Duration::ZERO,
            };
        }
        self.drops.insert(spawn.object_id, drop);
    }

    fn remove_drop(&mut self, update: openstory_protocol::MapDropRemove) {
        match update.mode {
            1 => {
                self.drops.remove(&update.object_id);
            }
            0 | 2 => {
                if let Some(drop) = self.drops.get_mut(&update.object_id) {
                    if update.mode == 2 {
                        drop.body.mode = PhysicsMode::Normal;
                        drop.body.vertical_speed = -4.5;
                    }
                    drop.motion = DropMotion::PickedUp {
                        mode: update.mode,
                        looter_id: (!update.looted_by_pet)
                            .then_some(update.looter_id)
                            .flatten(),
                        elapsed: Duration::ZERO,
                    };
                }
            }
            _ => {}
        }
    }

    fn nearest_drop(&self) -> Option<i32> {
        self.drops
            .iter()
            .filter(|(_, drop)| {
                !matches!(drop.motion, DropMotion::PickedUp { .. })
                    && self.player.x >= drop.x
                    && self.player.x <= drop.x + 32.0
                    && self.player.y >= drop.y
                    && self.player.y <= drop.y + 32.0
            })
            .min_by(|(_, left), (_, right)| {
                let left_distance = (left.x + 16.0 - self.player.x).powi(2)
                    + (left.y + 16.0 - self.player.y).powi(2);
                let right_distance = (right.x + 16.0 - self.player.x).powi(2)
                    + (right.y + 16.0 - self.player.y).powi(2);
                left_distance.total_cmp(&right_distance)
            })
            .map(|(object_id, _)| *object_id)
    }

    fn nearest_attack_targets(
        &self,
        maximum_targets: usize,
        range: [i16; 4],
        horizontal_range_multiplier: f64,
    ) -> Vec<(i32, MobTraits, (f64, f64))> {
        if maximum_targets == 0 {
            return Vec::new();
        }
        let horizontal_edge = f64::from(range[0]) * horizontal_range_multiplier;
        let (range_left, range_right) = if self.facing_right {
            (
                self.player.x - f64::from(range[1]),
                self.player.x - horizontal_edge,
            )
        } else {
            (
                self.player.x + horizontal_edge,
                self.player.x + f64::from(range[1]),
            )
        };
        let range_top = self.player.y + f64::from(range[2]);
        let range_bottom = self.player.y + f64::from(range[3]);
        let mut targets = self
            .mobs
            .iter()
            .filter(|(_, mob)| !mob.dying)
            .filter_map(|(object_id, mob)| {
                let dx = mob.x - self.player.x;
                let dy = mob.y - self.player.y;
                (mob.x >= range_left
                    && mob.x <= range_right
                    && mob.y >= range_top
                    && mob.y <= range_bottom)
                    .then_some((*object_id, mob.traits, (mob.x, mob.y), dx * dx + dy * dy))
            })
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.3.total_cmp(&right.3));
        targets
            .into_iter()
            .take(maximum_targets.min(15))
            .map(|(object_id, traits, position, _)| (object_id, traits, position))
            .collect()
    }

    fn check_mob_touch(&mut self, elapsed: Duration, protected: bool) -> Option<MobTouchAttack> {
        self.touch_damage_cooldown = self.touch_damage_cooldown.saturating_sub(elapsed);
        self.local_invincibility_remaining =
            self.local_invincibility_remaining.saturating_sub(elapsed);
        if protected
            || !self.touch_damage_cooldown.is_zero()
            || !self.local_invincibility_remaining.is_zero()
        {
            return None;
        }

        let player_left = self.player.x - 10.0;
        let player_right = self.player.x + 10.0;
        let player_top = self.player.y - 46.0;
        let player_bottom = self.player.y;
        let candidate = self
            .mobs
            .iter()
            .filter(|(_, mob)| !mob.dying && mob.traits.body_attack)
            .filter_map(|(object_id, mob)| {
                let frame = mob
                    .animations
                    .get(&mob.motion)
                    .and_then(|frames| frames.get(mob.frame_index))?;
                let bounds = frame.sprites.iter().fold(
                    None,
                    |bounds: Option<(f64, f64, f64, f64)>, (_, rect)| {
                        let left = mob.x + f64::from(rect.x - 512.0);
                        let top = mob.y + f64::from(rect.y - GAMEPLAY_CAMERA_CENTER_Y);
                        let right = left + f64::from(rect.width);
                        let bottom = top + f64::from(rect.height);
                        Some(match bounds {
                            Some((min_x, min_y, max_x, max_y)) => (
                                min_x.min(left),
                                min_y.min(top),
                                max_x.max(right),
                                max_y.max(bottom),
                            ),
                            None => (left, top, right, bottom),
                        })
                    },
                )?;
                let (mob_left, mob_top, mob_right, mob_bottom) = bounds;
                let overlaps = player_left < mob_right
                    && player_right > mob_left
                    && player_top < mob_bottom
                    && player_bottom > mob_top;
                overlaps.then_some((
                    *object_id,
                    mob.mob_id,
                    mob.traits.physical_attack,
                    mob.x,
                    (mob.x - self.player.x).powi(2) + (mob.y - self.player.y).powi(2),
                ))
            })
            .min_by(|left, right| left.4.total_cmp(&right.4));
        let (object_id, mob_id, physical_attack, origin_x, _) = candidate?;
        let minimum = (f64::from(physical_attack) * 0.8) as i32;
        let damage_span = physical_attack.saturating_sub(minimum).saturating_add(1);
        let damage = minimum
            .saturating_add((self.next_attack_random() * f64::from(damage_span.max(1))) as i32);
        self.touch_damage_cooldown = Duration::from_millis(500);
        Some(MobTouchAttack {
            object_id,
            mob_id,
            damage,
            origin_x,
        })
    }

    fn nearest_reactor_target(
        &self,
        range: [i16; 4],
        horizontal_range_multiplier: f64,
    ) -> Option<i32> {
        let horizontal_edge = f64::from(range[0]) * horizontal_range_multiplier;
        let (range_left, range_right) = if self.facing_right {
            (
                self.player.x - f64::from(range[1]),
                self.player.x - horizontal_edge,
            )
        } else {
            (
                self.player.x + horizontal_edge,
                self.player.x + f64::from(range[1]),
            )
        };
        let range_top = self.player.y + f64::from(range[2]);
        let range_bottom = self.player.y + f64::from(range[3]);
        self.reactors
            .iter()
            .filter(|(_, reactor)| reactor.hittable && !reactor.dead)
            .filter_map(|(object_id, reactor)| {
                let reactor_left = reactor.x - 30.0;
                let reactor_right = reactor.x + reactor.width - 10.0;
                let reactor_top = reactor.y - reactor.height;
                let reactor_bottom = reactor.y;
                let overlaps = range_left <= reactor_right
                    && range_right >= reactor_left
                    && range_top <= reactor_bottom
                    && range_bottom >= reactor_top;
                overlaps.then_some((
                    *object_id,
                    (reactor.x - self.player.x).powi(2) + (reactor.y - self.player.y).powi(2),
                ))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(object_id, _)| object_id)
    }

    fn set_reactor_state(&mut self, object_id: i32, state: i8) {
        let Some(reactor) = self.reactors.get_mut(&object_id) else {
            return;
        };
        let previous_state = reactor.state;
        if reactor.hittable {
            reactor.active_frames = reactor.hit_animations.get(&previous_state).cloned();
            reactor.frame_index = 0;
            reactor.frame_elapsed = Duration::ZERO;
        }
        reactor.state = state;
    }

    fn remove_reactor(&mut self, object_id: i32, _state: i8) {
        let Some(reactor) = self.reactors.get_mut(&object_id) else {
            return;
        };
        reactor.active_frames = reactor.hit_animations.get(&reactor.state).cloned();
        reactor.frame_index = 0;
        reactor.frame_elapsed = Duration::ZERO;
        reactor.dead = true;
        if reactor.active_frames.is_none() {
            self.reactors.remove(&object_id);
        }
    }

    fn advance_reactors(&mut self, updates: u8) {
        let elapsed = Duration::from_millis(8 * u64::from(updates));
        self.reactors.retain(|_, reactor| {
            let Some(frames) = reactor.active_frames.as_ref() else {
                return !reactor.dead;
            };
            if frames.is_empty() {
                return !reactor.dead;
            }
            reactor.frame_elapsed = reactor.frame_elapsed.saturating_add(elapsed);
            loop {
                let Some(frame) = frames.get(reactor.frame_index) else {
                    break;
                };
                if reactor.frame_elapsed < frame.delay {
                    break;
                }
                reactor.frame_elapsed -= frame.delay;
                if reactor.frame_index + 1 >= frames.len() {
                    if reactor.dead {
                        return false;
                    }
                    reactor.active_frames = None;
                    reactor.frame_index = 0;
                    reactor.frame_elapsed = Duration::ZERO;
                    break;
                }
                reactor.frame_index += 1;
            }
            true
        });
    }

    fn advance_drops(&mut self, updates: u8, local_character_id: Option<i32>) {
        let local = (self.player.x, self.player.y, self.player.horizontal_speed);
        let remote_players = &self.remote_players;
        let footholds = self.footholds.as_ref();
        let mut removed = Vec::new();
        for (object_id, drop) in &mut self.drops {
            let mut expired = false;
            for _ in 0..updates {
                if drop.frames.len() > 1 {
                    drop.frame_elapsed =
                        drop.frame_elapsed.saturating_add(Duration::from_millis(8));
                    while let Some(frame) = drop.frames.get(drop.frame_index) {
                        if drop.frame_elapsed < frame.delay {
                            break;
                        }
                        drop.frame_elapsed -= frame.delay;
                        drop.frame_index = (drop.frame_index + 1) % drop.frames.len();
                    }
                }
                if let Some(footholds) = footholds {
                    PhysicsEngine.move_body(footholds, &mut drop.body);
                } else if drop.body.mode != PhysicsMode::Fixated {
                    drop.body.previous_x = drop.body.x;
                    drop.body.previous_y = drop.body.y;
                    drop.body.vertical_speed += 0.14;
                    drop.body.x += drop.body.horizontal_speed;
                    drop.body.y += drop.body.vertical_speed;
                    if drop.body.y >= drop.destination_y {
                        drop.body.y = drop.destination_y;
                        drop.body.vertical_speed = 0.0;
                        drop.body.on_ground = true;
                    }
                }
                drop.x = drop.body.x;
                drop.y = drop.body.y;
                match &mut drop.motion {
                    DropMotion::Falling { elapsed } => {
                        *elapsed = elapsed.saturating_add(Duration::from_millis(8));
                        if drop.body.on_ground {
                            drop.body.mode = PhysicsMode::Fixated;
                            drop.body.horizontal_speed = 0.0;
                            drop.body.vertical_speed = 0.0;
                            drop.body.x = drop.destination_x;
                            drop.body.y = drop.destination_y;
                            drop.body.previous_x = drop.body.x;
                            drop.body.previous_y = drop.body.y;
                            drop.x = drop.destination_x;
                            drop.y = drop.destination_y;
                            drop.motion = DropMotion::Floating { phase: 0.0 };
                        }
                    }
                    DropMotion::Floating { phase } => {
                        *phase = (*phase + 0.025).rem_euclid(std::f64::consts::TAU);
                        drop.x = drop.destination_x;
                        drop.y = drop.destination_y + 5.0 + (phase.cos() - 1.0) * 2.5;
                        drop.body.x = drop.x;
                        drop.body.y = drop.y;
                    }
                    DropMotion::PickedUp {
                        mode,
                        looter_id,
                        elapsed,
                    } => {
                        if *mode == 2 {
                            let looter = looter_id.and_then(|id| {
                                if Some(id) == local_character_id {
                                    Some(local)
                                } else {
                                    remote_players
                                        .get(&id)
                                        .map(|player| (player.x, player.y, 0.0))
                                }
                            });
                            if let Some((x, _, horizontal_speed)) = looter {
                                drop.body.horizontal_speed =
                                    horizontal_speed / 2.0 + (x - drop.body.x - 16.0) / 48.0;
                            }
                        }
                        *elapsed = elapsed.saturating_add(Duration::from_millis(8));
                        expired = *elapsed >= Duration::from_millis(384);
                        if expired {
                            break;
                        }
                    }
                }
            }
            if expired {
                removed.push(*object_id);
            }
        }
        for object_id in removed {
            self.drops.remove(&object_id);
        }
    }

    fn launch_projectile(
        &mut self,
        frames: Vec<ProjectileFrameRuntime>,
        target_x: f64,
        target_y: f64,
    ) {
        let (source_x, source_y) = (self.player.x, self.player.y);
        self.launch_projectile_from(frames, source_x, source_y, target_x, target_y);
    }

    fn launch_projectile_from(
        &mut self,
        frames: Vec<ProjectileFrameRuntime>,
        source_x: f64,
        source_y: f64,
        target_x: f64,
        target_y: f64,
    ) {
        if frames.is_empty() {
            return;
        }
        if self.projectiles.len() >= 64 {
            self.projectiles.remove(0);
        }
        let facing_left = target_x < source_x;
        self.projectiles.push(MapProjectileRuntime {
            frames,
            frame_index: 0,
            frame_elapsed: Duration::ZERO,
            x: source_x + if facing_left { -30.0 } else { 30.0 },
            y: source_y - 26.0,
            target_x,
            target_y,
            facing_left,
        });
    }

    fn advance_projectiles(&mut self, updates: u8) {
        for _ in 0..updates {
            self.projectiles.retain_mut(|projectile| {
                projectile.frame_elapsed = projectile
                    .frame_elapsed
                    .saturating_add(Duration::from_millis(8));
                while let Some(frame) = projectile.frames.get(projectile.frame_index) {
                    if projectile.frame_elapsed < frame.delay {
                        break;
                    }
                    projectile.frame_elapsed -= frame.delay;
                    projectile.frame_index = (projectile.frame_index + 1) % projectile.frames.len();
                }

                let dx = projectile.target_x - projectile.x;
                let dy = projectile.target_y - projectile.y;
                if dx.abs() < 10.0 {
                    return false;
                }
                let horizontal_speed = if dx > 0.0 {
                    (dx / 32.0).clamp(3.0, 6.0)
                } else {
                    (dx / 32.0).clamp(-6.0, -3.0)
                };
                projectile.x += horizontal_speed;
                projectile.y += horizontal_speed * dy / dx;
                projectile.facing_left = horizontal_speed < 0.0;
                true
            });
        }
    }

    fn advance_local_chair_animations(&mut self, updates: u8) {
        let Some(chair) = self.local_chair.as_mut() else {
            return;
        };
        for animation in [&mut chair.behind, &mut chair.front] {
            if animation.frames.len() < 2 {
                continue;
            }
            animation.elapsed = animation
                .elapsed
                .saturating_add(Duration::from_millis(8 * u64::from(updates)));
            while let Some(frame) = animation.frames.get(animation.frame_index) {
                if animation.elapsed < frame.delay {
                    break;
                }
                animation.elapsed -= frame.delay;
                animation.frame_index = (animation.frame_index + 1) % animation.frames.len();
            }
        }
    }

    fn advance_summons(&mut self, updates: u8) {
        let footholds = self.footholds.as_ref();
        for summon in self.summons.values_mut() {
            for _ in 0..updates {
                if summon.frames.len() > 1 {
                    summon.elapsed = summon.elapsed.saturating_add(Duration::from_millis(8));
                    while let Some(frame) = summon.frames.get(summon.frame_index) {
                        if summon.elapsed < frame.delay {
                            break;
                        }
                        summon.elapsed -= frame.delay;
                        summon.frame_index = (summon.frame_index + 1) % summon.frames.len();
                    }
                }
                if let Some(footholds) = footholds {
                    PhysicsEngine.move_body(footholds, &mut summon.body);
                    summon.x = summon.body.x;
                    summon.y = summon.body.y;
                }
            }
        }
    }

    fn advance_dragons(&mut self, updates: u8) {
        let footholds = self.footholds.as_ref();
        for dragon in self.dragons.values_mut() {
            for _ in 0..updates {
                if dragon.frames.len() > 1 {
                    dragon.elapsed = dragon.elapsed.saturating_add(Duration::from_millis(8));
                    while let Some(frame) = dragon.frames.get(dragon.frame_index) {
                        if dragon.elapsed < frame.delay {
                            break;
                        }
                        dragon.elapsed -= frame.delay;
                        dragon.frame_index = (dragon.frame_index + 1) % dragon.frames.len();
                    }
                }
                if let Some(footholds) = footholds {
                    PhysicsEngine.move_body(footholds, &mut dragon.body);
                    dragon.x = dragon.body.x;
                    dragon.y = dragon.body.y;
                }
            }
        }
    }

    fn advance_pets(&mut self, updates: u8, local_character_id: Option<i32>) -> Vec<Vec<u8>> {
        let owner = (self.player.x, self.player.y);
        let mut packets = Vec::new();
        for _ in 0..updates {
            if self.pet_loot_cooldown > 0 {
                self.pet_loot_cooldown -= 1;
            }
            if self.pet_loot_cooldown == 0 {
                self.pet_loot_cooldown = 10;
                packets.extend(self.update_local_pet_loot(local_character_id));
            }
            let footholds = self.footholds.as_ref();
            for pet in self.pets.values_mut() {
                pet.advance(1);
                if Some(pet.owner_id) == local_character_id {
                    pet.follow_local(owner, footholds);
                }
            }
        }
        packets
    }

    fn update_local_pet_loot(&mut self, local_character_id: Option<i32>) -> Vec<Vec<u8>> {
        let Some(local_character_id) = local_character_id else {
            return Vec::new();
        };
        let player = (self.player.x, self.player.y);
        let drops = &self.drops;
        let pets = &mut self.pets;
        let mut cooldown = self.pet_loot_cooldown;
        let mut packets = Vec::new();

        for slot in 0..3 {
            let Some(pet) = pets.get_mut(&(local_character_id, slot)) else {
                continue;
            };
            let mut pickup: Option<i32> = None;
            let mut nearest: Option<(f64, i32, (f64, f64))> = None;
            for (&object_id, drop) in drops {
                if matches!(drop.motion, DropMotion::PickedUp { .. }) {
                    continue;
                }
                let pet_dx = (drop.x - pet.x).abs();
                let pet_dy = (drop.y - pet.y).abs();
                if pet_dx <= 30.0 && pet_dy <= 25.0 {
                    pickup = Some(pickup.map_or(object_id, |current| current.min(object_id)));
                }
                let player_distance = (drop.x - player.0).abs() + (drop.y - player.1).abs();
                if player_distance < 200.0
                    && nearest.is_none_or(|(best_distance, best_id, _)| {
                        player_distance < best_distance
                            || (player_distance == best_distance && object_id < best_id)
                    })
                {
                    nearest = Some((player_distance, object_id, (drop.x, drop.y)));
                }
            }
            if let Some(object_id) = pickup {
                packets.push(openstory_protocol::pet_loot_packet(
                    pet.unique_id,
                    object_id,
                ));
                pet.loot_target = None;
                cooldown = 60;
            } else if let Some((_, _, target)) = nearest {
                pet.loot_target = Some(target);
            } else if cooldown != 60 {
                pet.loot_target = None;
            }
        }
        self.pet_loot_cooldown = cooldown;
        packets
    }

    fn show_character_skill_effect(
        &mut self,
        character_id: Option<i32>,
        animations: Vec<CharacterSkillEffectAnimation>,
        speed_factor: f32,
    ) {
        for animation in animations {
            if !animation.frames.is_empty() {
                self.character_effects.push(MapCharacterSkillEffect {
                    character_id,
                    point_position: None,
                    z: animation.z,
                    speed_factor,
                    frames: animation.frames,
                    frame_index: 0,
                    elapsed: Duration::ZERO,
                });
            }
        }
    }

    fn show_point_skill_effect(&mut self, x: f64, y: f64, frames: Vec<ProjectileFrameRuntime>) {
        if !frames.is_empty() {
            self.character_effects.push(MapCharacterSkillEffect {
                character_id: None,
                point_position: Some((x, y)),
                z: 0,
                speed_factor: 1.0,
                frames,
                frame_index: 0,
                elapsed: Duration::ZERO,
            });
        }
    }

    fn advance_character_effects(&mut self, updates: u8) {
        self.character_effects.retain_mut(|effect| {
            for _ in 0..updates {
                let Some(frame) = effect.frames.get(effect.frame_index) else {
                    return false;
                };
                effect.elapsed = effect.elapsed.saturating_add(Duration::from_secs_f32(
                    0.008 * effect.speed_factor.clamp(0.2, 3.0),
                ));
                if effect.elapsed < frame.delay {
                    continue;
                }
                effect.elapsed -= frame.delay;
                if effect.frame_index + 1 >= effect.frames.len() {
                    return false;
                }
                effect.frame_index += 1;
            }
            true
        });
    }

    fn show_screen_effect(&mut self, frames: Vec<ProjectileFrameRuntime>) {
        self.screen_effect = (!frames.is_empty()).then_some(MapScreenEffect {
            frames,
            frame_index: 0,
            elapsed: Duration::ZERO,
        });
    }

    fn advance_screen_effect(&mut self, updates: u8) {
        let mut finished = false;
        if let Some(effect) = self.screen_effect.as_mut() {
            for _ in 0..updates {
                let Some(frame) = effect.frames.get(effect.frame_index) else {
                    finished = true;
                    break;
                };
                effect.elapsed = effect.elapsed.saturating_add(Duration::from_millis(6));
                if effect.elapsed < frame.delay {
                    continue;
                }
                effect.elapsed -= frame.delay;
                if effect.frame_index + 1 >= effect.frames.len() {
                    finished = true;
                    break;
                }
                effect.frame_index += 1;
            }
        }
        if finished {
            self.screen_effect = None;
        }
    }

    fn show_mob_hit_effects(
        &mut self,
        object_id: i32,
        animations: &[SkillHitEffectAnimation],
        flip_x: bool,
    ) {
        let Some(mob) = self.mobs.get_mut(&object_id).filter(|mob| !mob.dying) else {
            return;
        };
        let head_anchor = mob
            .animations
            .get(&mob.motion)
            .and_then(|frames| frames.get(mob.frame_index))
            .and_then(|frame| {
                frame
                    .sprites
                    .iter()
                    .fold(None, |bounds: Option<(f32, f32, f32)>, (_, rect)| {
                        Some(match bounds {
                            Some((top, left, right)) => (
                                top.min(rect.y),
                                left.min(rect.x),
                                right.max(rect.x + rect.width),
                            ),
                            None => (rect.y, rect.x, rect.x + rect.width),
                        })
                    })
            })
            .map_or((0.0, 0.0), |(top, left, right)| {
                ((left + right) * 0.5 - 512.0, top - GAMEPLAY_CAMERA_CENTER_Y)
            });
        for animation in animations {
            if animation.frames.is_empty() {
                continue;
            }
            if mob.hit_effects.len() >= 32 {
                mob.hit_effects.remove(0);
            }
            let (anchor_x, anchor_y) = if animation.pos == 0 {
                head_anchor
            } else {
                (0.0, 0.0)
            };
            mob.hit_effects.push(MapMobHitEffect {
                z: animation.z,
                flip_x,
                anchor_x,
                anchor_y,
                frames: animation.frames.clone(),
                frame_index: 0,
                elapsed: Duration::ZERO,
            });
        }
    }

    fn schedule_mob_attack_impact(
        &mut self,
        object_id: i32,
        hit_effects: Arc<Vec<SkillHitEffectAnimation>>,
        hit_effect_flip: bool,
        damage: Option<(openstory_protocol::MobDamage, bool, f32)>,
        delay: Duration,
    ) {
        if hit_effects.is_empty() && damage.is_none() {
            return;
        }
        if delay.is_zero() {
            self.apply_mob_attack_impact(
                object_id,
                &hit_effects,
                hit_effect_flip,
                damage,
            );
            return;
        }
        if self.pending_mob_attack_impacts.len() >= 512 {
            self.pending_mob_attack_impacts.remove(0);
        }
        self.pending_mob_attack_impacts.push(PendingMobAttackImpact {
            object_id,
            remaining: delay,
            hit_effects,
            hit_effect_flip,
            damage,
        });
    }

    fn apply_mob_attack_impact(
        &mut self,
        object_id: i32,
        hit_effects: &[SkillHitEffectAnimation],
        hit_effect_flip: bool,
        damage: Option<(openstory_protocol::MobDamage, bool, f32)>,
    ) {
        self.show_mob_hit_effects(object_id, hit_effects, hit_effect_flip);
        if let Some((damage, critical, stack_offset)) = damage {
            self.show_mob_damage_line(damage, critical, stack_offset);
        }
    }

    fn advance_mobs(&mut self, updates: u8) -> Vec<Vec<u8>> {
        let elapsed = Duration::from_millis(8 * u64::from(updates));
        self.damage_numbers.retain_mut(|number| {
            number.elapsed = number.elapsed.saturating_add(elapsed);
            number.elapsed < number.lifetime
        });
        let mut due_impacts = Vec::new();
        self.pending_mob_attack_impacts.retain_mut(|impact| {
            impact.remaining = impact.remaining.saturating_sub(elapsed);
            if impact.remaining.is_zero() {
                due_impacts.push(PendingMobAttackImpact {
                    object_id: impact.object_id,
                    remaining: impact.remaining,
                    hit_effects: Arc::clone(&impact.hit_effects),
                    hit_effect_flip: impact.hit_effect_flip,
                    damage: impact.damage,
                });
                false
            } else {
                true
            }
        });
        let footholds = self.footholds.as_ref();
        let mut packets = Vec::new();
        self.mobs.retain(|object_id, mob| {
            mob.hp_bar_remaining = mob.hp_bar_remaining.saturating_sub(elapsed);
            mob.hit_effects.retain_mut(|effect| {
                for _ in 0..updates {
                    effect.elapsed = effect.elapsed.saturating_add(Duration::from_millis(8));
                    loop {
                        let Some(frame) = effect.frames.get(effect.frame_index) else {
                            return false;
                        };
                        if effect.elapsed < frame.delay {
                            break;
                        }
                        effect.elapsed -= frame.delay;
                        if effect.frame_index + 1 >= effect.frames.len() {
                            return false;
                        }
                        effect.frame_index += 1;
                    }
                }
                true
            });
            if let Some(remaining) = mob.fade_remaining.as_mut() {
                *remaining = remaining.saturating_sub(elapsed);
                if remaining.is_zero() {
                    return false;
                }
            }
            if let Some(remaining) = mob.fade_in_remaining.as_mut() {
                *remaining = remaining.saturating_sub(elapsed);
                if remaining.is_zero() {
                    mob.fade_in_remaining = None;
                }
            }
            if mob.dying && mob.fade_remaining.is_some() {
                return true;
            }
            for _ in 0..updates {
                let animation_ended = advance_mob_animation(mob, Duration::from_millis(8));
                if mob.dying && mob.motion == MobMotion::Die && animation_ended {
                    return false;
                }
                if mob.dying {
                    continue;
                }
                let immobilized = mob.is_immobilized();
                if mob.control_mode <= 0 && !immobilized {
                    continue;
                }
                if immobilized {
                    if let Some(footholds) = footholds {
                        mob.body.apply_force(0.0, 0.0);
                        PhysicsEngine.move_body(footholds, &mut mob.body);
                        mob.x = mob.body.x;
                        mob.y = mob.body.y;
                        mob.foothold = mob.body.foothold_id;
                    }
                    continue;
                }

                mob.ai_counter = mob.ai_counter.saturating_add(1);
                if let Some(footholds) = footholds {
                    match mob.motion {
                        MobMotion::Move if mob.traits.can_fly => {
                            let horizontal = if mob.server_flip {
                                mob.traits.fly_speed
                            } else {
                                -mob.traits.fly_speed
                            };
                            let vertical = f64::from(mob.fly_direction) * mob.traits.fly_speed;
                            mob.body.apply_force(horizontal, vertical);
                        }
                        MobMotion::Move => {
                            let horizontal = if mob.server_flip {
                                mob.traits.speed
                            } else {
                                -mob.traits.speed
                            };
                            mob.body.apply_force(horizontal, 0.0);
                        }
                        MobMotion::Hit if mob.traits.can_move => {
                            let force = if mob.body.on_ground { 0.2 } else { 0.1 };
                            mob.body
                                .apply_force(if mob.server_flip { force } else { -force }, 0.0);
                        }
                        MobMotion::Jump => mob.body.apply_force(0.0, -5.0),
                        _ => mob.body.apply_force(0.0, 0.0),
                    }
                    PhysicsEngine.move_body(footholds, &mut mob.body);
                    mob.x = mob.body.x;
                    mob.y = mob.body.y;
                    mob.foothold = mob.body.foothold_id;
                    if !mob.body.flags.turn_at_edges {
                        mob.server_flip = !mob.server_flip;
                        mob.body.flags.turn_at_edges = true;
                        if mob.motion == MobMotion::Hit {
                            mob.set_motion(MobMotion::Stand);
                        }
                    }
                }

                if mob.motion == MobMotion::Jump && mob.body.on_ground {
                    mob.choose_next_move();
                    packets.push(mob_movement_packet(*object_id, mob, 0));
                    mob.ai_counter = 0;
                    continue;
                }

                let just_claimed = !mob.control_acked;
                if just_claimed {
                    mob.control_acked = true;
                    packets.push(mob_movement_packet(*object_id, mob, 0));
                } else if (mob.body.horizontal_speed != 0.0 || mob.body.vertical_speed != 0.0)
                    && mob.body.on_ground
                    && mob.ai_counter % 16 == 0
                {
                    packets.push(mob_movement_packet(*object_id, mob, 1));
                }

                let next = match mob.motion {
                    MobMotion::Hit => mob.ai_counter > 200,
                    MobMotion::Jump => false,
                    _ => animation_ended && mob.ai_counter > 200,
                };
                if next {
                    mob.choose_next_move();
                    packets.push(mob_movement_packet(*object_id, mob, 0));
                    mob.ai_counter = 0;
                }
            }
            true
        });
        for impact in due_impacts {
            self.apply_mob_attack_impact(
                impact.object_id,
                &impact.hit_effects,
                impact.hit_effect_flip,
                impact.damage,
            );
        }
        packets
    }

    fn apply_mob_movement(&mut self, movement: openstory_protocol::MapMobMovement) {
        let Some(mob) = self.mobs.get_mut(&movement.object_id) else {
            return;
        };
        if mob.control_mode > 0 {
            return;
        }
        // Equip/status-only fragments have no position fields. A trailing
        // command 10/14/21 must not reset a mob to the parser's zero defaults.
        if let Some(fragment) = movement
            .fragments
            .iter()
            .rev()
            .find(|fragment| fragment.kind != openstory_protocol::MovementKind::None)
        {
            mob.x = f64::from(fragment.x);
            mob.y = f64::from(fragment.y);
            if fragment.foothold != 0 {
                mob.foothold = fragment.foothold;
            }
            mob.body.previous_x = f64::from(fragment.last_x);
            mob.body.previous_y = f64::from(fragment.last_y);
            mob.body.x = mob.x;
            mob.body.y = mob.y;
            mob.body.foothold_id = mob.foothold;
            mob.body.horizontal_speed = 0.0;
            mob.body.vertical_speed = 0.0;
            mob.server_flip = fragment.stance & 1 == 0;
            mob.server_stance = fragment.stance;
            let desired = if mob.is_immobilized() {
                MobMotion::Stand
            } else {
                mob_motion_for_stance(fragment.stance)
            };
            if let Some(motion) = first_mob_motion(&mob.animations, desired) {
                if mob.motion != motion {
                    mob.motion = motion;
                    mob.frame_index = 0;
                    mob.elapsed = Duration::ZERO;
                }
            }
        }
        if !mob.is_immobilized() {
            if let Some(action) = mob_motion_for_action(movement.action) {
                mob.set_action_motion(action);
            }
        }
    }

    fn apply_mob_movement_response(
        &mut self,
        response: openstory_protocol::MobMovementResponse,
    ) -> Option<Vec<u8>> {
        if !response.use_skills {
            return None;
        }
        let mob = self.mobs.get_mut(&response.object_id)?;
        if mob.control_mode <= 0 || mob.dying || mob.is_immobilized() {
            return None;
        }
        let skill_id = response.skill_id as u8;
        let skill_level = u8::try_from(response.skill_level).ok()?;
        let (slot, action) = *mob.skill_actions.get(&(skill_id, skill_level))?;
        mob.set_action_motion(MobMotion::Skill(slot));
        Some(openstory_protocol::move_mob_action(
            response.object_id,
            0,
            action,
            skill_id,
            skill_level,
            0,
            openstory_protocol::PlayerMovement {
                x: rounded_i16(mob.body.x),
                y: rounded_i16(mob.body.y),
                last_x: rounded_i16(mob.body.previous_x),
                last_y: rounded_i16(mob.body.previous_y),
                foothold: mob.body.foothold_id,
                stance: mob.server_stance,
                duration: 1,
            },
        ))
    }

    fn spawn_mist(&mut self, mist: openstory_protocol::MapMistSpawn) {
        self.mists.insert(mist.object_id, mist);
    }

    fn remove_mist(&mut self, object_id: i32) {
        self.mists.remove(&object_id);
    }

    fn show_mob_hp(
        &mut self,
        update: openstory_protocol::MobHpUpdate,
    ) -> Option<(i32, u8)> {
        let Some(mob) = self.mobs.get_mut(&update.object_id) else {
            return None;
        };
        let percent = update.percentage.clamp(0, 100) as u8;
        mob.hp_percent = Some(percent);
        mob.hp_bar_remaining = Duration::from_secs(2);
        mob.traits.boss.then_some((mob.mob_id, percent))
    }

    fn show_mob_damage(&mut self, damage: openstory_protocol::MobDamage) {
        if damage.damage <= 0 {
            return;
        }
        self.show_mob_damage_line(damage, false, 0.0);
    }

    fn show_mob_damage_line(
        &mut self,
        damage: openstory_protocol::MobDamage,
        critical: bool,
        stack_offset: f32,
    ) {
        let Some(mob) = self.mobs.get(&damage.object_id) else {
            return;
        };
        let Some(frame) = mob
            .animations
            .get(&mob.motion)
            .and_then(|frames| frames.get(mob.frame_index))
        else {
            return;
        };
        let Some((top, left, right)) =
            frame
                .sprites
                .iter()
                .fold(None, |bounds: Option<(f32, f32, f32)>, (_, rect)| {
                    Some(match bounds {
                        Some((top, left, right)) => (
                            top.min(rect.y),
                            left.min(rect.x),
                            right.max(rect.x + rect.width),
                        ),
                        None => (rect.y, rect.x, rect.x + rect.width),
                    })
                })
        else {
            return;
        };
        let world_x = mob.x + f64::from((left + right) * 0.5 - 512.0);
        let world_y = mob.y + f64::from(top - GAMEPLAY_CAMERA_CENTER_Y - 4.0 - stack_offset);
        if self.damage_numbers.len() == 64 {
            self.damage_numbers.remove(0);
        }
        self.damage_numbers.push(MobDamageNumber {
            world_x,
            world_y,
            digits: if damage.damage <= 0 {
                Vec::new()
            } else {
                damage
                    .damage
                    .to_string()
                    .bytes()
                    .map(|digit| digit - b'0')
                    .collect()
            },
            miss: damage.damage <= 0,
            critical,
            heal: false,
            player_damage: false,
            lifetime: Duration::from_millis(1500),
            rise_speed: 31.25,
            elapsed: Duration::ZERO,
        });
    }

    fn show_remote_player_damage(&mut self, damage: openstory_protocol::RemotePlayerDamage) {
        let (world_x, world_y) = {
            let Some(player) = self.remote_players.get_mut(&damage.character_id) else {
                return;
            };
            player.invincibility_remaining = Duration::from_secs(2);
            if damage.damage > 0
                && let Some(direction) = damage.direction.filter(|direction| *direction <= 1)
            {
                player.recoil_velocity_x = if direction == 0 { -1.5 } else { 1.5 };
            }
            (player.x + player.recoil_offset_x, player.y - 48.0)
        };
        if self.damage_numbers.len() == 64 {
            self.damage_numbers.remove(0);
        }
        self.damage_numbers.push(MobDamageNumber {
            world_x,
            world_y,
            digits: if damage.damage <= 0 {
                Vec::new()
            } else {
                damage
                    .damage
                    .to_string()
                    .bytes()
                    .map(|digit| digit - b'0')
                    .collect()
            },
            miss: damage.damage <= 0,
            critical: false,
            heal: false,
            player_damage: true,
            lifetime: Duration::from_millis(1500),
            rise_speed: 31.25,
            elapsed: Duration::ZERO,
        });
    }

    fn show_local_player_damage(&mut self, damage: i32) {
        // Legacy Char::show_damage grants two seconds of contact immunity,
        // including when the displayed result is a miss.
        self.local_invincibility_remaining = Duration::from_secs(2);
        if self.damage_numbers.len() == 64 {
            self.damage_numbers.remove(0);
        }
        self.damage_numbers.push(MobDamageNumber {
            world_x: self.player.x,
            world_y: self.player.y - 48.0,
            digits: if damage <= 0 {
                Vec::new()
            } else {
                damage
                    .to_string()
                    .bytes()
                    .map(|digit| digit - b'0')
                    .collect()
            },
            miss: damage <= 0,
            critical: false,
            heal: false,
            player_damage: true,
            lifetime: Duration::from_millis(1500),
            rise_speed: 31.25,
            elapsed: Duration::ZERO,
        });
    }

    fn show_local_heal_backlash(&mut self, damage: i32) {
        if damage <= 0 {
            return;
        }
        if self.damage_numbers.len() == 64 {
            self.damage_numbers.remove(0);
        }
        self.damage_numbers.push(MobDamageNumber {
            world_x: self.player.x,
            world_y: self.player.y - 48.0,
            digits: damage
                .to_string()
                .bytes()
                .map(|digit| digit - b'0')
                .collect(),
            miss: false,
            critical: false,
            heal: false,
            player_damage: true,
            lifetime: Duration::from_millis(1500),
            rise_speed: 31.25,
            elapsed: Duration::ZERO,
        });
    }

    fn show_local_player_heal(&mut self, amount: i32) {
        if amount <= 0 {
            return;
        }
        if self.damage_numbers.len() == 64 {
            self.damage_numbers.remove(0);
        }
        self.damage_numbers.push(MobDamageNumber {
            world_x: self.player.x,
            world_y: self.player.y - 60.0,
            digits: amount
                .to_string()
                .bytes()
                .map(|digit| digit - b'0')
                .collect(),
            miss: false,
            critical: false,
            heal: true,
            player_damage: false,
            lifetime: Duration::from_millis(500),
            rise_speed: 7.5,
            elapsed: Duration::ZERO,
        });
    }

    fn show_remote_player_heal(&mut self, character_id: i32, amount: i32) {
        if amount <= 0 {
            return;
        }
        let Some(player) = self.remote_players.get(&character_id) else {
            return;
        };
        let world_x = player.x + player.recoil_offset_x;
        let world_y = player.y - 60.0;
        if self.damage_numbers.len() == 64 {
            self.damage_numbers.remove(0);
        }
        self.damage_numbers.push(MobDamageNumber {
            world_x,
            world_y,
            digits: amount
                .to_string()
                .bytes()
                .map(|digit| digit - b'0')
                .collect(),
            miss: false,
            critical: false,
            heal: true,
            player_damage: false,
            lifetime: Duration::from_millis(500),
            rise_speed: 7.5,
            elapsed: Duration::ZERO,
        });
    }

    fn apply_mob_status(&mut self, update: openstory_protocol::MobStatusUpdate) {
        let Some(mob) = self.mobs.get_mut(&update.object_id) else {
            return;
        };
        for entry in update.entries {
            mob.statuses.insert((entry.group, entry.flag));
        }
        if mob.is_immobilized() {
            if let Some(motion) = first_mob_motion(&mob.animations, MobMotion::Stand) {
                if mob.motion != motion {
                    mob.motion = motion;
                    mob.frame_index = 0;
                    mob.elapsed = Duration::ZERO;
                }
            }
        }
    }

    fn cancel_mob_status(&mut self, update: openstory_protocol::MobStatusCancel) {
        let Some(mob) = self.mobs.get_mut(&update.object_id) else {
            return;
        };
        for (group, mut mask) in [
            (
                openstory_protocol::MobStatusMaskGroup::First,
                update.first_mask,
            ),
            (
                openstory_protocol::MobStatusMaskGroup::Second,
                update.second_mask,
            ),
        ] {
            while mask != 0 {
                let flag = 1_u32 << mask.trailing_zeros();
                mob.statuses.remove(&(group, flag));
                mask &= !flag;
            }
        }
        if !mob.is_immobilized() {
            let desired = mob_motion_for_stance(mob.server_stance);
            if let Some(motion) = first_mob_motion(&mob.animations, desired) {
                if mob.motion != motion {
                    mob.motion = motion;
                    mob.frame_index = 0;
                    mob.elapsed = Duration::ZERO;
                }
            }
        }
    }

    fn set_mob_control(&mut self, object_id: i32, mode: i8) {
        if let Some(mob) = self.mobs.get_mut(&object_id) {
            let newly_controlled = mob.control_mode <= 0 && mode > 0;
            mob.control_mode = mode;
            if newly_controlled {
                mob.control_acked = false;
                if mob.motion == MobMotion::Stand {
                    mob.choose_next_move();
                }
            }
        }
    }

    fn refresh_existing_mob(&mut self, spawn: &openstory_protocol::MapMobSpawn) -> bool {
        let Some(mob) = self.mobs.get_mut(&spawn.object_id) else {
            return false;
        };
        if mob.control_mode <= 0 {
            mob.x = f64::from(spawn.x);
            mob.y = f64::from(spawn.y);
            mob.body.x = mob.x;
            mob.body.y = mob.y;
            mob.body.previous_x = mob.x;
            mob.body.previous_y = mob.y;
            mob.body.horizontal_speed = 0.0;
            mob.body.vertical_speed = 0.0;
            mob.body.foothold_id = spawn.foothold;
            mob.foothold = spawn.foothold;
            mob.server_flip = spawn.stance & 1 == 0;
            mob.server_stance = spawn.stance;
            let desired = if mob.is_immobilized() {
                MobMotion::Stand
            } else {
                mob_motion_for_stance(spawn.stance)
            };
            mob.set_motion(desired);
        }
        if spawn.control_mode > 0 && mob.control_mode <= 0 {
            mob.control_mode = spawn.control_mode;
            mob.control_acked = false;
        } else if spawn.control_mode > 0 {
            mob.control_mode = spawn.control_mode;
        }
        mob.dying = false;
        mob.fade_remaining = None;
        mob.fade_in_remaining = None;
        if mob.motion == MobMotion::Die {
            mob.set_motion(MobMotion::Stand);
            mob.refresh_server_stance();
        }
        true
    }

    fn kill_mob(&mut self, object_id: i32, animation: i8) {
        if animation == 0 {
            self.mobs.remove(&object_id);
            return;
        }
        let Some(mob) = self.mobs.get_mut(&object_id) else {
            return;
        };
        match animation {
            1 => {
                // The legacy client falls back to a stance the mob has when its
                // NX archive has no DIE clip, then keeps it until server removal.
                mob.set_motion(MobMotion::Die);
                mob.dying = true;
            }
            2 => {
                mob.dying = true;
                let opacity = if let Some(remaining) = mob.fade_remaining {
                    remaining.as_secs_f32() / MOB_FADE_DURATION.as_secs_f32()
                } else if let Some(remaining) = mob.fade_in_remaining {
                    1.0 - remaining.as_secs_f32() / MOB_FADE_DURATION.as_secs_f32()
                } else {
                    1.0
                }
                .clamp(0.0, 1.0);
                mob.fade_remaining = Some(MOB_FADE_DURATION.mul_f32(opacity));
                mob.fade_in_remaining = None;
            }
            _ => {}
        }
    }

    fn remote_summary(&self, id: i32) -> Option<String> {
        let player = self.remote_players.get(&id)?;
        Some(format!(
            "{} id={} level={} job={} at ({:.1},{:.1}) from ({:.1},{:.1}) foothold={} stance={} skin={} face={} hair={}",
            player.name,
            id,
            player.level,
            player.job,
            player.x,
            player.y,
            player.previous_x,
            player.previous_y,
            player.foothold,
            player.stance,
            player.look.skin,
            player.look.face_id,
            player.look.hair_id,
        ))
    }

    fn first_frame_for_motion(&self, motion: AvatarMotion) -> Option<usize> {
        self.avatar_frames
            .iter()
            .position(|frame| frame.motion == motion)
            .or_else(|| {
                let fallback = match motion {
                    AvatarMotion::Stand => AvatarMotion::Walk,
                    AvatarMotion::Sit => AvatarMotion::Stand,
                    AvatarMotion::Alert | AvatarMotion::Crouch | AvatarMotion::Swim => {
                        AvatarMotion::Stand
                    }
                    AvatarMotion::Walk
                    | AvatarMotion::Jump
                    | AvatarMotion::Ladder
                    | AvatarMotion::Rope => AvatarMotion::Stand,
                    AvatarMotion::Attack(_) => AvatarMotion::Stand,
                };
                self.avatar_frames
                    .iter()
                    .position(|frame| frame.motion == fallback)
            })
            .or_else(|| (!self.avatar_frames.is_empty()).then_some(0))
    }

    fn refresh_avatar_draws(&self, draws: &mut [(usize, SpriteRect)]) {
        for (frame_index, frame) in self.avatar_frames.iter().enumerate() {
            let active = self.active_avatar_frame == Some(frame_index);
            for sprite in &frame.sprites {
                let Some(index) = sprite.draw_index else {
                    continue;
                };
                let Some((_, rect)) = draws.get_mut(index) else {
                    continue;
                };
                *rect = sprite.rect;
                if active && self.facing_right {
                    rect.x = 1024.0 - (rect.x + rect.width);
                }
                if !active {
                    rect.width = 0.0;
                    rect.height = 0.0;
                }
            }
        }
    }
}

fn rounded_i16(value: f64) -> i16 {
    value
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn advance_portal_animation(
    frames: &[PortalFrameRuntime],
    frame_index: &mut usize,
    elapsed: &mut Duration,
) {
    if frames.len() < 2 {
        return;
    }
    *elapsed = elapsed.saturating_add(Duration::from_millis(8));
    while let Some(frame) = frames.get(*frame_index) {
        if *elapsed < frame.delay {
            break;
        }
        *elapsed -= frame.delay;
        *frame_index = (*frame_index + 1) % frames.len();
    }
}

fn portal_bounds_contains(portal: &MapPortalRuntime, x: f64, y: f64) -> bool {
    (-25.0..=25.0).contains(&(x - portal.x)) && (-100.0..=25.0).contains(&(y - portal.y))
}

fn nearby_map_seat(seats: &[(i16, i16)], player: &PhysicsBody) -> Option<(i16, i16)> {
    let player_x = rounded_i16(player.x);
    let player_y = rounded_i16(player.y);
    seats
        .iter()
        .copied()
        .find(|(x, y)| player_x.abs_diff(*x) <= 10 && player_y.abs_diff(*y) <= 10)
}

fn place_player_on_map_seat(
    seat: (i16, i16),
    player: &mut PhysicsBody,
    footholds: Option<&FootholdTree>,
) {
    player.x = f64::from(seat.0);
    player.y = f64::from(seat.1);
    player.horizontal_speed = 0.0;
    player.vertical_speed = 0.0;
    player.on_ground = true;
    if let Some((foothold_id, _)) =
        footholds.and_then(|tree| tree.find_below(f64::from(seat.0), f64::from(seat.1)))
    {
        player.foothold_id = foothold_id;
    }
}

fn player_movement(
    player: &PhysicsBody,
    facing_right: bool,
    left: bool,
    right: bool,
    sitting: bool,
    down: bool,
    climbing: Option<bool>,
) -> openstory_protocol::PlayerMovement {
    let base_stance: u8 = if let Some(is_ladder) = climbing {
        if is_ladder { 14 } else { 16 }
    } else if sitting {
        20
    } else if player.on_ground && down {
        10
    } else if !player.on_ground {
        6
    } else if left != right && player.horizontal_speed != 0.0 {
        2
    } else {
        4
    };
    openstory_protocol::PlayerMovement {
        x: rounded_i16(player.x),
        y: rounded_i16(player.y),
        last_x: rounded_i16(player.previous_x),
        last_y: rounded_i16(player.previous_y),
        foothold: player.foothold_id,
        stance: base_stance + u8::from(!facing_right),
        duration: 1,
    }
}
