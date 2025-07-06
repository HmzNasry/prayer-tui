use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, stdout, Read, Write},
    path::Path,
    time::Duration,
};

use chrono::{Local, NaiveTime, Timelike};
use notify_rust::Notification;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    city: String,
    country: String,
    method: u8,
    madhab: u8,
}

#[derive(Deserialize, Debug)]
struct Data {
    data: Timings,
}

#[derive(Deserialize, Debug)]
struct Timings {
    timings: PrayerTimes,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct PrayerTimes {
    #[serde(rename = "Fajr")]
    fajr: String,
    #[serde(rename = "Dhuhr")]
    dhuhr: String,
    #[serde(rename = "Asr")]
    asr: String,
    #[serde(rename = "Maghrib")]
    maghrib: String,
    #[serde(rename = "Isha")]
    isha: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppState {
    notified_prayers: Vec<String>,
    date: String,
}

async fn get_prayer_times(config: &Config) -> Result<PrayerTimes, Error> {
    let url = format!(
        "https://api.aladhan.com/v1/timingsByCity?city={}&country={}&method={}&madhab={}",
        config.city,
        config.country,
        config.method,
        config.madhab
    );
    let response = reqwest::get(&url).await?.json::<Data>().await?;
    Ok(response.data.timings)
}

struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    fn new() -> Result<Self, io::Error> {
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(Self { terminal })
    }

    fn enter(&self) -> Result<(), io::Error> {
        terminal::enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn exit(&self) -> Result<(), io::Error> {
        stdout().execute(LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }
}

fn load_config() -> Result<Config, io::Error> {
    let path = Path::new("config.toml");
    if !path.exists() {
        let default_config = Config {
            city: "Seattle".to_string(),
            country: "US".to_string(),
            method: 2,
            madhab: 1,
        };
        let toml = toml::to_string(&default_config).unwrap();
        let mut file = File::create(path)?;
        file.write_all(toml.as_bytes())?;
        return Ok(default_config);
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents).unwrap();
    Ok(config)
}

fn load_app_state() -> Result<AppState, io::Error> {
    let path = Path::new("state.json");
    if !path.exists() {
        return Ok(AppState {
            notified_prayers: Vec::new(),
            date: Local::now().format("%Y-%m-%d").to_string(),
        });
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let state: AppState = serde_json::from_str(&contents)?;
    Ok(state)
}

fn save_app_state(state: &AppState) -> Result<(), io::Error> {
    let path = Path::new("state.json");
    let json = serde_json::to_string(state)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tui = Tui::new()?;
    tui.enter()?;

    if let Err(e) = run_app(&mut tui).await {
        eprintln!("Error: {}", e);
    }

    tui.exit()?;
    Ok(())
}

async fn run_app(tui: &mut Tui) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let mut prayer_times = get_prayer_times(&config).await?;
    let mut app_state = load_app_state()?;

    loop {
        let now = Local::now();
        let now_time = now.time();
        let today_str = now.format("%Y-%m-%d").to_string();

        // Reload prayer times at midnight
        if app_state.date != today_str {
            prayer_times = get_prayer_times(&config).await?;
            app_state.date = today_str;
            app_state.notified_prayers.clear();
            save_app_state(&app_state)?;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let prayers = vec![
            ("Fajr", NaiveTime::parse_from_str(&prayer_times.fajr, "%H:%M")?),            ("Dhuhr", NaiveTime::parse_from_str(&prayer_times.dhuhr, "%H:%M")?),
            ("Asr", NaiveTime::parse_from_str(&prayer_times.asr, "%H:%M")?),
            ("Maghrib", NaiveTime::parse_from_str(&prayer_times.maghrib, "%H:%M")?),
            ("Isha", NaiveTime::parse_from_str(&prayer_times.isha, "%H:%M")?),
        ];

        let mut current_prayer_index = 0;
        for (i, (_, prayer_time)) in prayers.iter().enumerate() {
            if now_time >= *prayer_time {
                current_prayer_index = i;
            }
        }

        let next_prayer_index = (current_prayer_index + 1) % prayers.len();
        let (next_prayer_name, next_prayer_time) = prayers[next_prayer_index];

        let time_remaining = if now_time > next_prayer_time {
            (NaiveTime::from_hms_opt(23, 59, 59).unwrap() - now_time) + next_prayer_time.signed_duration_since(NaiveTime::from_hms_opt(0,0,0).unwrap()) + chrono::Duration::seconds(1)
        } else {
            next_prayer_time.signed_duration_since(now_time)
        };

        let hours = time_remaining.num_hours();
        let minutes = time_remaining.num_minutes() % 60;
        let seconds = time_remaining.num_seconds() % 60;

        if time_remaining.num_seconds() > 0 && time_remaining.num_seconds() <= 1 && !app_state.notified_prayers.contains(&next_prayer_name.to_string()) {
            Notification::new()
                .summary(&format!("{} Time", next_prayer_name))
                .body(&format!("It's time for {} prayer.", next_prayer_name))
                .show()?;
            app_state.notified_prayers.push(next_prayer_name.to_string());
            save_app_state(&app_state)?;
        }

        tui.terminal.draw(|frame| {
            let size = frame.size();
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Length(2),
                    Constraint::Length(3),
                    Constraint::Percentage(40),
                ])
                .split(size);

            let countdown_style = Style::default().add_modifier(Modifier::BOLD);
            let countdown_text = format!(
                "{}: {:02}:{:02}:{:02}",
                next_prayer_name, hours, minutes, seconds
            );
            
            let countdown_paragraph = Paragraph::new(countdown_text)
                .style(countdown_style)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::NONE));

            frame.render_widget(countdown_paragraph, main_layout[1]);

            let prayer_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 5); 5])
                .split(main_layout[3]);

            for (i, (name, time)) in prayers.iter().enumerate() {
                let is_current_prayer = i == current_prayer_index;
                let style = if is_current_prayer {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::BOLD)
                };

                let prayer_box_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(30),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Percentage(30),
                    ])
                    .split(prayer_layout[i]);

                frame.render_widget(
                    Paragraph::new(*name)
                        .style(style)
                        .alignment(Alignment::Center),
                    prayer_box_layout[1],
                );
                frame.render_widget(
                    Paragraph::new(time.format("%I:%M %p").to_string())
                        .style(style)
                        .alignment(Alignment::Center),
                    prayer_box_layout[2],
                );
            }
        })?;

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}
