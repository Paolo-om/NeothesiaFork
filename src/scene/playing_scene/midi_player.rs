use crate::{output_manager::OutputManager, target::Target};
use num::FromPrimitive;
use std::{cell::RefCell, collections::HashSet, rc::Rc, time::Duration};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseButton},
};

use crate::midi_event::MidiEvent;

mod rewind_controler;
use rewind_controler::RewindController;

pub struct MidiPlayer {
    playback: lib_midi::PlaybackState,
    rewind_controller: RewindController,
    output_manager: Rc<RefCell<OutputManager>>,
    midi_file: Rc<lib_midi::Midi>,
    play_along: PlayAlong,
}

impl MidiPlayer {
    pub fn new(target: &mut Target) -> Self {
        let midi_file = target.midi_file.as_ref().unwrap();

        let mut player = Self {
            playback: lib_midi::PlaybackState::new(Duration::from_secs(3), &midi_file.merged_track),
            rewind_controller: RewindController::None,
            output_manager: target.output_manager.clone(),
            midi_file: midi_file.clone(),
            play_along: PlayAlong::default(),
        };
        player.update(target, Duration::ZERO);

        player
    }

    /// Variance
pub fn update(
    &mut self,
    target: &mut Target,
    delta: Duration,
    ) -> Option<Vec<lib_midi::MidiEvent>> {
    rewind_controler::update(self, target);

    let elapsed = (delta / 10) * (target.config.speed_multiplier * 10.0) as u32;

    let mut events = self.playback.update(&self.midi_file.merged_track, elapsed);

    // Get the timestamp of the next note in the MIDI file
    let next_note_timestamp = events
        .iter()
        .filter_map(|event| match event.message {
            MidiMessage::NoteOn { key, .. } => Some(key.timestamp),
            _ => None,
        })
        .min();

    // If there are no more notes in the MIDI file, assume the user has finished
    let user_finished = next_note_timestamp.is_none();

    // Calculate the acceptable error range
    let error_range = if user_finished {
        // If the user has finished, the acceptable error range is infinity
        None
    } else {
        let max_latency = Duration::from_millis(300);
        let time_left = next_note_timestamp.unwrap() - elapsed;
        let max_latency = max_latency.min(time_left);
        let min_latency = max_latency.neg();
        Some((min_latency, max_latency))
    };

    // Handle user input
    let mut note_pressed = false;
    let mut note_released = false;
    let mut user_latency = None;
    if let Some(key) = self.play_along.get_current_key() {
        let user_timestamp = key.timestamp;
        if let Some((min_latency, max_latency)) = error_range {
            if user_timestamp >= min_latency && user_timestamp <= max_latency {
                user_latency = Some(user_timestamp - next_note_timestamp.unwrap());
                note_pressed = key.is_pressed();
                note_released = !key.is_pressed();
            }
        }
    }

    // Determine the state based on the user's latency
    let state = match user_latency {
        Some(latency) if latency <= Duration::from_millis(50) => "perfect",
        Some(latency) if latency <= Duration::from_millis(100) => "little late",
        Some(latency) if latency <= Duration::from_millis(200) => "late",
        Some(latency) if latency <= Duration::from_millis(300) => "late",
        Some(latency) if latency >= Duration::from_millis(-50) => "perfect",
        Some(latency) if latency >= Duration::from_millis(-100) => "little early",
        Some(latency) if latency >= Duration::from_millis(-200) => "early",
        _ => "",
    };

    // Handle MIDI events
    events.retain(|event| match event.message {
        MidiMessage::NoteOn { key, vel } => {
            let event = midi::Message::NoteOn(
                midi::Channel::from_u8(event.channel).unwrap(),
                key.as_int(),
                vel.as_int(),
            );
            self.output_manager.borrow_mut().midi_event(event);
            self.play_along.press_key(KeyPressSource::File, key.as_int(), true);
            true
        }
        MidiMessage::NoteOff { key, .. } => {
            let event = midi::Message::NoteOff(
                midi::Channel::from_u8(event.channel).unwrap(),
                key.as_int(),
                0,
            );
            self.output_manager.borrow_mut().midi_event(event);
            self.play_along.press
            // Find the earliest note that hasn't been played yet
            let next_note = self
            .midi_file
            .merged_track
            .iter()
            .filter_map(|event| match event.message {
            MidiMessage::NoteOn { key, .. } => Some((event.delta.as_int(), key.as_int())),
            _ => None,
            })
            .find(|(delta, )| *delta >= self.playback.pos)
            .map(|(, key)| key);
                let time_since_next_note = next_note
        .map(|key| {
            let next_note_timestamp = self
                .midi_file
                .merged_track
                .iter()
                .filter_map(|event| match event.message {
                    MidiMessage::NoteOn { key: k, .. } if k == key => Some(event.delta),
                    _ => None,
                })
                .next()
                .unwrap();

            let next_note_elapsed = self.playback.pos - next_note_timestamp.as_int();

            (next_note_elapsed as f32 / 10.0) * target.config.speed_multiplier
        })
        .unwrap_or(0.0);

    let elapsed_with_lead = (elapsed as f32 + time_since_next_note) as u32;

    let events = self
        .playback
        .update(&self.midi_file.merged_track, elapsed_with_lead);

    // Check if the user's input matches a note in the file
    let input_match = self.check_input_match(target);

    // Determine the timing of the user's input relative to the nearest note in the file
    let timing = if let Some((expected_time, _)) = input_match {
        let delta = expected_time as i32 - self.playback.pos as i32;
        if delta < -300 {
            Timing::Early
        } else if delta < -100 {
            Timing::LittleEarly
        } else if delta < 100 {
            Timing::Perfect
        } else if delta < 300 {
            Timing::LittleLate
        } else {
            Timing::Late
        }
    } else {
        Timing::None
    };

    // Handle the user's input based on the timing
    match timing {
        Timing::Perfect => {
            self.handle_input_match(target, true);
        }
        Timing::Early | Timing::LittleEarly => {
            self.handle_input_match(target, false);
            target.combo.reset();
        }
        Timing::Late | Timing::LittleLate => {
            self.handle_input_miss(target);
            target.combo.reset();
        }
        Timing::None => {}
    }

    events.iter().for_each(|event| {
        use lib_midi::midly::MidiMessage;
        match event.message {
            MidiMessage::NoteOn { key, vel } => {
                let event = midi::Message::NoteOn(
                    midi::Channel::from_u8(event.channel).unwrap(),
                    key.as_int(),
                    vel.as_int(),
                );
                self.output_manager.borrow_mut().midi_event(event);
                self.play_along
                    .press_key(KeyPressSource::File, key.as_int(), true);
            }
            MidiMessage::NoteOff { key, .. } => {
                let event = midi::Message::NoteOff(
                    midi::Channel::from_u8(event.channel).unwrap(),
                    key.as_int(),
                    0,
                );
                self.output_manager.borrow_mut().midi_event(event);
                self.play_along
                    .press_key(KeyPressSource::File, key.as_int(), false);
            }
            _ => {}
        }
    });

    None
}
    
    

    fn clear(&mut self) {
        let mut output = self.output_manager.borrow_mut();
        for note in self.playback.active_notes().iter() {
            output.midi_event(
                MidiEvent::NoteOff {
                    channel: note.channel,
                    key: note.key,
                }
                .into(),
            )
        }
    }
}

impl Drop for MidiPlayer {
    fn drop(&mut self) {
        self.clear();
    }
}

impl MidiPlayer {
    pub fn start(&mut self) {
        self.resume();
    }

    pub fn pause_resume(&mut self) {
        if self.playback.is_paused() {
            self.resume();
        } else {
            self.pause();
        }
    }

    pub fn pause(&mut self) {
        self.clear();
        self.playback.pause();
    }

    pub fn resume(&mut self) {
        self.playback.resume();
    }

    fn set_time(&mut self, time: Duration) {
        self.playback.set_time(time);

        // Discard all of the events till that point
        let events = self
            .playback
            .update(&self.midi_file.merged_track, Duration::ZERO);
        std::mem::drop(events);

        self.clear();
    }

    pub fn rewind(&mut self, delta: i64) {
        let mut time = self.playback.time();

        if delta < 0 {
            let delta = Duration::from_millis((-delta) as u64);
            time = time.saturating_sub(delta);
        } else {
            let delta = Duration::from_millis(delta as u64);
            time = time.saturating_add(delta);
        }

        self.set_time(time);
    }

    pub fn set_percentage_time(&mut self, p: f32) {
        self.set_time(Duration::from_secs_f32(
            (p * self.playback.lenght().as_secs_f32()).max(0.0),
        ));
    }

    pub fn percentage(&self) -> f32 {
        self.playback.percentage()
    }

    pub fn time_without_lead_in(&self) -> f32 {
        self.playback.time().as_secs_f32() - self.playback.leed_in().as_secs_f32()
    }

    pub fn is_paused(&self) -> bool {
        self.playback.is_paused()
    }
}

impl MidiPlayer {
    pub fn keyboard_input(&mut self, input: &KeyboardInput) {
        rewind_controler::handle_keyboard_input(self, input)
    }

    pub fn mouse_input(&mut self, target: &mut Target, state: &ElementState, button: &MouseButton) {
        rewind_controler::handle_mouse_input(self, target, state, button)
    }

    pub fn handle_cursor_moved(&mut self, target: &mut Target, position: &PhysicalPosition<f64>) {
        rewind_controler::handle_cursor_moved(self, target, position)
    }
}

impl MidiPlayer {
    pub fn play_along(&self) -> &PlayAlong {
        &self.play_along
    }

    pub fn play_along_mut(&mut self) -> &mut PlayAlong {
        &mut self.play_along
    }
}

pub enum KeyPressSource {
    File,
    User,
}

#[derive(Debug, Default)]
pub struct PlayAlong {
    required_notes: HashSet<u8>,
}

impl PlayAlong {
    fn user_press_key(&mut self, note_id: u8, active: bool) {
        if active {
            self.required_notes.remove(&note_id);
        }
    }

    fn file_press_key(&mut self, note_id: u8, active: bool) {
        if active {
            self.required_notes.insert(note_id);
        } else {
            self.required_notes.remove(&note_id);
        }
    }

    pub fn press_key(&mut self, src: KeyPressSource, note_id: u8, active: bool) {
        match src {
            KeyPressSource::User => self.user_press_key(note_id, active),
            KeyPressSource::File => self.file_press_key(note_id, active),
        }
    }

    pub fn are_required_keys_pressed(&self) -> bool {
        self.required_notes.is_empty()
    }
}
