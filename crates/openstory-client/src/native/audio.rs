use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use openstory_assets::{NodeValue, NxArchive, NxNode};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, SpatialPlayer};

const NX_AUDIO_HEADER_LENGTH: usize = 82;
const MAX_AUDIO_CACHE_ENTRIES: usize = 4096;
const MAX_AUDIO_CACHE_BYTES: usize = 64 * 1024 * 1024;
const FULL_VOLUME_RANGE: f64 = 400.0;
const SILENT_RANGE: f64 = 1200.0;
const PAN_RANGE: f64 = 800.0;

/// Native music/effects playback. NX data remains lazy: a sound is read and decoded
/// only when first requested, and the cache is capped so long sessions do not retain
/// the whole Sound.nx archive in memory.
pub(super) struct AudioEngine {
    output: Option<MixerDeviceSink>,
    music: Option<Player>,
    music_path: Option<String>,
    pending_music: Option<(String, Receiver<Result<Option<Arc<[u8]>>, String>>)>,
    music_volume: f32,
    sound_volume: f32,
    listener: (i16, i16),
    audio_cache: HashMap<String, CachedAudio>,
    audio_cache_access_order: VecDeque<(u64, String)>,
    cached_audio_bytes: usize,
    audio_cache_access_clock: u64,
}

struct CachedAudio {
    payload: Arc<[u8]>,
    last_access: u64,
}

impl AudioEngine {
    pub(super) fn new() -> Self {
        let output = match DeviceSinkBuilder::open_default_sink() {
            Ok(output) => Some(output),
            Err(error) => {
                eprintln!("OpenStory audio: output device unavailable: {error}");
                None
            }
        };
        let mut engine = Self {
            output,
            music: None,
            music_path: None,
            pending_music: None,
            music_volume: 1.0,
            sound_volume: 1.0,
            listener: (0, 0),
            audio_cache: HashMap::new(),
            audio_cache_access_order: VecDeque::new(),
            cached_audio_bytes: 0,
            audio_cache_access_clock: 0,
        };
        engine.set_music_volume(configured_volume("OPENSTORY_BGM_VOLUME"));
        engine.set_sound_volume(configured_volume("OPENSTORY_SFX_VOLUME"));
        engine
    }

    pub(super) fn play_music(&mut self, archive: &NxArchive, path: &str) {
        let cache_key = path.to_lowercase();
        if self
            .pending_music
            .as_ref()
            .is_some_and(|(pending_path, _)| pending_path != &cache_key)
        {
            self.pending_music = None;
        }
        if self.music_path.as_deref() == Some(cache_key.as_str()) {
            return;
        }
        if self
            .pending_music
            .as_ref()
            .is_some_and(|(pending_path, _)| pending_path == &cache_key)
        {
            return;
        }
        if let Some(payload) = self.cached_audio_payload(&cache_key) {
            self.start_music_with_payload(cache_key, payload);
            return;
        }
        let archive = archive.clone();
        let path = path.to_owned();
        let (sender, receiver) = mpsc::channel();
        match std::thread::Builder::new()
            .name("map-music-load".to_owned())
            .spawn(move || {
                let _ = sender.send(read_audio_payload(&archive, &path));
            }) {
            Ok(_) => self.pending_music = Some((cache_key, receiver)),
            Err(error) => eprintln!("OpenStory audio: could not start music load: {error}"),
        }
    }

    pub(super) fn process_pending_music(&mut self) {
        let ready =
            self.pending_music
                .as_ref()
                .and_then(|(path, receiver)| match receiver.try_recv() {
                    Ok(result) => Some((path.clone(), result)),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => Some((
                        path.clone(),
                        Err("music loading worker stopped unexpectedly".to_owned()),
                    )),
                });
        let Some((path, result)) = ready else {
            return;
        };
        self.pending_music = None;
        match result {
            Ok(Some(payload)) => {
                self.cache_audio_payload(path.clone(), payload.clone());
                self.start_music_with_payload(path, payload);
            }
            Ok(None) => {}
            Err(error) => eprintln!("OpenStory audio: could not load music {path:?}: {error}"),
        }
    }

    fn start_music_with_payload(&mut self, cache_key: String, payload: Arc<[u8]>) {
        let source = match Decoder::new_looped(Cursor::new(payload)) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("OpenStory audio: could not decode music {cache_key:?}: {error}");
                return;
            }
        };
        let Some(output) = self.output.as_ref() else {
            return;
        };
        let player = Player::connect_new(output.mixer());
        player.set_volume(self.music_volume);
        player.append(source);
        self.music = Some(player);
        self.music_path = Some(cache_key);
    }

    pub(super) fn set_music_volume(&mut self, percent: u8) {
        self.music_volume = f32::from(percent.min(100)) / 100.0;
        if let Some(music) = self.music.as_ref() {
            music.set_volume(self.music_volume);
        }
    }

    pub(super) fn set_sound_volume(&mut self, percent: u8) {
        self.sound_volume = f32::from(percent.min(100)) / 100.0;
    }

    pub(super) fn set_listener(&mut self, position: (i16, i16)) {
        self.listener = position;
    }

    pub(super) fn play_ui_sound(&mut self, archive: &NxArchive, path: &str) {
        let payload = match self.audio_payload(archive, path) {
            Ok(Some(payload)) => payload,
            Ok(None) => return,
            Err(error) => {
                eprintln!("OpenStory audio: could not load sound {path:?}: {error}");
                return;
            }
        };
        let source = match Decoder::try_from(Cursor::new(payload)) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("OpenStory audio: could not decode sound {path:?}: {error}");
                return;
            }
        };
        let Some(output) = self.output.as_ref() else {
            return;
        };
        let player = Player::connect_new(output.mixer());
        player.set_volume(self.sound_volume);
        player.append(source);
        player.detach();
    }

    pub(super) fn play_item_sound(&mut self, archive: &NxArchive, item_id: i32) {
        if item_id <= 0 {
            return;
        }
        let exact_id = format!("{item_id:08}");
        let category_id = format!("{:08}", (item_id / 10_000) * 10_000);
        let candidates = [exact_id, category_id, String::from("02000000")];
        for (index, sound_id) in candidates.iter().enumerate() {
            if candidates[..index].contains(sound_id) {
                continue;
            }
            let path = format!("Item.img/{sound_id}/Use");
            match self.audio_payload(archive, &path) {
                Ok(Some(_)) => {
                    self.play_ui_sound(archive, &path);
                    return;
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("OpenStory audio: could not load item sound {path:?}: {error}");
                }
            }
        }
    }

    pub(super) fn play_field_effect_sound(&mut self, archive: &NxArchive, path: &str) {
        let Some((category, name)) = path.split_once('/') else {
            return;
        };
        let mut candidates = vec![
            format!("Field.img/{category}/{name}"),
            format!("{category}.img/{name}"),
        ];
        if category == "Party1" {
            candidates.push(format!(
                "Game.img/{}",
                if name == "Clear" {
                    "QuestClear"
                } else {
                    "QuestAlert"
                }
            ));
        }
        for candidate in candidates {
            if matches!(resolve_relaxed(archive, &candidate), Ok(Some(_))) {
                self.play_ui_sound(archive, &candidate);
                return;
            }
        }
    }

    pub(super) fn play_world_sound(
        &mut self,
        archive: &NxArchive,
        path: &str,
        position: (i16, i16),
    ) {
        let dx = f64::from(position.0) - f64::from(self.listener.0);
        let dy = f64::from(position.1) - f64::from(self.listener.1);
        let distance = (dx * dx + dy * dy).sqrt();
        if distance >= SILENT_RANGE || self.sound_volume == 0.0 {
            return;
        }
        let attenuation = if distance <= FULL_VOLUME_RANGE {
            1.0
        } else {
            ((SILENT_RANGE - distance) / (SILENT_RANGE - FULL_VOLUME_RANGE)) as f32
        };
        let pan = (dx / PAN_RANGE).clamp(-1.0, 1.0) as f32;
        let payload = match self.audio_payload(archive, path) {
            Ok(Some(payload)) => payload,
            Ok(None) => return,
            Err(error) => {
                eprintln!("OpenStory audio: could not load sound {path:?}: {error}");
                return;
            }
        };
        let source = match Decoder::try_from(Cursor::new(payload)) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("OpenStory audio: could not decode sound {path:?}: {error}");
                return;
            }
        };
        let Some(output) = self.output.as_ref() else {
            return;
        };

        // Encode the legacy horizontal pan in a compact spatial coordinate. Manual
        // distance attenuation matches the old 400/1200 world-unit falloff.
        let emitter = [pan, 0.0, 0.0];
        let player =
            SpatialPlayer::connect_new(output.mixer(), emitter, [-0.1, 0.0, 0.0], [0.1, 0.0, 0.0]);
        player.set_volume(self.sound_volume * attenuation);
        player.append(source);
        player.detach();
    }

    pub(super) fn play_skill_use(
        &mut self,
        archive: &NxArchive,
        skill_id: i32,
        position: (i16, i16),
    ) {
        if skill_id <= 0 {
            return;
        }
        let skill_name = format!("{skill_id:07}");
        self.play_world_sound(archive, &format!("Skill.img/{skill_name}/Use"), position);
    }

    pub(super) fn play_skill_hit(
        &mut self,
        archive: &NxArchive,
        skill_id: i32,
        position: (i16, i16),
    ) {
        if skill_id <= 0 {
            return;
        }
        let skill_name = format!("{skill_id:07}");
        self.play_world_sound(archive, &format!("Skill.img/{skill_name}/Hit"), position);
    }

    pub(super) fn play_weapon_attack(
        &mut self,
        character_archive: &NxArchive,
        sound_archive: &NxArchive,
        weapon_id: i32,
        degenerate: bool,
        position: (i16, i16),
    ) {
        if weapon_id <= 0 {
            return;
        }
        let weapon_type = weapon_id / 10_000;
        let sfx = character_archive
            .root()
            .resolve(&format!("Weapon/0{weapon_id}.img/info/sfx"))
            .ok()
            .flatten()
            .and_then(|node| match node.value().ok()? {
                NodeValue::String(value) if !value.trim().is_empty() => Some(value),
                _ => None,
            })
            .unwrap_or_else(|| default_weapon_sfx(weapon_type).to_owned());
        let base_path = format!("Weapon.img/{sfx}");
        let attack2_path = format!("{base_path}/Attack2");
        let has_attack2 = degenerate
            && resolve_relaxed(sound_archive, &attack2_path)
                .ok()
                .flatten()
                .and_then(|node| node.value().ok())
                .is_some_and(|value| matches!(value, NodeValue::Audio { .. }));
        let sound_name = if has_attack2 { "Attack2" } else { "Attack" };
        self.play_world_sound(
            sound_archive,
            &format!("{base_path}/{sound_name}"),
            position,
        );
    }

    pub(super) fn play_mob_hit(&mut self, archive: &NxArchive, mob_id: i32, position: (i16, i16)) {
        let Some(path) = mob_sound_path(mob_id, "Damage") else {
            return;
        };
        self.play_world_sound(archive, &path, position);
    }

    pub(super) fn play_mob_death(
        &mut self,
        archive: &NxArchive,
        mob_id: i32,
        position: (i16, i16),
    ) {
        let Some(path) = mob_sound_path(mob_id, "Die") else {
            return;
        };
        self.play_world_sound(archive, &path, position);
    }

    pub(super) fn play_reactor_hit(&mut self, archive: &NxArchive, reactor_id: i32, state: i8) {
        let Some(paths) = reactor_hit_sound_candidates(reactor_id, state) else {
            return;
        };
        self.play_ui_sound_candidates(archive, &paths);
    }

    pub(super) fn play_reactor_break(&mut self, archive: &NxArchive, reactor_id: i32, state: i8) {
        let Some(paths) = reactor_break_sound_candidates(reactor_id, state) else {
            return;
        };
        self.play_ui_sound_candidates(archive, &paths);
    }

    fn audio_payload(
        &mut self,
        archive: &NxArchive,
        path: &str,
    ) -> Result<Option<Arc<[u8]>>, String> {
        let key = path.to_lowercase();
        if let Some(payload) = self.cached_audio_payload(&key) {
            return Ok(Some(payload));
        }
        let Some(payload) = read_audio_payload(archive, path)? else {
            return Ok(None);
        };
        self.cache_audio_payload(key, payload.clone());
        Ok(Some(payload))
    }

    fn cached_audio_payload(&mut self, key: &str) -> Option<Arc<[u8]>> {
        if !self.audio_cache.contains_key(key) {
            return None;
        }
        let last_access = self.next_audio_cache_access();
        let payload = self.audio_cache.get_mut(key).map(|cached| {
            cached.last_access = last_access;
            cached.payload.clone()
        });
        if payload.is_some() {
            self.audio_cache_access_order
                .push_back((last_access, key.to_owned()));
            self.compact_audio_cache_access_order();
        }
        payload
    }

    fn cache_audio_payload(&mut self, key: String, payload: Arc<[u8]>) {
        if payload.len() <= MAX_AUDIO_CACHE_BYTES {
            while self.audio_cache.len() >= MAX_AUDIO_CACHE_ENTRIES
                || self.cached_audio_bytes.saturating_add(payload.len()) > MAX_AUDIO_CACHE_BYTES
            {
                let Some((access, evicted_key)) = self.audio_cache_access_order.pop_front() else {
                    break;
                };
                if !self
                    .audio_cache
                    .get(&evicted_key)
                    .is_some_and(|cached| cached.last_access == access)
                {
                    continue;
                }
                if let Some(evicted) = self.audio_cache.remove(&evicted_key) {
                    self.cached_audio_bytes = self
                        .cached_audio_bytes
                        .saturating_sub(evicted.payload.len());
                }
            }
        }
        if payload.len() <= MAX_AUDIO_CACHE_BYTES
            && self.audio_cache.len() < MAX_AUDIO_CACHE_ENTRIES
            && self.cached_audio_bytes.saturating_add(payload.len()) <= MAX_AUDIO_CACHE_BYTES
        {
            let last_access = self.next_audio_cache_access();
            self.cached_audio_bytes += payload.len();
            self.audio_cache.insert(
                key.clone(),
                CachedAudio {
                    payload: payload.clone(),
                    last_access,
                },
            );
            self.audio_cache_access_order.push_back((last_access, key));
            self.compact_audio_cache_access_order();
        }
    }

    fn play_ui_sound_candidates(&mut self, archive: &NxArchive, paths: &[String]) {
        for path in paths {
            match self.audio_payload(archive, path) {
                Ok(Some(_)) => {
                    self.play_ui_sound(archive, path);
                    return;
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("OpenStory audio: could not load sound {path:?}: {error}");
                }
            }
        }
    }

    fn compact_audio_cache_access_order(&mut self) {
        if self.audio_cache_access_order.len() <= MAX_AUDIO_CACHE_ENTRIES.saturating_mul(2) {
            return;
        }
        self.audio_cache_access_order.retain(|(access, key)| {
            self.audio_cache
                .get(key)
                .is_some_and(|cached| cached.last_access == *access)
        });
    }

    fn next_audio_cache_access(&mut self) -> u64 {
        if self.audio_cache_access_clock == u64::MAX {
            // Rebase while retaining the order of the least to most recent entries.
            let mut recency: Vec<_> = self
                .audio_cache
                .iter()
                .map(|(key, cached)| (key.clone(), cached.last_access))
                .collect();
            recency.sort_unstable_by_key(|(_, last_access)| *last_access);
            self.audio_cache_access_order.clear();
            for (access, (key, _)) in recency.into_iter().enumerate() {
                if let Some(cached) = self.audio_cache.get_mut(&key) {
                    cached.last_access = access as u64;
                }
                self.audio_cache_access_order
                    .push_back((access as u64, key));
            }
            self.audio_cache_access_clock = self.audio_cache.len() as u64;
        }
        self.audio_cache_access_clock += 1;
        self.audio_cache_access_clock
    }
}

fn configured_volume(variable: &str) -> u8 {
    std::env::var(variable)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|volume| *volume <= 100)
        .unwrap_or(100)
}

fn mob_sound_path(mob_id: i32, sound_name: &str) -> Option<String> {
    (mob_id > 0).then(|| format!("Mob.img/{mob_id:07}/{sound_name}"))
}

fn default_weapon_sfx(weapon_type: i32) -> &'static str {
    match weapon_type {
        130 | 140 => "swordL",
        133 => "swordS",
        131 | 141 => "axe",
        132 | 137 | 138 | 142 => "mace",
        143 => "spear",
        144 => "poleArm",
        145 => "bow",
        146 => "cBow",
        147 => "tGlove",
        148 => "knuckle",
        149 => "gun",
        _ => "swordL",
    }
}

fn reactor_hit_sound_candidates(reactor_id: i32, state: i8) -> Option<[String; 2]> {
    (reactor_id > 0 && state >= 0).then(|| {
        [
            format!("Reactor.img/{reactor_id}/{state}/Hit"),
            format!("Reactor.img/{reactor_id}/hit"),
        ]
    })
}

fn reactor_break_sound_candidates(reactor_id: i32, state: i8) -> Option<[String; 2]> {
    (reactor_id > 0).then(|| {
        [
            format!("Reactor.img/{reactor_id}/break"),
            format!("Reactor.img/{reactor_id}/{state}/Break"),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::{
        default_weapon_sfx, mob_sound_path, reactor_break_sound_candidates,
        reactor_hit_sound_candidates,
    };

    #[test]
    fn mob_sound_paths_use_the_numeric_archive_node() {
        assert_eq!(
            mob_sound_path(100_100, "Die").as_deref(),
            Some("Mob.img/0100100/Die")
        );
        assert_eq!(mob_sound_path(0, "Die"), None);
    }

    #[test]
    fn weapon_sound_defaults_match_legacy_weapon_categories() {
        for (weapon_type, expected) in [
            (130, "swordL"),
            (133, "swordS"),
            (145, "bow"),
            (146, "cBow"),
            (147, "tGlove"),
            (149, "gun"),
        ] {
            assert_eq!(default_weapon_sfx(weapon_type), expected);
        }
    }

    #[test]
    fn reactor_hit_paths_match_state_scoped_v83_audio_nodes() {
        assert_eq!(
            reactor_hit_sound_candidates(2000, 3),
            Some([
                String::from("Reactor.img/2000/3/Hit"),
                String::from("Reactor.img/2000/hit"),
            ])
        );
        assert_eq!(reactor_hit_sound_candidates(0, 0), None);
        assert_eq!(reactor_hit_sound_candidates(2000, -1), None);
    }

    #[test]
    fn reactor_break_paths_cover_legacy_and_state_scoped_audio_nodes() {
        assert_eq!(
            reactor_break_sound_candidates(2000, 3),
            Some([
                String::from("Reactor.img/2000/break"),
                String::from("Reactor.img/2000/3/Break"),
            ])
        );
        assert_eq!(reactor_break_sound_candidates(0, 3), None);
    }
}

fn read_audio_payload(archive: &NxArchive, path: &str) -> Result<Option<Arc<[u8]>>, String> {
    let Some(node) = resolve_relaxed(archive, path)? else {
        return Ok(None);
    };
    let Some(bytes) = node.audio_bytes().map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    if bytes.len() <= NX_AUDIO_HEADER_LENGTH {
        return Ok(None);
    }
    Ok(Some(Arc::from(&bytes[NX_AUDIO_HEADER_LENGTH..])))
}

fn resolve_relaxed(archive: &NxArchive, path: &str) -> Result<Option<NxNode>, String> {
    if let Some(node) = archive
        .root()
        .resolve(path)
        .map_err(|error| error.to_string())?
    {
        return Ok(Some(node));
    }

    let mut current = archive.root();
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        let exact = current.child(segment).map_err(|error| error.to_string())?;
        let Some(next) = exact.or_else(|| {
            current.children().ok()?.into_iter().find(|candidate| {
                candidate
                    .name()
                    .is_ok_and(|name| name.eq_ignore_ascii_case(segment))
            })
        }) else {
            return Ok(None);
        };
        current = next;
    }
    Ok(Some(current))
}
