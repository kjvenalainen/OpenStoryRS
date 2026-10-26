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
                .unwrap_or(100),
            effects_volume: environment_volume("OPENSTORY_SFX_VOLUME")
                .or(effects_volume)
                .or(legacy_effects_volume)
                .unwrap_or(100),
        }
    }

    pub(super) fn save(self) -> Result<PathBuf, String> {
        update_settings_file(
            &[
                ("BGMVolume", self.music_volume.min(100).to_string()),
                ("SFXVolume", self.effects_volume.min(100).to_string()),
            ],
            &["MusicVolume", "EffectsVolume"],
        )
    }
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
