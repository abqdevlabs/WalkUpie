use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Work,
    Break,
}

impl Phase {
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Work => "Work",
            Phase::Break => "Break",
        }
    }

    pub fn from_str(s: &str) -> Option<Phase> {
        match s {
            "work" => Some(Phase::Work),
            "break" => Some(Phase::Break),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Phase::Work => "work",
            Phase::Break => "break",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Transition {
    ToBreak,
    ToWork,
}

pub struct Timer {
    pub phase: Phase,
    pub running: bool,
    work: Duration,
    brk: Duration,
    override_duration: Option<Duration>,
    started: Option<Instant>,
    paused_remaining: Duration,
}

impl Timer {
    pub fn new(work: Duration, brk: Duration) -> Self {
        Self {
            phase: Phase::Work,
            running: true,
            work,
            brk,
            override_duration: None,
            started: Some(Instant::now()),
            paused_remaining: Duration::ZERO,
        }
    }

    pub fn restore(
        phase: Phase,
        remaining: Duration,
        work: Duration,
        brk: Duration,
    ) -> Self {
        let mut t = Self::new(work, brk);
        t.phase = phase;
        t.paused_remaining = remaining;
        t.running = false;
        t.started = None;
        t.resume();
        t
    }

    pub fn set_durations(&mut self, work: Duration, brk: Duration) {
        self.work = work;
        self.brk = brk;
    }

    pub fn duration(&self) -> Duration {
        self.override_duration.unwrap_or(match self.phase {
            Phase::Work => self.work,
            Phase::Break => self.brk,
        })
    }

    pub fn remaining(&self) -> Duration {
        match self.started {
            Some(start) => self.duration().saturating_sub(start.elapsed()),
            None => self.paused_remaining,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.duration().saturating_sub(self.remaining())
    }

    pub fn is_paused(&self) -> bool {
        !self.running
    }

    pub fn tick(&mut self) -> Option<Transition> {
        if !self.running {
            return None;
        }
        if let Some(start) = self.started {
            if start.elapsed() >= self.duration() {
                return Some(self.switch());
            }
        }
        None
    }

    fn switch(&mut self) -> Transition {
        let transition = match self.phase {
            Phase::Work => {
                self.phase = Phase::Break;
                Transition::ToBreak
            }
            Phase::Break => {
                self.phase = Phase::Work;
                Transition::ToWork
            }
        };
        self.override_duration = None;
        self.started = Some(Instant::now());
        self.running = true;
        transition
    }

    pub fn skip(&mut self) -> Transition {
        self.switch()
    }

    pub fn snooze(&mut self, duration: Duration) {
        self.phase = Phase::Break;
        self.override_duration = Some(duration);
        self.started = Some(Instant::now());
        self.running = true;
    }

    pub fn pause(&mut self) {
        if self.running {
            self.paused_remaining = self.remaining();
            self.running = false;
            self.started = None;
        }
    }

    pub fn resume(&mut self) {
        if !self.running {
            self.started = Some(Instant::now());
            self.running = true;
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.running {
            self.pause();
        } else {
            self.resume();
        }
    }
}

pub fn fmt_mmss(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}