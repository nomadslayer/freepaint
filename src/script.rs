//! Tiny input-automation format used for smoke testing the real UI:
//!
//! ```text
//! click X Y [right] [cmd] [shift]
//! move X Y
//! scroll X Y DY
//! drag X1 Y1 X2 Y2 [STEPS] [right] [cmd] [shift]
//! key NAME [cmd] [shift]        (NAME = egui key name, e.g. Enter, Delete, A)
//! text SOME TEXT
//! wait FRAMES
//! ```
//! Coordinates are window points.

use std::collections::VecDeque;

use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput};

struct Step {
    events: Vec<Event>,
    modifiers: Modifiers,
}

pub struct Script {
    steps: VecDeque<Step>,
}

fn mods(words: &[&str]) -> Modifiers {
    let mut m = Modifiers::NONE;
    if words.contains(&"cmd") {
        m = m | Modifiers::COMMAND;
    }
    if words.contains(&"shift") {
        m = m | Modifiers::SHIFT;
    }
    m
}

impl Script {
    pub fn parse(src: &str) -> Self {
        let mut steps = VecDeque::new();
        let push = |steps: &mut VecDeque<Step>, events: Vec<Event>, m: Modifiers| steps.push_back(Step { events, modifiers: m });
        for line in src.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
            let words: Vec<&str> = rest.split_whitespace().collect();
            let nums: Vec<f32> = words.iter().filter_map(|w| w.parse().ok()).collect();
            let button = if words.contains(&"right") { PointerButton::Secondary } else { PointerButton::Primary };
            let m = mods(&words);
            match cmd {
                "click" if nums.len() >= 2 => {
                    let p = Pos2::new(nums[0], nums[1]);
                    push(&mut steps, vec![Event::PointerMoved(p)], m);
                    push(&mut steps, vec![Event::PointerButton { pos: p, button, pressed: true, modifiers: m }], m);
                    push(&mut steps, vec![Event::PointerButton { pos: p, button, pressed: false, modifiers: m }], m);
                    push(&mut steps, vec![], m);
                }
                "drag" if nums.len() >= 4 => {
                    let a = Pos2::new(nums[0], nums[1]);
                    let b = Pos2::new(nums[2], nums[3]);
                    let n = nums.get(4).copied().unwrap_or(10.0).max(1.0) as usize;
                    push(&mut steps, vec![Event::PointerMoved(a)], m);
                    push(&mut steps, vec![Event::PointerButton { pos: a, button, pressed: true, modifiers: m }], m);
                    for i in 1..=n {
                        let t = i as f32 / n as f32;
                        push(&mut steps, vec![Event::PointerMoved(a + (b - a) * t)], m);
                    }
                    push(&mut steps, vec![Event::PointerButton { pos: b, button, pressed: false, modifiers: m }], m);
                    push(&mut steps, vec![], m);
                }
                "scroll" if nums.len() >= 3 => {
                    let p = Pos2::new(nums[0], nums[1]);
                    push(&mut steps, vec![Event::PointerMoved(p)], m);
                    push(&mut steps, vec![Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, nums[2]), modifiers: m }], m);
                    push(&mut steps, vec![], m);
                }
                "move" if nums.len() >= 2 => {
                    push(&mut steps, vec![Event::PointerMoved(Pos2::new(nums[0], nums[1]))], m);
                    push(&mut steps, vec![], m);
                }
                "key" if !words.is_empty() => {
                    if let Some(key) = Key::from_name(words[0]) {
                        push(&mut steps, vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: m }], m);
                        push(&mut steps, vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: m }], m);
                    }
                }
                "text" => push(&mut steps, vec![Event::Text(rest.to_string())], Modifiers::NONE),
                "wait" => {
                    for _ in 0..nums.first().copied().unwrap_or(1.0) as usize {
                        push(&mut steps, vec![], Modifiers::NONE);
                    }
                }
                _ => eprintln!("script: cannot parse line: {line}"),
            }
        }
        Self { steps }
    }

    pub fn is_done(&self) -> bool {
        self.steps.is_empty()
    }

    /// Feed the next step's events into this frame's input.
    pub fn apply(&mut self, raw: &mut RawInput) {
        if let Some(step) = self.steps.pop_front() {
            raw.modifiers = step.modifiers;
            raw.events.extend(step.events);
        }
    }
}
