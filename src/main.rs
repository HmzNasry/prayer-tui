use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
};
use ratatui::{prelude::*, widgets::*};
use std::{
    env,
    io::{self},
    time::Duration,
};
use tokio::process::Command;

use chrono::{Local, NaiveTime};
use clap::Parser;

mod app;
use app::{config, prayers, state, tui::Tui};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Run the application in background mode, sending only notifications.
    #[arg(short, long)]
    background: bool,
}

async fn send_notification(summary: &str, body: &str, dbus_addr: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut command = Command::new("notify-send");
    command.arg(summary).arg(body);

    if let Some(addr) = dbus_addr {
        command.env("DBUS_SESSION_BUS_ADDRESS", addr);
    }

    let output = command.output().await?;

    if !output.status.success() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        eprintln!("notify-send failed: {}", stderr_str);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to send notification: {}", stderr_str),
        )));
    }

    Ok(())
}

async fn run_background_notifications(dbus_addr: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::load_config()?;
    let mut prayer_times = prayers::get_prayer_times(&config).await?;
    let mut app_state = state::load_app_state()?;
    app_state.notified_prayers.clear();
    state::save_app_state(&app_state)?;

    loop {
        let now = Local::now();
        let now_time = now.time();
        let today_str = now.format("%Y-%m-%d").to_string();

        if app_state.date != today_str {
            prayer_times = prayers::get_prayer_times(&config).await?;
            app_state.date = today_str;
            app_state.notified_prayers.clear();
            state::save_app_state(&app_state)?;
        }

        let prayers_vec = vec![
            ("Fajr", &prayer_times.fajr),
            ("Sunrise", &prayer_times.sunrise),
            ("Dhuhr", &prayer_times.dhuhr),
            ("Asr", &prayer_times.asr),
            ("Sunset", &prayer_times.sunset),
            ("Maghrib", &prayer_times.maghrib),
            ("Isha", &prayer_times.isha),
        ];

        for (name, time_str) in prayers_vec {
            let prayer_time = NaiveTime::parse_from_str(time_str, "%H:%M").unwrap();
            if now_time >= prayer_time && !app_state.notified_prayers.contains(&name.to_string()) {
                let summary = if name == "Sunrise" {
                    "It's Sunrise"
                } else if name == "Sunset" {
                    "It's Sunset"
                } else {
                    "Prayer Time"
                };

                let body = if name == "Sunrise" || name == "Sunset" {
                    ""
                } else {
                    &format!("It's time for {} prayer", name)
                };
                send_notification(summary, body, dbus_addr.clone()).await?;
                app_state.notified_prayers.push(name.to_string());
                state::save_app_state(&app_state)?;
            }
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let cli = Cli::parse();

    if cli.background {
        let dbus_addr = env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        run_background_notifications(dbus_addr).await?;
    } else {
        println!("DBUS_SESSION_BUS_ADDRESS: {:?}", env::var("DBUS_SESSION_BUS_ADDRESS"));
        let mut tui = Tui::new()?;
        tui.enter()?;

        if let Err(e) = run_app(&mut tui).await {
            eprintln!("Error: {}", e);
        }

        tui.exit()?;
    }

    Ok(())
}

async fn run_app(tui: &mut Tui) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::load_config()?;
    let mut prayer_times = prayers::get_prayer_times(&config).await?;
    let mut app_state = state::load_app_state()?;

    loop {
        let now = Local::now();
        let now_time = now.time();
        let today_str = now.format("%Y-%m-%d").to_string();

        if app_state.date != today_str {
            prayer_times = prayers::get_prayer_times(&config).await?;
            app_state.date = today_str;
            app_state.notified_prayers.clear();
            state::save_app_state(&app_state)?;
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

        let mut last_prayer_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let prayers = vec![
            ("Fajr", NaiveTime::parse_from_str(&prayer_times.fajr, "%H:%M").unwrap()),
            ("Dhuhr", NaiveTime::parse_from_str(&prayer_times.dhuhr, "%H:%M").unwrap()),
            ("Asr", NaiveTime::parse_from_str(&prayer_times.asr, "%H:%M").unwrap()),
            ("Maghrib", NaiveTime::parse_from_str(&prayer_times.maghrib, "%H:%M").unwrap()),
            ("Isha", NaiveTime::parse_from_str(&prayer_times.isha, "%H:%M").unwrap()),
        ].into_iter().map(|(name, time)| {
            if time < last_prayer_time {
                (name, time + chrono::Duration::hours(12))
            } else {
                last_prayer_time = time;
                (name, time)
            }
        }).collect::<Vec<(&str, NaiveTime)>>();
        let mut current_prayer_index: Option<usize> = None;
        for (i, (_, prayer_time)) in prayers.iter().enumerate() {
            if now_time >= *prayer_time {
                current_prayer_index = Some(i);
            } else {
                break;
            }
        }
        let actual_current_prayer_index = current_prayer_index.unwrap_or(prayers.len() - 1);

        let next_prayer_index = (actual_current_prayer_index + 1) % prayers.len();
        let (next_prayer_name, next_prayer_time) = prayers[next_prayer_index];

        let time_remaining = if now_time > next_prayer_time {
            (NaiveTime::from_hms_opt(23, 59, 59).unwrap() - now_time) + next_prayer_time.signed_duration_since(NaiveTime::from_hms_opt(0,0,0).unwrap()) + chrono::Duration::seconds(1)
        } else {
            next_prayer_time.signed_duration_since(now_time)
        };

        let hours = time_remaining.num_hours();
        let minutes = time_remaining.num_minutes() % 60;
        let seconds = time_remaining.num_seconds() % 60;

        for (name, prayer_time) in &prayers {
            if now_time >= *prayer_time && !app_state.notified_prayers.contains(&name.to_string()) {
                send_notification("Prayer Time", &format!("It's time for {} prayer", name), None).await?;
                app_state.notified_prayers.push(name.to_string());
                state::save_app_state(&app_state)?;
            }
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

            let total_prayer_columns_width = 5 * 16;

            let padding_x = (main_layout[3].width.saturating_sub(total_prayer_columns_width)) / 2;

            let prayer_area = Rect::new(
                main_layout[3].x + padding_x,
                main_layout[3].y,
                total_prayer_columns_width,
                main_layout[3].height,
            );

            let prayer_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(16),
                    Constraint::Length(16),
                    Constraint::Length(16),
                    Constraint::Length(16),
                    Constraint::Length(16),
                ])
                .split(prayer_area);

            let sunrise_time = NaiveTime::parse_from_str(&prayer_times.sunrise, "%H:%M").unwrap();
            let sunset_time = NaiveTime::parse_from_str(&prayer_times.sunset, "%H:%M").unwrap();

            for (i, (name, time)) in prayers.iter().enumerate() {
                let _prayer_index = i + 1;
                let is_current_prayer = i == actual_current_prayer_index;
                let mut style = Style::default().add_modifier(Modifier::BOLD);

                if is_current_prayer {
                    if *name == "Fajr" && now_time >= sunrise_time && now_time < NaiveTime::parse_from_str(&prayer_times.dhuhr, "%H:%M").unwrap() {
                    } else if (*name == "Asr" || *name == "Maghrib") && now_time >= sunset_time && now_time < NaiveTime::parse_from_str(&prayer_times.isha, "%H:%M").unwrap() {
                    } else {
                        style = style.fg(Color::Green);
                    }
                }

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

            let sun_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Ratio(1, 4),
                    Constraint::Ratio(1, 4),
                    Constraint::Percentage(40),
                ])
                .split(main_layout[4]);

            let sunrise_time = NaiveTime::parse_from_str(&prayer_times.sunrise, "%H:%M").unwrap();
            let sunset_time = NaiveTime::parse_from_str(&prayer_times.sunset, "%H:%M").unwrap();

            let is_sunrise = now_time >= sunrise_time && now_time < sunset_time;
            let is_sunset = now_time >= sunset_time && now_time < NaiveTime::parse_from_str(&prayer_times.isha, "%H:%M").unwrap();

            let mut sunrise_style = Style::default().add_modifier(Modifier::BOLD);
            if is_sunrise {
                let maghrib_time = NaiveTime::parse_from_str(&prayer_times.maghrib, "%H:%M").unwrap();
                if now_time < maghrib_time {
                    sunrise_style = sunrise_style.fg(Color::Green);
                }
            }

            let mut sunset_style = Style::default().add_modifier(Modifier::BOLD);
            if is_sunset {
                let maghrib_time = NaiveTime::parse_from_str(&prayer_times.maghrib, "%H:%M").unwrap();
                if now_time >= maghrib_time + chrono::Duration::minutes(1) {
                    sunset_style = sunset_style.fg(Color::Green);
                }
            }

            let sunrise_box_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Percentage(30),
                ])
                .split(sun_layout[1]);

            frame.render_widget(
                Paragraph::new("Sunrise")
                    .style(sunrise_style)
                    .alignment(Alignment::Center),
                sunrise_box_layout[1],
            );
            frame.render_widget(
                Paragraph::new(sunrise_time.format("%I:%M %p").to_string())
                    .style(sunrise_style)
                    .alignment(Alignment::Center),
                sunrise_box_layout[2],
            );

            let sunset_box_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Percentage(30),
                ])
                .split(sun_layout[2]);

            frame.render_widget(
                Paragraph::new("Sunset")
                    .style(sunset_style)
                    .alignment(Alignment::Center),
                sunset_box_layout[1],
            );
            frame.render_widget(
                Paragraph::new(sunset_time.format("%I:%M %p").to_string())
                    .style(sunset_style)
                    .alignment(Alignment::Center),
                sunset_box_layout[2],
            );
        })?;

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}