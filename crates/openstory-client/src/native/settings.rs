use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioSettings {
    pub music_volume: u8,
    pub effects_volume: u8,
}

impl AudioSettings {
    pub(super) fn load() -> Self {
        let mut music_volume = None;
        let mut effects_volume = None;
        let mut legacy_music_volume = None;
        let mut legacy_effects_volume = None;
        if let Some(path) = settings_path() {
            if let Ok(contents) = fs::read_to_string(path) {
                for line in contents.lines() {
                    let Some((name, value)) = line.split_once('=') else {
                        continue;
                    };
                    let value = value
                        .trim()
                        .parse::<u8>()
                        .ok()
                        .filter(|value| *value <= 100);
                    match name.trim() {
                        "BGMVolume" => music_volume = value,
                        "SFXVolume" => effects_volume = value,
                        "MusicVolume" => legacy_music_volume = value,
                        "EffectsVolume" => legacy_effects_volume = value,
                        _ => {}
                    }
                }
            }
        }
        Self {
            music_volume: environment_volume("OPENSTORY_BGM_VOLUME")
                .or(music_volume)
                .or(legacy_music_volume)
                .unwrap_or(50),
            effects_volume: environment_volume("OPENSTORY_SFX_VOLUME")
                .or(effects_volume)
                .or(legacy_effects_volume)
                .unwrap_or(50),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GameplaySettings {
    pub screen_shake_enabled: bool,
    pub graphics_quality: u8,
    pub effects_quality: u8,
    pub hp_warning: u8,
    pub mp_warning: u8,
}

impl GameplaySettings {
    pub(super) fn load() -> Self {
        let mut settings = Self::default();
        let Some(path) = settings_path() else {
            return settings;
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return settings;
        };
        for line in contents.lines() {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match name.trim() {
                "ScreenShake" => {
                    if let Some(enabled) = parse_bool(value) {
                        settings.screen_shake_enabled = enabled;
                    }
                }
                "GraphicsQuality" => {
                    if let Some(quality) = parse_percent(value) {
                        settings.graphics_quality = quality;
                    }
                }
                "EffectsQuality" => {
                    if let Some(quality) = parse_percent(value) {
                        settings.effects_quality = quality;
                    }
                }
                "HPWarning" => {
                    if let Some(threshold) = parse_percent(value) {
                        settings.hp_warning = threshold;
                    }
                }
                "MPWarning" => {
                    if let Some(threshold) = parse_percent(value) {
                        settings.mp_warning = threshold;
                    }
                }
                _ => {}
            }
        }
        settings
    }

    pub(super) fn hp_warning_opacity(self, hp: u16, max_hp: u16) -> Option<f32> {
        warning_opacity(hp, max_hp, self.hp_warning, 0.15, 0.10)
    }

    pub(super) fn mp_warning_opacity(self, mp: u16, max_mp: u16) -> Option<f32> {
        warning_opacity(mp, max_mp, self.mp_warning, 0.12, 0.08)
    }
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            screen_shake_enabled: true,
            graphics_quality: 50,
            effects_quality: 50,
            hp_warning: 50,
            mp_warning: 50,
        }
    }
}

pub(super) fn save(audio: AudioSettings, gameplay: GameplaySettings) -> Result<PathBuf, String> {
    update_settings_file(
        &[
            ("BGMVolume", audio.music_volume.min(100).to_string()),
            ("SFXVolume", audio.effects_volume.min(100).to_string()),
            ("ScreenShake", gameplay.screen_shake_enabled.to_string()),
            (
                "GraphicsQuality",
                gameplay.graphics_quality.min(100).to_string(),
            ),
            (
                "EffectsQuality",
                gameplay.effects_quality.min(100).to_string(),
            ),
            ("HPWarning", gameplay.hp_warning.min(100).to_string()),
            ("MPWarning", gameplay.mp_warning.min(100).to_string()),
        ],
        &["MusicVolume", "EffectsVolume"],
    )
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct LoginPreferences {
    pub save_account_id: bool,
    pub account_id: String,
}

impl LoginPreferences {
    pub(super) fn load() -> Self {
        let Some(path) = settings_path() else {
            return Self::default();
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };
        let mut preferences = Self::default();
        for line in contents.lines() {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            match name.trim() {
                "SaveLogin" => {
                    preferences.save_account_id = matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "true" | "1" | "yes"
                    );
                }
                "Account" => {
                    preferences.account_id = value.trim().chars().take(12).collect();
                }
                _ => {}
            }
        }
        preferences
    }

    pub(super) fn save(&self) -> Result<PathBuf, String> {
        update_settings_file(
            &[
                ("SaveLogin", self.save_account_id.to_string()),
                (
                    "Account",
                    if self.save_account_id {
                        self.account_id.clone()
                    } else {
                        String::new()
                    },
                ),
            ],
            &["Password"],
        )
    }
}

pub(super) fn load_quickslot_keys() -> Option<[u8; 8]> {
    let path = settings_path()?;
    let contents = fs::read_to_string(path).ok()?;
    let value = contents.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == "QuickSlotKeys").then_some(value.trim())
    })?;
    parse_quickslot_keys(value)
}

pub(super) fn save_quickslot_keys(keys: [u8; 8]) -> Result<PathBuf, String> {
    let value = keys.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
    update_settings_file(&[("QuickSlotKeys", value)], &[])
}

fn parse_quickslot_keys(value: &str) -> Option<[u8; 8]> {
    let mut keys = [0; 8];
    let mut seen = [false; 90];
    let mut fields = value.split(',');
    for key in &mut keys {
        let parsed = fields.next()?.parse::<u8>().ok()?;
        if !(1..90).contains(&parsed) || seen[usize::from(parsed)] {
            return None;
        }
        seen[usize::from(parsed)] = true;
        *key = parsed;
    }
    fields.next().is_none().then_some(keys)
}

fn update_settings_file(
    updates: &[(&str, String)],
    remove_keys: &[&str],
) -> Result<PathBuf, String> {
    let path = settings_path()
        .ok_or_else(|| "no per-user settings directory is available on this platform".to_owned())?;
    let parent = path
        .parent()
        .ok_or_else(|| "settings file has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create settings directory: {error}"))?;

    let contents = fs::read_to_string(&path).unwrap_or_default();
    let mut emitted = vec![false; updates.len()];
    let mut output = Vec::new();
    for line in contents.lines() {
        let key = line
            .split_once('=')
            .map(|(key, _)| key.trim())
            .unwrap_or_default();
        if key == "Password" || remove_keys.contains(&key) {
            continue;
        }
        if let Some((index, (name, value))) = updates
            .iter()
            .enumerate()
            .find(|(_, (name, _))| *name == key)
        {
            if !emitted[index] {
                output.push(format!("{name} = {value}"));
                emitted[index] = true;
            }
        } else {
            output.push(line.to_owned());
        }
    }
    for (index, (name, value)) in updates.iter().enumerate() {
        if !emitted[index] {
            output.push(format!("{name} = {value}"));
        }
    }
    let mut serialized = output.join("\n");
    serialized.push('\n');
    fs::write(&path, serialized)
        .map_err(|error| format!("could not write settings file: {error}"))?;
    Ok(path)
}

fn environment_volume(name: &str) -> Option<u8> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|volume| *volume <= 100)
}

fn parse_percent(value: &str) -> Option<u8> {
    value.parse::<u8>().ok().filter(|value| *value <= 100)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn warning_opacity(
    current: u16,
    maximum: u16,
    threshold: u8,
    base: f32,
    range: f32,
) -> Option<f32> {
    if maximum == 0 || threshold == 0 {
        return None;
    }
    let percent = u32::from(current) * 100 / u32::from(maximum);
    if percent > u32::from(threshold) {
        return None;
    }
    let intensity = 1.0 - percent as f32 / f32::from(threshold);
    Some(base + range * intensity.clamp(0.0, 1.0))
}

fn settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return Some(
            PathBuf::from(std::env::var_os("HOME")?)
                .join("Library/Application Support/OpenStoryRS/settings.ini"),
        );
    }

    #[cfg(target_os = "windows")]
    {
        return Some(PathBuf::from(std::env::var_os("APPDATA")?).join("OpenStoryRS/settings.ini"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(config_dir) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(config_dir).join("openstoryrs/settings.ini"));
        }
        return Some(
            PathBuf::from(std::env::var_os("HOME")?).join(".config/openstoryrs/settings.ini"),
        );
    }

    #[allow(unreachable_code)]
    None
}
