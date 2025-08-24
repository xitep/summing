use std::{borrow::Cow, io};

use anyhow::Result;
use game::{Cursor, Game};
use rand::{Rng, SeedableRng};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget},
    DefaultTerminal, Frame,
};

mod args;
mod game;

fn main() -> Result<()> {
    let args = args::from_env();
    let mut app = App {
        game: Game::new(if let Some(seed) = args.seed {
            rand::rngs::StdRng::seed_from_u64(seed)
        } else {
            rand::rngs::StdRng::from_os_rng()
        }),
        point: Some(Cursor::default()),
        seed_info: args.seed.map(|seed| {
            let mut b = itoa::Buffer::new();
            let seed = b.format(seed);
            let mut s = String::with_capacity(seed.len() + 2);
            s.push('[');
            s.push_str(seed);
            s.push(']');
            s
        }),
        mode: ScreenMode::Playing,
        help_return_mode: ScreenMode::Playing,
    };
    #[cfg(feature = "dev")]
    if let Some(state) = args.start_state {
        match state {
            args::StartState::Success => app.game.set_finished(game::Finished::Success),
            args::StartState::Failure => app.game.set_finished(game::Finished::Failure),
        }
        app.mode = ScreenMode::GameOver;
    }
    let terminal = ratatui::init();
    let result = app.run(terminal);
    ratatui::restore();
    result
}

// --------------------------------------------------------------------

struct App<R> {
    game: Game<R>,
    point: Option<Cursor>,
    seed_info: Option<String>,
    mode: ScreenMode,
    // ~ the mode to return to when closing the 'help' window;
    // maintained/set when opening the 'help' window
    help_return_mode: ScreenMode,
}

#[derive(Clone, Copy)]
enum ScreenMode {
    Playing,
    GameOver,
    // Maintains the current scroll position
    Help(u16),
    Exit,
}

impl<R: Rng> App<R> {
    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !matches!(self.mode, ScreenMode::Exit) {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let height = self.game.rows() as u16 + 2;
        let width = self.game.cols() as u16 * 2 + 1 + 5;

        let frame_area = frame.area();
        if frame_area.width < width || frame_area.height < height {
            let [area] = Layout::vertical([Constraint::Length(1)])
                .flex(Flex::Center)
                .areas(frame_area);
            frame.render_widget(Line::raw("Window too small!").centered(), area);
            return;
        }

        let board_area = {
            Rect {
                x: frame_area.x + (frame_area.width - width) / 2,
                y: frame_area.y + (frame_area.height - height) / 2,
                width,
                height,
            }
        };
        frame.render_widget(&self.game, board_area);

        match self.mode {
            ScreenMode::Playing | ScreenMode::GameOver => {
                if let Some(state) = self.game.is_finished() {
                    let s = match state {
                        game::Finished::Success => Cow::Owned(format!(
                            "Congratulations!\n\nYou made it with {} placements only! 😎",
                            self.game.num_placed(),
                        )),
                        game::Finished::Failure => {
                            Cow::Borrowed("Too bad, no more placements possible!\n\nGame over! 😕")
                        }
                    };
                    // ~ make the row above and below blank as well
                    let mut area = Rect {
                        x: frame_area.x,
                        y: frame_area.y + (frame_area.height / 2) - 2 - 1,
                        width: frame_area.width,
                        height: 5,
                    };
                    frame.render_widget(Clear, area);
                    // ~ shrink the area
                    area.y += 1;
                    frame.render_widget(Paragraph::new(s).centered(), area);
                } else if let Some(point) = self.point {
                    frame.set_cursor_position(Position {
                        x: board_area.x + 1 + point.x as u16 * 2,
                        y: board_area.y + 1 + point.y as u16,
                    });
                }
            }
            ScreenMode::Help(ref mut scroll) => {
                frame.render_stateful_widget(
                    Help,
                    Rect {
                        x: frame_area.x,
                        y: frame_area.y,
                        width: frame_area.width,
                        height: frame_area.height.saturating_sub(1),
                    },
                    scroll,
                );
            }
            ScreenMode::Exit => {}
        }

        let hint_rect = Rect {
            x: 0,
            y: frame_area.y + frame_area.height - 1,
            width: frame_area.width,
            height: 1,
        };
        if let Some(seed_info) = self.seed_info.as_ref() {
            frame.render_widget(
                Line::raw(seed_info).right_aligned().fg(Color::DarkGray),
                hint_rect,
            );
        }
        let line = match self.mode {
            ScreenMode::GameOver => Line::from_iter([
                Span::raw(" "),
                Span::raw("q").fg(Color::Magenta),
                Span::raw("uit | "),
                Span::raw("n").fg(Color::Magenta),
                Span::raw("ew game | "),
                Span::raw("h").fg(Color::Magenta),
                Span::raw("elp"),
            ]),
            ScreenMode::Help(_) => Line::from_iter([
                Span::raw(" "),
                Span::raw("q").fg(Color::Magenta),
                Span::raw("/"),
                Span::raw("esc").fg(Color::Magenta),
                Span::raw(" close"),
            ]),
            _ => Line::from_iter([
                Span::raw(" "),
                Span::raw("q").fg(Color::Magenta).bold(),
                Span::raw("uit | "),
                Span::raw("h").fg(Color::Magenta).bold(),
                Span::raw("elp | ←↑↓→ <space>"),
            ]),
        };
        frame.render_widget(line.fg(Color::DarkGray), hint_rect);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, event: KeyEvent) {
        match self.mode {
            ScreenMode::Playing => match event.code {
                KeyCode::Char('q') => {
                    self.mode = ScreenMode::Exit;
                }
                KeyCode::Char('h') => {
                    self.help_return_mode = self.mode;
                    self.mode = ScreenMode::Help(0);
                }
                KeyCode::Char('p') if event.modifiers == KeyModifiers::CONTROL => {
                    self.move_cursor(game::Direction::North)
                }
                KeyCode::Up => self.move_cursor(game::Direction::North),
                KeyCode::Char('n') if event.modifiers == KeyModifiers::CONTROL => {
                    self.move_cursor(game::Direction::South);
                }
                KeyCode::Down => self.move_cursor(game::Direction::South),
                KeyCode::Char('f') if event.modifiers == KeyModifiers::CONTROL => {
                    self.move_cursor(game::Direction::East)
                }
                KeyCode::Right => self.move_cursor(game::Direction::East),
                KeyCode::Char('b') if event.modifiers == KeyModifiers::CONTROL => {
                    self.move_cursor(game::Direction::West);
                }
                KeyCode::Left => {
                    self.move_cursor(game::Direction::West);
                }
                KeyCode::Char(' ') => {
                    if let Some(point) = self.point {
                        if self.game.place_next(point) {
                            self.point = self.game.find_free_any(point);
                        }
                        if self.game.is_finished().is_some() {
                            self.mode = ScreenMode::GameOver;
                        }
                    }
                }
                _ => {}
            },
            ScreenMode::GameOver => match event.code {
                KeyCode::Char('q') => {
                    self.mode = ScreenMode::Exit;
                }
                KeyCode::Char('h') => {
                    self.help_return_mode = self.mode;
                    self.mode = ScreenMode::Help(0);
                }
                KeyCode::Char('n') => {
                    self.game.reinit();
                    self.point = Some(Cursor::default());
                    self.mode = ScreenMode::Playing;
                }
                _ => {}
            },
            ScreenMode::Help(scroll) => match event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.mode = self.help_return_mode;
                }
                KeyCode::Char('p') if event.modifiers == KeyModifiers::CONTROL => {
                    self.mode = ScreenMode::Help(scroll.saturating_sub(1));
                }
                KeyCode::Up => {
                    self.mode = ScreenMode::Help(scroll.saturating_sub(1));
                }
                KeyCode::Char('n') if event.modifiers == KeyModifiers::CONTROL => {
                    self.mode = ScreenMode::Help(scroll.saturating_add(1));
                }
                KeyCode::Down => {
                    self.mode = ScreenMode::Help(scroll.saturating_add(1));
                }
                _ => {}
            },
            ScreenMode::Exit => {}
        }
    }

    fn move_cursor(&mut self, direction: game::Direction) {
        if let Some(point) = self.point {
            self.point = self.game.find_free(point, direction);
        }
    }
}

impl<R> Widget for &Game<R> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        // board ------------------------------------------------------

        let mut y = area.y + 1; // ~ one for the border
        for r in 0..self.rows() {
            let mut x = area.left() + 1; // ~ one for the border
            for c in 0..self.cols() {
                if let Some(s) = self.get(r, c) {
                    buf[Position { x, y }]
                        .set_style(stone_style(s).bold())
                        .set_symbol(s);
                }
                x += 2;
            }
            y += 1;
        }

        // ~ the last colum is only one char wide
        Block::bordered().render(
            Rect {
                x: area.x,
                y: area.y,
                width: 1 + self.cols() as u16 * 2, /* +1 -1 */
                height: self.rows() as u16 + 2,
            },
            buf,
        );

        // nexts ------------------------------------------------------

        Block::bordered().render(
            Rect {
                x: area.x + 1 + self.cols() as u16 * 2,
                y: area.y,
                width: 5,
                height: self.rows() as u16 + 2,
            },
            buf,
        );
        let x = area.x + 1 + 1 + self.cols() as u16 * 2 + 1;
        if area.y > 0 {
            buf[Position {
                x: x - 3,
                y: area.y - 1,
            }]
            .set_symbol("↶")
            .set_fg(Color::DarkGray);
        }
        y = area.y + 1;
        for (i, s) in self.nexts().enumerate() {
            if i > 0 {
                buf[Position { x, y }]
                    .set_symbol("↑")
                    .set_fg(Color::DarkGray);
                y += 1;
            }
            buf[Position { x, y }]
                .set_style(stone_style(s))
                .set_symbol(s);
            y += 1;
        }
        buf[Position { x, y }]
            .set_symbol("—")
            .set_fg(Color::DarkGray);
        y += 1;

        // num_placed stones so far -----------------------------------
        let mut b = itoa::Buffer::new();
        let s = b.format(self.num_placed());
        buf[Position {
            x: if s.len() > 1 { x - 1 } else { x },
            y,
        }]
        .set_symbol(s);
    }
}

static STONE_STYLES: [Style; 10] = [
    /* 0 */ Style::new().bg(Color::DarkGray).fg(Color::White),
    /* 1 */ Style::new().bg(Color::Magenta).fg(Color::White),
    /* 2 */ Style::new().bg(Color::Blue).fg(Color::White),
    /* 3 */ Style::new().bg(Color::Red).fg(Color::LightYellow),
    /* 4 */ Style::new().bg(Color::Yellow).fg(Color::Black),
    /* 5 */ Style::new().bg(Color::Green).fg(Color::Black),
    /* 6 */ Style::new().bg(Color::LightBlue).fg(Color::Black),
    /* 7 */ Style::new().bg(Color::Magenta).fg(Color::Black),
    /* 8 */ Style::new().bg(Color::DarkGray).fg(Color::Yellow),
    /* 9 */ Style::new().bg(Color::Gray).fg(Color::Black),
];

fn stone_style(label: &str) -> Style {
    STONE_STYLES[label.as_bytes()[0] as usize - '0' as usize]
}

struct Help;

impl StatefulWidget for Help {
    type State = u16;

    fn render(self, area: Rect, buf: &mut Buffer, scroll: &mut Self::State) {
        if *scroll as usize + area.height as usize - 2 > HELP_LINES {
            *scroll = HELP_LINES.saturating_sub(area.height as usize - 2) as u16;
        }
        Clear.render(area, buf);
        Paragraph::new(HELP_TEXT)
            .centered()
            .on_blue()
            .white()
            .block(
                Block::bordered()
                    .title(HELP_TITLE)
                    .title_alignment(Alignment::Center),
            )
            .scroll((*scroll, 0))
            .render(area, buf);
    }
}

const HELP_TITLE: &str = constcat::concat!(
    " ",
    env!("CARGO_BIN_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " "
);

const HELP_TEXT: &str = r#"
You're goal is to iteratively clear the 9x9 board on the left
by placing a given number onto the board such that the sum of
all the neighbours of the chosen place on the board (in any
direction, including the diagonals) modulo 10 is equal to it;
i.e. `sum(neighbours) % 10 == number`. In other words, the
last (decimal) digit of the neighbours' sum must equal the
given number. Of course, numbers can only be placed not yet
unoccupied places on the board.

If the sum matches, all the neighbours disappear.  If it
doesn't, the chosen place on the board becomes occupied by
the number at hand.

Numbers are handed out to you from the top of the magazine
on the right.  You can see the next four to come in their
order of being available to you; this allows you to be clever
and strategic about the numbers' placements.

Apart of clearing the board, the ultimate challenge is in
doing so in as few placements as possible.  The current number
of placements in a game is displayed at the bottom of the
magazine.

--

To move around the board  use the arrow keys.  The cursor
will jump from one free place to the next.  Press 'space'
to place the next, top number from the magazine to the
current cursor position on the board.

--

This version of the game is a nostalgic remake of
"Summing for PalmOS" (https://palmdb.net/app/summing-math);
written in Rust with Ratatui.

--

Enjoy and have fun!
"#;

const HELP_LINES: usize = num_lines(HELP_TEXT);

const fn num_lines(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut n = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            n += 1;
        }
        i += 1;
    }
    n
}
