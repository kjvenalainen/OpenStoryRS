use super::{BlendMode, GpuTexture, SpriteDraw};

const PARTICLE_COUNT: usize = 120;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    None,
    Particles,
    Fog,
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    x: f32,
    y: f32,
    speed: f32,
    drift: f32,
    alpha: f32,
    scale: f32,
}

pub(super) struct WeatherOverlay {
    mode: Mode,
    width: f32,
    height: f32,
    particles: Vec<Particle>,
    rng: u32,
    fog_offset: f32,
}

impl Default for WeatherOverlay {
    fn default() -> Self {
        Self {
            mode: Mode::None,
            width: 1024.0,
            height: 768.0,
            particles: Vec::with_capacity(PARTICLE_COUNT),
            rng: 0x6d2b_79f5,
            fog_offset: 0.0,
        }
    }
}

impl WeatherOverlay {
    pub(super) fn start(&mut self, path: &str, width: u32, height: u32) {
        self.width = width.max(1) as f32;
        self.height = height.max(1) as f32;
        self.particles.clear();
        self.fog_offset = 0.0;
        self.rng = path.bytes().fold(0x6d2b_79f5, |state, byte| {
            state.rotate_left(5) ^ u32::from(byte)
        });
        if self.rng == 0 {
            self.rng = 0x6d2b_79f5;
        }

        let lowercase_path = path.to_ascii_lowercase();
        self.mode = if ["ghost", "fog", "mist"]
            .iter()
            .any(|marker| lowercase_path.contains(marker))
        {
            Mode::Fog
        } else {
            Mode::Particles
        };

        if self.mode == Mode::Particles {
            self.particles.reserve(PARTICLE_COUNT);
            for _ in 0..PARTICLE_COUNT {
                let particle = self.spawn_particle(true);
                self.particles.push(particle);
            }
        }
    }

    pub(super) fn clear(&mut self) {
        self.mode = Mode::None;
        self.particles.clear();
        self.fog_offset = 0.0;
    }

    pub(super) fn advance(&mut self, updates: u8, texture_width: u32) {
        if updates == 0 {
            return;
        }
        let steps = f32::from(updates);
        match self.mode {
            Mode::None => {}
            Mode::Fog => {
                let width = texture_width as f32;
                if width > 0.0 {
                    self.fog_offset = (self.fog_offset + 0.5 * steps) % width;
                }
            }
            Mode::Particles => {
                for index in 0..self.particles.len() {
                    let should_respawn = {
                        let particle = &mut self.particles[index];
                        particle.y += particle.speed * steps;
                        particle.x += particle.drift * steps;
                        particle.y > self.height + 20.0
                            || particle.x < -20.0
                            || particle.x > self.width + 20.0
                    };
                    if should_respawn {
                        let particle = self.spawn_particle(false);
                        self.particles[index] = particle;
                    }
                }
            }
        }
    }

    pub(super) fn append_draws<'a>(
        &self,
        texture: &'a GpuTexture,
        draws: &mut Vec<SpriteDraw<'a>>,
    ) {
        let texture_width = texture.width() as f32;
        let texture_height = texture.height() as f32;
        if texture_width <= 0.0 || texture_height <= 0.0 {
            return;
        }

        match self.mode {
            Mode::None => {}
            Mode::Fog => {
                let mut y = 0.0;
                while y < self.height {
                    let mut x = -self.fog_offset;
                    while x < self.width {
                        draws.push(weather_sprite(
                            texture,
                            x,
                            y,
                            texture_width,
                            texture_height,
                            0.45,
                        ));
                        x += texture_width;
                    }
                    y += texture_height;
                }
            }
            Mode::Particles => {
                for particle in &self.particles {
                    draws.push(weather_sprite(
                        texture,
                        particle.x,
                        particle.y,
                        texture_width * particle.scale,
                        texture_height * particle.scale,
                        particle.alpha,
                    ));
                }
            }
        }
    }

    fn spawn_particle(&mut self, spread_vertically: bool) -> Particle {
        let x = self.random_range(0.0, self.width);
        let y = if spread_vertically {
            self.random_range(0.0, self.height)
        } else {
            -20.0
        };
        Particle {
            x,
            y,
            speed: self.random_range(0.3, 1.0),
            drift: self.random_range(-0.3, 0.3),
            alpha: self.random_range(0.4, 0.9),
            scale: self.random_range(0.3, 0.6),
        }
    }

    fn random_range(&mut self, min: f32, max: f32) -> f32 {
        let mut state = self.rng;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.rng = state;
        let unit = (state as f64 / u32::MAX as f64) as f32;
        min + (max - min) * unit
    }
}

fn weather_sprite<'a>(
    texture: &'a GpuTexture,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> SpriteDraw<'a> {
    SpriteDraw {
        texture,
        x,
        y,
        width,
        height,
        uv: [0.0, 0.0, 1.0, 1.0],
        tint: [1.0, 1.0, 1.0],
        blend_mode: BlendMode::Alpha,
        flip_x: false,
        opacity,
    }
}
