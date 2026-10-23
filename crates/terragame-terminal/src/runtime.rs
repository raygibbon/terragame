use std::{
    io,
    time::{Duration, Instant},
};
use terrabox::{Event, FrameClock, Key, KeyEvent, Modifiers, Size, Surface, Terminal};
use terragame_core::FixedStep;
/// Thin lifecycle adapter. Terrabox owns restoration, input and changed output.
pub struct Session {
    terminal: Terminal,
    pub surface: Surface,
}
impl Session {
    pub fn new() -> io::Result<Self> {
        let mut terminal = Terminal::new()?;
        terminal.hide_cursor();
        let surface = Surface::new(terminal.size());
        Ok(Self { terminal, surface })
    }
    pub fn size(&self) -> Size {
        self.terminal.size()
    }
    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        self.terminal.wait_event(timeout)
    }
    pub fn present(&mut self) -> io::Result<()> {
        self.terminal
            .draw_surface(&self.surface, Default::default());
        self.terminal.present()
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Control {
    pub redraw: bool,
    pub quit: bool,
}
impl Control {
    pub const REDRAW: Self = Self {
        redraw: true,
        quit: false,
    };
    pub const QUIT: Self = Self {
        redraw: false,
        quit: true,
    };
}
/// Input occurs on receipt; tick and render are independent. render must not
/// mutate simulation state. Restarting game state does not reset the engine tick.
pub trait Application {
    fn event(&mut self, event: Event) -> Control;
    /// Return true when simulation changes need a future render.
    fn tick(&mut self, tick: u64) -> bool;
    fn render(&self, surface: &mut Surface, size: Size);
    fn paused(&self) -> bool {
        false
    }
    fn finished(&self) -> bool {
        false
    }
}
/// Blocking single-threaded driver. Render frames skip missed deadlines; only
/// simulation catches up (bounded). Pause removes all deadlines and blocks.
pub fn run(
    app: &mut impl Application,
    step: Duration,
    render_interval: Duration,
) -> io::Result<()> {
    let mut fixed = FixedStep::new(step, 4).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid simulation interval")
    })?;
    let now = Instant::now();
    let mut render_clock = FrameClock::new(render_interval, now)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid render interval"))?;
    let mut session = Session::new()?;
    let mut last = now;
    let mut paused = app.paused();
    let mut redraw = true;
    let mut dirty = false;
    if paused {
        render_clock.pause();
    }
    loop {
        let now = Instant::now();
        if !paused {
            let before = fixed.tick();
            for offset in 1..=fixed.advance(now.saturating_duration_since(last)) {
                dirty |= app.tick(before + u64::from(offset));
                if app.paused() || app.finished() {
                    fixed.set_tick(before + u64::from(offset));
                    break;
                }
            }
            if render_clock.poll(now).is_some() && dirty {
                redraw = true;
            }
        }
        last = now;
        if app.finished() {
            return Ok(());
        }
        if paused != app.paused() {
            paused = app.paused();
            fixed.reset();
            last = Instant::now();
            if paused {
                render_clock.pause();
            } else {
                render_clock.resume(last);
            }
            redraw = true;
        }
        if redraw {
            let size = session.size();
            if session.surface.size() != size {
                session.surface.resize(size);
            }
            app.render(&mut session.surface, size);
            session.present()?;
            redraw = false;
            dirty = false;
        }
        let timeout = if paused {
            None
        } else {
            let simulation = fixed.remaining().saturating_sub(last.elapsed());
            Some(
                render_clock
                    .deadline()
                    .map(|d| d.remaining(Instant::now()).min(simulation))
                    .unwrap_or(simulation),
            )
        };
        if let Some(event) = session.wait_event(timeout)? {
            let resize = matches!(event, Event::Resize(_));
            let control = app.event(event);
            if control.quit {
                return Ok(());
            }
            redraw |= control.redraw || resize;
            // Input can pause/resume. Do not charge time blocked in pause to play.
            if paused != app.paused() {
                paused = app.paused();
                fixed.reset();
                last = Instant::now();
                if paused {
                    render_clock.pause();
                } else {
                    render_clock.resume(last);
                }
                redraw = true;
            }
        }
    }
}

/// Conventional terminal-game quit binding, after Terrabox has decoded input.
/// Control letters may arrive upper- or lower-case depending on the protocol.
pub fn quit_key(event: KeyEvent) -> bool {
    !event.modifiers.contains(Modifiers::ALT)
        && (matches!(event.key, Key::Escape | Key::Char('q' | 'Q'))
            || event.modifiers.contains(Modifiers::CTRL)
                && matches!(event.key, Key::Char('c' | 'C')))
}
