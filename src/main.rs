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

mod app;
use app::{config, prayers, state, tui::Tui};

async fn send_notification(summary: &str, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dbus_addr = env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();
    let status = Command::new("notify-send")
        .arg(summary)
        .arg(body)
        .env("DBUS_SESSION_BUS_ADDRESS", dbus_addr)
        .status()
        .await?;

    if !status.success() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "Failed to send notification",
        )));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("DBUS_SESSION_BUS_ADDRESS: {:?}", env::var("DBUS_SESSION_BUS_ADDRESS"));
    let mut tui = Tui::new()?;
    tui.enter()?;

    if let Err(e) = run_app(&mut tui).await {
        eprintln!("Error: {}", e);
    }

    tui.exit()?;
    Ok(())
}

async fn run_app(tui: &mut Tui) -> Result<(), Box<dyn std::error::Error>> {
    send_notification("Prayer TUI", "Starting up...").await?;
    let config = config::load_config()?;
    let mut prayer_times = prayers::get_prayer_times(&config).await?;
    let mut app_state = state::load_app_state()?;

    loop {
        let now = Local::now();
        let now_time = now.time();
        let today_str = now.format("%Y-%m-%d").to_string();

        // Reload prayer times at midnight
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
        let mut current_prayer_index = prayers.len() - 1;
        for (i, (_, prayer_time)) in prayers.iter().enumerate() {
            if now_time >= *prayer_time {
                current_prayer_index = i;
            } else {
                break;
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

        for (name, prayer_time) in &prayers {
            if now_time >= *prayer_time && !app_state.notified_prayers.contains(&name.to_string()) {
                send_notification("Prayer Time", &format!("It's time for {} prayer", name)).await?;
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
