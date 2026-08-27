use std::io;
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph, Clear, Borders, BorderType},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    MainMenu,
    Playing,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrafficDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarType {
    Red,
    Blue,
    Yellow,
    Magenta,
}

impl CarType {
    fn speed(&self) -> u32 {
        match self {
            CarType::Red => 2,
            CarType::Blue => 3,
            CarType::Yellow => 4,
            CarType::Magenta => 5,
        }
    }

    fn color(&self) -> Color {
        match self {
            CarType::Red => Color::Red,
            CarType::Blue => Color::Blue,
            CarType::Yellow => Color::Yellow,
            CarType::Magenta => Color::Magenta,
        }
    }

    fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..4) {
            0 => CarType::Red,
            1 => CarType::Blue,
            2 => CarType::Yellow,
            _ => CarType::Magenta,
        }
    }
}

#[derive(Debug, Clone)]
struct Car {
    x: i16,
    car_type: CarType,
}

#[derive(Debug, Clone)]
struct RoadData {
    direction: TrafficDirection,
    car_type: CarType,
    cars: Vec<Car>,
    spawn_interval: u32,
    ticks_since_spawn: u32,
}

#[derive(Debug, Clone)]
enum LaneType {
    Road(RoadData),
    SafeZone,
}

#[derive(Debug, Clone)]
struct Lane {
    lane_type: LaneType,
}

struct Player {
    x: u16,
    lane_idx: usize,
}

struct App {
    state: AppState,
    player: Player,
    score: u32,
    max_lane_idx: usize,
    lanes: Vec<Lane>,
    next_block_type: LaneType,
    next_block_remaining: usize,
    tick_count: u32,
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl App {
    fn road_lane_range(&self, score: usize) -> std::ops::RangeInclusive<usize> {
        if score <= 20 {
            2..=5
        } else if score <= 60 {
            3..=8
        } else if score <= 100 {
            4..=10
        } else {
            let shift = (score - 100) / 50;
            (4 + shift)..=(10 + shift)
        }
    }

    fn new(width: u16, height: u16) -> Self {
        let mut app = Self {
            state: AppState::MainMenu,
            player: Player {
                x: width / 2,
                lane_idx: 0,
            },
            score: 0,
            max_lane_idx: 0,
            lanes: Vec::new(),
            next_block_type: LaneType::SafeZone,
            next_block_remaining: 0,
            tick_count: 0,
        };

        // Spawn zone: 5 safe zones
        for _ in 0..5 {
            app.lanes.push(Lane { lane_type: LaneType::SafeZone });
        }

        let mut rng = rand::thread_rng();
        app.next_block_remaining = rng.gen_range(app.road_lane_range(app.lanes.len()));
        app.next_block_type = LaneType::Road(RoadData {
            direction: TrafficDirection::Right,
            car_type: CarType::Red,
            cars: Vec::new(),
            spawn_interval: 50,
            ticks_since_spawn: 0,
        });

        app.ensure_lanes(0, width, height as usize);
        app
    }

    fn reset(&mut self, width: u16, height: u16) {
        self.player = Player {
            x: width / 2,
            lane_idx: 0,
        };
        self.score = 0;
        self.max_lane_idx = 0;
        self.lanes.clear();
        self.tick_count = 0;
        self.state = AppState::Playing;

        // Spawn zone: 5 safe zones
        for _ in 0..5 {
            self.lanes.push(Lane { lane_type: LaneType::SafeZone });
        }

        let mut rng = rand::thread_rng();
        self.next_block_remaining = rng.gen_range(self.road_lane_range(self.lanes.len()));
        self.next_block_type = LaneType::Road(RoadData {
            direction: TrafficDirection::Right,
            car_type: CarType::Red,
            cars: Vec::new(),
            spawn_interval: 50,
            ticks_since_spawn: 0,
        });

        self.ensure_lanes(0, width, height as usize);
    }

    fn ensure_lanes(&mut self, player_lane_idx: usize, viewport_width: u16, viewport_height: usize) {
        let target = player_lane_idx + viewport_height * 2 + 20;
        let mut rng = rand::thread_rng();
        while self.lanes.len() < target {
            if self.next_block_remaining == 0 {
                match self.next_block_type {
                    LaneType::Road(_) => {
                        self.next_block_remaining = rng.gen_range(1..=2);
                        self.next_block_type = LaneType::SafeZone;
                    }
                    LaneType::SafeZone => {
                        self.next_block_remaining = rng.gen_range(self.road_lane_range(self.lanes.len()));
                        let direction = if rng.gen_bool(0.5) {
                            TrafficDirection::Left
                        } else {
                            TrafficDirection::Right
                        };
                        let car_type = CarType::random();
                        let spawn_interval = rng.gen_range(45..=85);
                        self.next_block_type = LaneType::Road(RoadData {
                            direction,
                            car_type,
                            cars: Vec::new(),
                            spawn_interval,
                            ticks_since_spawn: 0,
                        });
                    }
                }
            }

            let lane_type = match self.next_block_type {
                LaneType::SafeZone => LaneType::SafeZone,
                LaneType::Road(ref template) => {
                    let mut cars = Vec::new();
                    let num_cars = rng.gen_range(1..=2);
                    let lane_car_type = CarType::random();
                    for _ in 0..num_cars {
                        let car_x = rng.gen_range(0..viewport_width) as i16;
                        let mut attempts = 0;
                        let mut x = car_x;
                        while attempts < 10 && cars.iter().any(|c: &Car| (c.x - x).abs() < 3) {
                            x = rng.gen_range(0..viewport_width) as i16;
                            attempts += 1;
                        }
                        cars.push(Car { x, car_type: lane_car_type });
                    }
                    LaneType::Road(RoadData {
                        direction: template.direction,
                        car_type: lane_car_type,
                        cars,
                        spawn_interval: template.spawn_interval,
                        ticks_since_spawn: rng.gen_range(0..template.spawn_interval),
                    })
                }
            };

            self.lanes.push(Lane { lane_type });
            self.next_block_remaining -= 1;
        }
    }

    fn move_player(&mut self, dir: Direction, width: u16, height: u16) {
        if self.state != AppState::Playing {
            return;
        }
        match dir {
            Direction::Up => {
                self.player.lane_idx += 1;
                if self.player.lane_idx > self.max_lane_idx {
                    self.score += 1;
                    self.max_lane_idx = self.player.lane_idx;
                }
            }
            Direction::Down => {
                if self.player.lane_idx > 0 {
                    self.player.lane_idx -= 1;
                }
            }
            Direction::Left => {
                if self.player.x > 0 {
                    self.player.x -= 1;
                }
            }
            Direction::Right => {
                if self.player.x < width.saturating_sub(2) {
                    self.player.x += 1;
                }
            }
        }
        self.ensure_lanes(self.player.lane_idx, width, height as usize);
        self.check_collisions();
    }

    fn check_collisions(&mut self) {
        let player_lane = self.player.lane_idx;
        if let Some(lane) = self.lanes.get(player_lane) {
            if let LaneType::Road(ref road) = lane.lane_type {
                for car in &road.cars {
                    // Crab is 2 wide, car is 3 wide.
                    let px = self.player.x as i16;
                    if px <= car.x + 2 && px + 1 >= car.x {
                        self.state = AppState::GameOver;
                        break;
                    }
                }
            }
        }
    }

    fn tick(&mut self, width: u16) {
        if self.state != AppState::Playing {
            return;
        }

        self.tick_count += 1;
        let w_i16 = width as i16;

        for lane in self.lanes.iter_mut() {
            if let LaneType::Road(ref mut road) = lane.lane_type {
                // 1. Spawning
                road.ticks_since_spawn += 1;
                if road.ticks_since_spawn >= road.spawn_interval {
                    let can_spawn = match road.direction {
                        TrafficDirection::Right => !road.cars.iter().any(|c| c.x < 1),
                        TrafficDirection::Left => !road.cars.iter().any(|c| c.x > w_i16 - 4),
                    };
                    if can_spawn {
                        let spawn_x = match road.direction {
                            TrafficDirection::Right => -3,
                            TrafficDirection::Left => w_i16,
                        };
                        road.cars.push(Car { x: spawn_x, car_type: road.car_type });
                        road.ticks_since_spawn = 0;
                    }
                }

                // Sort cars so the leading car is processed first to handle queuing correctly
                match road.direction {
                    TrafficDirection::Right => {
                        road.cars.sort_by(|a, b| b.x.cmp(&a.x));
                    }
                    TrafficDirection::Left => {
                        road.cars.sort_by(|a, b| a.x.cmp(&b.x));
                    }
                }

                // 2. Movement
                for i in 0..road.cars.len() {
                    let car_speed = road.cars[i].car_type.speed();
                    if self.tick_count % car_speed == 0 {
                        let next_x = match road.direction {
                            TrafficDirection::Right => road.cars[i].x + 1,
                            TrafficDirection::Left => road.cars[i].x - 1,
                        };

                        // Check collision with the car directly in front (width 3)
                        let mut can_move = true;
                        if i > 0 {
                            let ahead_x = road.cars[i - 1].x;
                            match road.direction {
                                TrafficDirection::Right => {
                                    if next_x + 2 >= ahead_x {
                                        can_move = false;
                                    }
                                }
                                TrafficDirection::Left => {
                                    if next_x <= ahead_x + 2 {
                                        can_move = false;
                                    }
                                }
                            }
                        }

                        if can_move {
                            road.cars[i].x = next_x;
                        }
                    }
                }

                // 3. Cleanup off-screen cars
                road.cars.retain(|car| {
                    match road.direction {
                        TrafficDirection::Right => car.x <= w_i16,
                        TrafficDirection::Left => car.x >= -3,
                    }
                });
            }
        }

        self.check_collisions();
    }
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);

    let tick_rate = std::time::Duration::from_millis(50);
    let mut last_tick = std::time::Instant::now();

    loop {
        let current_size = terminal.size()?;
        terminal.draw(|frame| {
            let area = frame.area();
            let width = area.width;
            let height = area.height;
            let start_y = height.saturating_sub(1) as i32;
            let midpoint = (height / 2) as i32;

            // Clamp player X to viewport size
            app.player.x = app.player.x.min(width.saturating_sub(2));

            // Calculate player Y in screen-space terms (logical)
            let player_y = start_y - app.player.lane_idx as i32;

            // Calculate camera offset
            let camera_offset_y = if player_y < midpoint {
                player_y - midpoint
            } else {
                0
            };

            // 1. Draw background lanes (always drawn to show frozen state behind pop-up during GameOver)
            for (i, lane) in app.lanes.iter().enumerate() {
                let lane_y = start_y - (i as i32);
                let visual_y = lane_y - camera_offset_y;
                if visual_y >= 0 && visual_y < height as i32 {
                    let color = match lane.lane_type {
                        LaneType::Road(_) => Color::DarkGray,
                        LaneType::SafeZone => Color::Yellow,
                    };
                    let rect = Rect::new(0, visual_y as u16, width, 1);
                    frame.render_widget(Block::default().style(Style::default().bg(color)), rect);

                    // Draw traffic on road lanes
                    if let LaneType::Road(ref road) = lane.lane_type {
                        for car in &road.cars {
                            if car.x >= 0 && car.x < width as i16 {
                                let car_rect = Rect::new(car.x as u16, visual_y as u16, 3, 1);
                                let car_color = car.car_type.color();
                                let car_symbol = match road.direction {
                                    TrafficDirection::Right => "[o>",
                                    TrafficDirection::Left => "<o]",
                                };
                                frame.render_widget(
                                    Paragraph::new(car_symbol).style(Style::default().fg(car_color)),
                                    car_rect,
                                );
                            }
                        }
                    }
                }
            }

            // 2. Draw crab at its visual Y coordinate
            let visual_player_y = player_y - camera_offset_y;
            if visual_player_y >= 0 && visual_player_y < height as i32 {
                let crab_rect = Rect::new(app.player.x, visual_player_y as u16, 2, 1);
                frame.render_widget(Paragraph::new("🦀"), crab_rect);
            }

            // 3. Draw Score in the top left corner
            let score_text = format!(" Score: {} ", app.score);
            let score_rect = Rect::new(0, 0, 15, 1);
            frame.render_widget(Paragraph::new(score_text).style(Style::default().fg(Color::Black).bg(Color::White)), score_rect);

            // 4. Overlays depending on state
            match app.state {
                AppState::MainMenu => {
                    let menu_rect = centered_rect(42, 10, area);
                    frame.render_widget(Clear, menu_rect);
                    let block = Block::default()
                        .title(" CRABBER TUI ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .border_style(Style::default().fg(Color::Green));
                    let text = "\n  Help 🦀 cross the busy roads!\n\n  Controls:\n  - WASD / Arrow Keys / HJKL to Move\n\n  Press ENTER or SPACE to Start\n  Press Q to Quit";
                    let paragraph = Paragraph::new(text)
                        .block(block)
                        .style(Style::default().fg(Color::White).bg(Color::Black));
                    frame.render_widget(paragraph, menu_rect);
                }
                AppState::GameOver => {
                    let popup_rect = centered_rect(32, 8, area);
                    frame.render_widget(Clear, popup_rect);
                    let block = Block::default()
                        .title(" GAME OVER ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .border_style(Style::default().fg(Color::Red));
                    let text = format!(
                        "\n   Final Score: {}\n\n   Press R to Restart\n   Press Q to Quit",
                        app.score
                    );
                    let paragraph = Paragraph::new(text)
                        .block(block)
                        .style(Style::default().fg(Color::White).bg(Color::Black));
                    frame.render_widget(paragraph, popup_rect);
                }
                AppState::Playing => {}
            }
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(std::time::Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match app.state {
                        AppState::MainMenu => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                app.state = AppState::Playing;
                            }
                            _ => {}
                        },
                        AppState::Playing => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Char('k') | KeyCode::Char('K') => {
                                app.move_player(Direction::Up, current_size.width, current_size.height);
                            }
                            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('j') | KeyCode::Char('J') => {
                                app.move_player(Direction::Down, current_size.width, current_size.height);
                            }
                            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('h') | KeyCode::Char('H') => {
                                app.move_player(Direction::Left, current_size.width, current_size.height);
                            }
                            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('l') | KeyCode::Char('L') => {
                                app.move_player(Direction::Right, current_size.width, current_size.height);
                            }
                            _ => {}
                        },
                        AppState::GameOver => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                app.reset(current_size.width, current_size.height);
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick(current_size.width);
            last_tick = std::time::Instant::now();
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
