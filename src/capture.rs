//! Glue between the game loop and `video`: draws video frames off screen at
//! 1080 × 1920 and keeps track of the current round's recording.

use std::path::PathBuf;
use std::thread::JoinHandle;

use macroquad::prelude::*;

use crate::app::App;
use crate::audio::sfx_index;
use crate::game::{Event, State};
use crate::render::{self, Effects, View};
use crate::t;
use crate::video::{self, Recorder, Recording, HEIGHT, WIDTH};

pub struct Capture {
    target: RenderTarget,
    camera: Camera2D,
    /// Pixels read back from the GPU are bottom-up on some platforms.
    flip: bool,
    enabled: bool,
    round: u64,
    /// The current round may still start recording (at its first launch).
    armed: bool,
    recorder: Option<Recorder>,
    recording: Option<Recording>,
    save_job: Option<JoinHandle<Result<PathBuf, String>>>,
    /// Old recordings to delete once the running save no longer needs them.
    discard_later: Vec<Recording>,
    samples: Vec<Vec<f32>>,
}

impl Capture {
    pub fn new(samples: Vec<Vec<f32>>, app: &mut App) -> Self {
        video::remove_old_temp_files();
        let enabled = video::ffmpeg_available();
        if !enabled {
            app.video_unavailable = Some(t!("video.no_ffmpeg").into());
        }
        let target = render_target(WIDTH, HEIGHT);
        target.texture.set_filter(FilterMode::Linear);
        let mut camera =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32));
        camera.render_target = Some(target.clone());
        let flip = enabled && reads_bottom_up(&camera, &target);
        Capture {
            target,
            camera,
            flip,
            enabled,
            round: app.round,
            armed: false,
            recorder: None,
            recording: None,
            save_job: None,
            discard_later: Vec::new(),
            samples,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.is_some()
    }

    /// Call once per frame after the game has been updated; `events` are
    /// this frame's game events (for the sound track).
    pub fn frame(&mut self, app: &mut App, fx: &Effects, frame_time: f32, events: &[Event]) {
        if app.round != self.round {
            self.new_round(app);
        }
        let plan = video::plan_frame(video::RoundState {
            recording: self.recorder.is_some(),
            armed: self.armed,
            launched: events.contains(&Event::Launch),
            in_game: app.in_game(),
            over: app.round_over(),
            paused: app.game.state == State::Paused,
        });
        if plan.start {
            self.armed = false;
            match Recorder::start(self.flip) {
                Ok(rec) => self.recorder = Some(rec),
                Err(e) => {
                    app.video_unavailable = Some(t!("video.start_failed", error = e));
                    return;
                }
            }
        }
        if let Some(rec) = &mut self.recorder {
            if plan.capture {
                // The launch frame is the video's first frame, at time 0.
                let dt = if plan.start { 0.0 } else { frame_time as f64 };
                let copies = rec.frames_due(dt);
                for event in events {
                    rec.add_sound(sfx_index(event));
                }
                if copies > 0 {
                    set_camera(&self.camera);
                    let view = View::fit(WIDTH as f32, HEIGHT as f32);
                    render::draw(&app.game, fx, view, app.back_hint(), plan.overlays);
                    set_default_camera();
                    let image = self.target.texture.get_texture_data();
                    rec.push_frame(image.bytes, copies);
                }
            }
        }
        if plan.finish {
            self.end_round(app);
        }
    }

    /// A new round: drop the previous (unsaved) recording and wait for the
    /// first launch before recording again.
    fn new_round(&mut self, app: &mut App) {
        self.round = app.round;
        if let Some(rec) = self.recorder.take() {
            rec.cancel();
        }
        if let Some(old) = self.recording.take() {
            if self.save_job.is_some() {
                self.discard_later.push(old);
            } else {
                old.discard();
            }
        }
        app.video_ready = false;
        self.armed = self.enabled;
    }

    fn end_round(&mut self, app: &mut App) {
        let Some(rec) = self.recorder.take() else {
            return;
        };
        match rec.finish() {
            Ok(recording) => {
                self.recording = Some(recording);
                app.video_ready = true;
            }
            Err(e) => app.video_message = Some((t!("video.record_failed", error = e), true)),
        }
    }

    /// Starts requested saves and reports finished ones.
    pub fn poll_saves(&mut self, app: &mut App) {
        for path in std::mem::take(&mut app.video_requests) {
            match (&self.recording, &self.save_job) {
                (Some(rec), None) => {
                    let (rec, samples) = (rec.clone(), self.samples.clone());
                    // Messages from the save thread use the player's language.
                    let lang = crate::i18n::lang();
                    self.save_job = Some(std::thread::spawn(move || {
                        crate::i18n::set_lang(lang);
                        rec.save(&samples, &path).map(|()| path)
                    }));
                }
                (None, _) => app.video_saved(Err(t!("video.nothing").into())),
                (Some(_), Some(_)) => app.video_saved(Err(t!("video.busy").into())),
            }
        }
        if self.save_job.as_ref().is_some_and(|j| j.is_finished()) {
            let job = self.save_job.take().expect("checked above");
            let result = job
                .join()
                .unwrap_or_else(|_| Err(t!("video.crashed").into()));
            app.video_saved(result);
            for old in self.discard_later.drain(..) {
                old.discard();
            }
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        if let Some(rec) = self.recorder.take() {
            rec.cancel();
        }
        if let Some(rec) = self.recording.take() {
            rec.discard();
        }
    }
}

/// Draws red in the top half and checks which end of the read-back image
/// it lands in.
fn reads_bottom_up(camera: &Camera2D, target: &RenderTarget) -> bool {
    set_camera(camera);
    clear_background(BLACK);
    draw_rectangle(0.0, 0.0, WIDTH as f32, HEIGHT as f32 / 2.0, RED);
    set_default_camera();
    let image = target.texture.get_texture_data();
    let first_row_red = image.bytes.first().is_some_and(|&r| r > 128);
    !first_row_red
}
