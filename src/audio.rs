//! Tiny synthesizer: every sound effect is generated in code as a WAV
//! buffer at startup, so the game needs no audio files.

use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

use crate::game::Event;

pub const SAMPLE_RATE: u32 = 22_050;
/// Playback volume, also used when mixing the sound track of a video.
pub const VOLUME: f32 = 0.6;

#[derive(Clone, Copy)]
enum Wave {
    Square,
    Triangle,
    Noise,
}

/// One tone with a linear frequency sweep and a short attack/decay envelope.
#[derive(Clone, Copy)]
struct Note {
    wave: Wave,
    f0: f32,
    f1: f32,
    secs: f32,
    vol: f32,
}

const fn note(wave: Wave, f0: f32, f1: f32, secs: f32, vol: f32) -> Note {
    Note {
        wave,
        f0,
        f1,
        secs,
        vol,
    }
}

fn synth(notes: &[Note]) -> Vec<f32> {
    let mut out = Vec::new();
    let mut noise: u32 = 0x1234_5678;
    for n in notes {
        let len = (n.secs * SAMPLE_RATE as f32) as usize;
        let attack = (0.004 * SAMPLE_RATE as f32) as usize;
        let mut phase = 0.0f32;
        for i in 0..len {
            let t = i as f32 / len as f32;
            let freq = n.f0 + (n.f1 - n.f0) * t;
            phase = (phase + freq / SAMPLE_RATE as f32).fract();
            let raw = match n.wave {
                Wave::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Noise => {
                    noise ^= noise << 13;
                    noise ^= noise >> 17;
                    noise ^= noise << 5;
                    (noise as f32 / u32::MAX as f32) * 2.0 - 1.0
                }
            };
            let env = if i < attack {
                i as f32 / attack as f32
            } else {
                (1.0 - t).powf(1.5)
            };
            out.push(raw * env * n.vol);
        }
    }
    out
}

/// Encodes mono samples in [-1, 1] as a 16-bit PCM WAV file.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    b.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    b.extend_from_slice(&2u16.to_le_bytes()); // block align
    b.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

#[derive(Clone, Copy)]
enum Sfx {
    Launch,
    Wall,
    Paddle,
    BrickHit,
    BrickBreak,
    LifeLost,
    LevelClear,
    GameOver,
    Victory,
    Stone,
    Bonus,
    BottomWall,
    WallBreak,
}

const ALL: [Sfx; 13] = [
    Sfx::Launch,
    Sfx::Wall,
    Sfx::Paddle,
    Sfx::BrickHit,
    Sfx::BrickBreak,
    Sfx::LifeLost,
    Sfx::LevelClear,
    Sfx::GameOver,
    Sfx::Victory,
    Sfx::Stone,
    Sfx::Bonus,
    Sfx::BottomWall,
    Sfx::WallBreak,
];

fn recipe(sfx: Sfx) -> Vec<Note> {
    use Wave::*;
    match sfx {
        Sfx::Launch => vec![note(Square, 330.0, 660.0, 0.09, 0.25)],
        Sfx::Wall => vec![note(Triangle, 260.0, 240.0, 0.05, 0.45)],
        Sfx::Paddle => vec![note(Square, 420.0, 520.0, 0.07, 0.3)],
        Sfx::BrickHit => vec![note(Triangle, 180.0, 150.0, 0.08, 0.6)],
        Sfx::BrickBreak => vec![
            note(Square, 780.0, 1040.0, 0.05, 0.22),
            note(Noise, 0.0, 0.0, 0.05, 0.12),
        ],
        Sfx::LifeLost => vec![note(Square, 440.0, 70.0, 0.55, 0.3)],
        Sfx::LevelClear => [523.0, 659.0, 784.0, 1047.0]
            .iter()
            .map(|&f| note(Square, f, f, 0.1, 0.25))
            .collect(),
        Sfx::GameOver => [392.0, 330.0, 262.0, 196.0]
            .iter()
            .map(|&f| note(Triangle, f, f * 0.97, 0.2, 0.55))
            .collect(),
        Sfx::Stone => vec![
            note(Square, 1200.0, 1150.0, 0.03, 0.15),
            note(Triangle, 600.0, 580.0, 0.06, 0.35),
        ],
        Sfx::Bonus => [660.0, 880.0, 1320.0]
            .iter()
            .map(|&f| note(Square, f, f, 0.05, 0.22))
            .collect(),
        Sfx::BottomWall => vec![note(Triangle, 140.0, 110.0, 0.09, 0.7)],
        Sfx::WallBreak => vec![
            note(Noise, 0.0, 0.0, 0.18, 0.25),
            note(Square, 300.0, 90.0, 0.2, 0.2),
        ],
        Sfx::Victory => [523.0, 659.0, 784.0, 659.0, 784.0, 1047.0]
            .iter()
            .map(|&f| note(Square, f, f, 0.12, 0.25))
            .collect(),
    }
}

fn sfx_for(event: &Event) -> Sfx {
    match event {
        Event::Launch => Sfx::Launch,
        Event::WallBounce => Sfx::Wall,
        Event::PaddleBounce => Sfx::Paddle,
        Event::BrickHit { .. } => Sfx::BrickHit,
        Event::BrickDestroyed { .. } => Sfx::BrickBreak,
        Event::LifeLost => Sfx::LifeLost,
        Event::LevelCleared => Sfx::LevelClear,
        Event::GameOver => Sfx::GameOver,
        Event::Victory => Sfx::Victory,
        Event::StoneHit { .. } => Sfx::Stone,
        Event::BonusCaught { .. } => Sfx::Bonus,
        Event::BottomWallHit { remaining } if *remaining > 0 => Sfx::BottomWall,
        Event::BottomWallHit { .. } | Event::BottomWallBroken => Sfx::WallBreak,
    }
}

/// Samples of every effect, indexed like `sfx_index`.
pub fn all_samples() -> Vec<Vec<f32>> {
    ALL.into_iter().map(|sfx| synth(&recipe(sfx))).collect()
}

/// Which effect an event plays, as an index into `Sounds::samples`.
pub fn sfx_index(event: &Event) -> usize {
    sfx_for(event) as usize
}

pub struct Sounds {
    sounds: Vec<Option<Sound>>,
    /// The raw samples of every effect, for mixing video sound tracks.
    samples: Vec<Vec<f32>>,
}

impl Sounds {
    /// Generates and loads all effects. A sound that fails to load is
    /// simply skipped, so the game still runs without working audio.
    pub async fn load() -> Self {
        let samples = all_samples();
        let mut sounds = Vec::new();
        for s in &samples {
            sounds.push(load_sound_from_bytes(&wav_bytes(s)).await.ok());
        }
        Sounds { sounds, samples }
    }

    pub fn samples(&self) -> &[Vec<f32>] {
        &self.samples
    }

    pub fn play(&self, event: &Event) {
        if let Some(Some(sound)) = self.sounds.get(sfx_index(event)) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: VOLUME,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_valid() {
        let samples = synth(&recipe(Sfx::Paddle));
        let wav = wav_bytes(&samples);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + samples.len() * 2);
        let riff_len = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(riff_len as usize, wav.len() - 8);
    }

    #[test]
    fn all_sounds_are_short_and_in_range() {
        for (i, sfx) in ALL.into_iter().enumerate() {
            assert_eq!(sfx as usize, i, "ALL must match enum order");
            let s = synth(&recipe(sfx));
            assert!(!s.is_empty());
            assert!(s.len() < SAMPLE_RATE as usize, "effects stay under 1 s");
            assert!(s.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        }
    }
}
