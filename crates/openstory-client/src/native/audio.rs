use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::sync::Arc;

use openstory_assets::{NxArchive, NxNode};
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
    music_volume: f32,
    sound_volume: f32,
    listener: (i16, i16),
    audio_cache: HashMap<String, Arc<[u8]>>,
    audio_cache_order: VecDeque<String>,
    cached_audio_bytes: usize,
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
            music_volume: 1.0,
            sound_volume: 1.0,
            listener: (0, 0),
            audio_cache: HashMap::new(),
            audio_cache_order: VecDeque::new(),
            cached_audio_bytes: 0,
        };
        engine.set_music_volume(configured_volume("OPENSTORY_BGM_VOLUME"));
        engine.set_sound_volume(configured_volume("OPENSTORY_SFX_VOLUME"));
        engine
    }

    pub(super) fn play_music(&mut self, archive: &NxArchive, path: &str) {
        let cache_key = path.to_lowercase();
        if self.music_path.as_deref() == Some(cache_key.as_str()) {
            return;
        }
        let payload = match self.audio_payload(archive, path) {
            Ok(Some(payload)) => payload,
            Ok(None) => return,
            Err(error) => {
                eprintln!("OpenStory audio: could not load music {path:?}: {error}");
                return;
            }
        };
        let source = match Decoder::new_looped(Cursor::new(payload)) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("OpenStory audio: could not decode music {path:?}: {error}");
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

    fn audio_payload(
        &mut self,
        archive: &NxArchive,
        path: &str,
    ) -> Result<Option<Arc<[u8]>>, String> {
        let key = path.to_lowercase();
        if let Some(payload) = self.audio_cache.get(&key) {
            return Ok(Some(payload.clone()));
        }
        let Some(node) = resolve_relaxed(archive, path)? else {
            return Ok(None);
        };
        let Some(bytes) = node.audio_bytes().map_err(|error| error.to_string())? else {
            return Ok(None);
        };
        if bytes.len() <= NX_AUDIO_HEADER_LENGTH {
            return Ok(None);
        }
        let payload: Arc<[u8]> = Arc::from(&bytes[NX_AUDIO_HEADER_LENGTH..]);
        if payload.len() <= MAX_AUDIO_CACHE_BYTES {
            while self.audio_cache.len() >= MAX_AUDIO_CACHE_ENTRIES
                || self.cached_audio_bytes.saturating_add(payload.len()) > MAX_AUDIO_CACHE_BYTES
            {
                let Some(evicted_key) = self.audio_cache_order.pop_front() else {
                    break;
                };
                if let Some(evicted) = self.audio_cache.remove(&evicted_key) {
                    self.cached_audio_bytes = self.cached_audio_bytes.saturating_sub(evicted.len());
                }
            }
        }
        if payload.len() <= MAX_AUDIO_CACHE_BYTES
            && self.audio_cache.len() < MAX_AUDIO_CACHE_ENTRIES
            && self.cached_audio_bytes.saturating_add(payload.len()) <= MAX_AUDIO_CACHE_BYTES
        {
            self.cached_audio_bytes += payload.len();
            self.audio_cache_order.push_back(key.clone());
            self.audio_cache.insert(key, payload.clone());
        }
        Ok(Some(payload))
    }
}

fn configured_volume(variable: &str) -> u8 {
    std::env::var(variable)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|volume| *volume <= 100)
        .unwrap_or(100)
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
