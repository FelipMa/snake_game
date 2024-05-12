use color_eyre::{eyre::WrapErr, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        block::{Position, Title},
        canvas::*,
        *,
    },
};
use std::time::{Duration, Instant};

mod errors;
mod tui;

fn main() -> Result<()> {
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    App::new().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug)]
pub struct App {
    status: AppStatus,
    exit: bool,
    snake: Snake,
    tick_rate: Duration,
    tick_count: u64,
    field: Rect,
    score: u64,
    apple: Apple,
}

#[derive(Debug, PartialEq)]
enum AppStatus {
    Menu,
    Playing,
    GameOver,
}

impl App {
    pub fn new() -> Self {
        Self {
            status: AppStatus::Menu,
            exit: false,
            snake: Snake::new(),
            tick_rate: Duration::from_millis(128),
            tick_count: 0,
            field: Rect::default(),
            score: 0,
            apple: Apple {
                point: Point { x: 0.0, y: 0.0 },
            },
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        let mut last_tick = Instant::now();
        while !self.exit {
            terminal.draw(|frame| {
                self.field = frame.size();
                self.render_frame(frame)
            })?;

            let timeout = self.tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                self.handle_events().wrap_err("handle events failed")?;
            }

            if self.status == AppStatus::Playing && last_tick.elapsed() >= self.tick_rate {
                self.tick()?;
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<()> {
        self.tick_count += 1;
        if self.status == AppStatus::Playing {
            self.snake.direction = self.snake.next_direction;

            let head_next_point = match self.snake.direction {
                Direction::Up => Point {
                    x: self.snake.body[0].x,
                    y: self.snake.body[0].y - 1.0,
                },
                Direction::Down => Point {
                    x: self.snake.body[0].x,
                    y: self.snake.body[0].y + 1.0,
                },
                Direction::Left => Point {
                    x: self.snake.body[0].x - 2.0,
                    y: self.snake.body[0].y,
                },
                Direction::Right => Point {
                    x: self.snake.body[0].x + 2.0,
                    y: self.snake.body[0].y,
                },
            };

            if head_next_point.x == self.apple.point.x && head_next_point.y == self.apple.point.y {
                self.snake
                    .body
                    .push(self.snake.body.last().unwrap().clone());
                self.score += 1;
                self.generate_apple();
            }

            for i in (1..self.snake.body.len()).rev() {
                self.snake.body[i] = self.snake.body[i - 1];
                if head_next_point == self.snake.body[i - 1] {
                    self.status = AppStatus::GameOver;
                }
            }

            if head_next_point.x < 0.0
                || head_next_point.x > self.field.width as f64 - 3.0
                || head_next_point.y < 0.0
                || head_next_point.y > self.field.height as f64 - 3.0
            {
                self.status = AppStatus::GameOver;
            }

            self.snake.body[0] = head_next_point;
        }
        Ok(())
    }

    fn generate_apple(&mut self) {
        let mut possible_point = Point {
            x: (rand::random::<f64>() * ((self.field.width as f64) - 3.0)).floor(),
            y: (rand::random::<f64>() * ((self.field.height as f64) - 3.0)).floor(),
        };

        if possible_point.x % 2.0 != 0.0 {
            possible_point.x += 1.0;
        }

        if self.snake.body.contains(&possible_point) {
            self.generate_apple();
        } else {
            self.apple.point = possible_point;
        }
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc => self.exit(),
            KeyCode::Char(' ') => match self.status {
                AppStatus::Menu => self.start_game(),
                AppStatus::GameOver => self.start_game(),
                _ => {}
            },
            KeyCode::Enter => match self.status {
                AppStatus::GameOver => self.status = AppStatus::Menu,
                _ => {}
            },
            KeyCode::Up | KeyCode::Char('w') => match self.snake.direction {
                Direction::Down => {}
                _ => self.snake.next_direction = Direction::Up,
            },
            KeyCode::Down | KeyCode::Char('s') => match self.snake.direction {
                Direction::Up => {}
                _ => self.snake.next_direction = Direction::Down,
            },
            KeyCode::Left | KeyCode::Char('a') => match self.snake.direction {
                Direction::Right => {}
                _ => self.snake.next_direction = Direction::Left,
            },
            KeyCode::Right | KeyCode::Char('d') => match self.snake.direction {
                Direction::Left => {}
                _ => self.snake.next_direction = Direction::Right,
            },
            _ => {}
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn start_game(&mut self) {
        self.snake = Snake::new();
        self.score = 0;
        self.generate_apple();
        self.status = AppStatus::Playing;
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        match self.status {
            AppStatus::Menu => {
                frame.render_widget(self.generate_menu_widget(), frame.size());
            }
            AppStatus::Playing => {
                frame.render_widget(self.generate_game_widget(), frame.size());
            }
            AppStatus::GameOver => {
                frame.render_widget(self.generate_game_over_widget(), frame.size());
            }
        }
    }

    fn generate_menu_widget(&self) -> impl Widget + '_ {
        let title = Title::from(" Snake Game ".bold());
        let instructions = Title::from(text::Line::from(vec![
            " Play ".into(),
            "<Space>".blue().bold(),
            " Quit ".into(),
            "<Esc> ".blue().bold(),
        ]));

        Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_set(border::THICK)
    }

    fn generate_game_widget(&self) -> impl Widget + '_ {
        let game_border = Block::bordered()
            .title(Title::from(format!(" Score: {} ", self.score)).alignment(Alignment::Center))
            .border_set(border::THICK);

        Canvas::default()
            .block(game_border)
            .marker(Marker::Dot)
            .paint(move |ctx| {
                ctx.draw(&self.snake);
                ctx.draw(&self.apple);
            })
            .x_bounds([0.0, self.field.width as f64 - 3.0])
            .y_bounds([0.0, self.field.height as f64 - 3.0])
    }

    fn generate_game_over_widget(&self) -> impl Widget + '_ {
        let title = Title::from(" Game over ".bold());
        let instructions = Title::from(text::Line::from(vec![
            " Play Again ".into(),
            "<Space>".blue().bold(),
            " Main Menu ".into(),
            "<Enter>".blue().bold(),
            " Quit ".into(),
            "<Esc> ".blue().bold(),
        ]));
        let score = Title::from(format!(" Your score was: {} ", self.score));

        Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .title(score.alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_set(border::THICK)
    }
}

#[derive(Debug)]
pub struct Snake {
    body: Vec<Point>,
    direction: Direction,
    next_direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Snake {
    pub fn new() -> Self {
        Self {
            body: vec![
                Point { x: 8.0, y: 0.0 },
                Point { x: 6.0, y: 0.0 },
                Point { x: 4.0, y: 0.0 },
                Point { x: 2.0, y: 0.0 },
                Point { x: 0.0, y: 0.0 },
            ],
            direction: Direction::Right,
            next_direction: Direction::Right,
        }
    }
}

impl Shape for Snake {
    fn draw(&self, painter: &mut Painter) {
        for point in &self.body[1..] {
            painter.paint(point.x as usize, point.y as usize, Color::Green);
        }
        painter.paint(
            self.body[0].x as usize,
            self.body[0].y as usize,
            Color::Yellow,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Apple {
    pub point: Point,
}

impl Shape for Apple {
    fn draw(&self, painter: &mut Painter) {
        painter.paint(self.point.x as usize, self.point.y as usize, Color::Red);
    }
}
