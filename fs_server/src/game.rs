use clap::error::ContextKind;
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use log::{debug, error, info, warn};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    ops::Add,
    time::{Duration, Instant},
};
use tui::{
    backend::Backend,
    layout::{Constraint, Layout},
    style::Style,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use tui_logger::{TuiLoggerSmartWidget, TuiWidgetState};

use super::world::ServerChunk;
use fs_common::game::{
    common::{
        cli::{CLArgs, CLSubcommand},
        commands::CommandHandler,
        networking::{Packet, PacketType},
        world::{Chunk, ChunkState, CHUNK_AREA},
        FileHelper,
    },
    BuildData, GameData,
};

pub struct ServerGame(pub GameData<ServerChunk>);

impl ServerGame {
    pub fn new(file_helper: FileHelper, build_data: BuildData) -> Self {
        Self(GameData::new(file_helper, build_data))
    }

    #[profiling::function]
    pub fn run<TB: Backend>(
        &mut self,
        args: &CLArgs,
        term: &mut Terminal<TB>,
    ) -> Result<(), String> {
        tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
        tui_logger::set_default_level(log::LevelFilter::Trace);
        if !self.0.file_helper.game_path("logs/").exists() {
            info!("logs dir missing, creating it...");
            std::fs::create_dir_all(self.0.file_helper.game_path("logs/"))
                .expect("Failed to create logs dir:");
        }
        tui_logger::set_log_file(
            self.0
                .file_helper
                .game_path("logs/server_latest.log")
                .to_str()
                .expect("Server log path must be UTF-8."),
        )
        .unwrap();

        term.clear().unwrap();

        let CLSubcommand::Server { port } = args.subcommand.as_ref().unwrap();
        let net_listener =
            TcpListener::bind(format!("127.0.0.1:{port}")).map_err(|e| e.to_string())?;
        net_listener
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;

        info!(target: "", "Server listening on port {}...", port);

        let mut connections: Vec<(TcpStream, SocketAddr)> = Vec::new();

        let mut prev_tick_time = std::time::Instant::now();
        let mut prev_tick_physics_time = std::time::Instant::now();

        let mut last_frame = Instant::now();
        let mut counter_last_frame = Instant::now();

        let mut do_tick_next = false;
        let mut do_tick_physics_next = false;

        let mut physics_ticks = 0;

        let mut input: String = String::new();
        let mut tui_widget_state = TuiWidgetState::new();
        tui_widget_state.transition(&tui_logger::TuiWidgetEvent::HideKey);

        let mut command_handler = CommandHandler::new();

        'mainLoop: loop {
            if let Ok((mut stream, addr)) = net_listener.accept() {
                info!("Incoming Connection: {}", addr.to_string());
                stream.set_nonblocking(false).unwrap();
                if let Some(w) = &self.0.world {
                    for ci in unsafe { w.chunk_handler.manager.raw().iter() } {
                        // println!("Writing SyncChunkPacket");
                        let (chunk_x, chunk_y) = *ci.0;
                        let packet = Packet {
                            packet_type: PacketType::SyncChunkPacket {
                                chunk_x,
                                chunk_y,
                                pixels: ci.1.pixels().as_ref().unwrap().to_vec(),
                                colors: ci.1.colors().to_vec(),
                            },
                        };
                        // let buf = serde_json::to_string(&packet).unwrap().into_bytes();
                        // let size_buf = serde_json::to_string(&(buf.len() as u32)).unwrap().into_bytes();
                        let buf = bincode::serialize(&packet).unwrap();
                        let size_buf = bincode::serialize(&(buf.len() as u32)).unwrap();
                        stream.write_all(&size_buf).unwrap();
                        stream.flush().unwrap();
                        stream.write_all(&buf).unwrap();
                        stream.flush().unwrap();

                        // println!("Wrote SyncChunkPacket");
                    }
                }
                stream.set_nonblocking(true).unwrap();
                connections.push((stream, addr));
            }

            for c in &mut connections {
                let mut buf = [0; 4];
                if c.0.read_exact(&mut buf).is_ok() {
                    let size: u32 = bincode::deserialize(&buf).unwrap();
                    debug!("Incoming packet, size = {}.", size);

                    let mut buf = Vec::with_capacity(size as usize);

                    debug!("read_to_end...");
                    match std::io::Read::by_ref(&mut c.0)
                        .take(u64::from(size))
                        .read_to_end(&mut buf)
                    {
                        Ok(_) => {
                            debug!("Read {} bytes.", buf.len());
                            let p: Packet =
                                bincode::deserialize(&buf).expect("Failed to deserialize packet.");
                            debug!(
                                "Recieved packet from {:?}: {:?}",
                                c.1,
                                match p.packet_type {
                                    PacketType::SyncChunkPacket { .. } => "SyncChunkPacket",
                                    PacketType::SyncLiquidFunPacket { .. } => "SyncLiquidFunPacket",
                                }
                            );
                        },
                        Err(e) => {
                            // TODO: this needs to be handled correctly like in client::game
                            //         since when read_to_end fails, it can still have read some of the bytes
                            panic!("read_to_end failed: {e}");
                        },
                    }
                }
            }

            let now = std::time::Instant::now();

            // tick

            let can_tick = self.0.settings.tick;

            if do_tick_next && can_tick {
                if now.saturating_duration_since(prev_tick_time).as_millis() > 500 {
                    warn!(target: "", "50+ ms behind, skipping some ticks to catch up...");
                    prev_tick_time = now;
                } else {
                    prev_tick_time = prev_tick_time.add(Duration::from_nanos(
                        1_000_000_000 / u64::from(self.0.settings.tick_speed),
                    ));
                }
                let st = Instant::now();
                self.tick();

                if self.0.tick_time % 4 == 0 {
                    if let Some(w) = &self.0.world {
                        let mut n = 0;
                        for ci in unsafe { w.chunk_handler.manager.raw().iter() } {
                            n += 1;
                            if ci.1.state() == ChunkState::Active
                                && ci.1.dirty
                                && n % (self.0.tick_time / 4) % 4 == 0
                            {
                                for c in &mut connections {
                                    // println!("Writing SyncChunkPacket");
                                    let (chunk_x, chunk_y) = *ci.0;
                                    let pixels_vec = ci.1.pixels().as_ref().unwrap().to_vec();
                                    let colors_vec = ci.1.colors().to_vec();

                                    assert!(
                                        pixels_vec.len() == CHUNK_AREA,
                                        "Almost sent wrong size pixels Vec: {} (expected {})",
                                        pixels_vec.len(),
                                        CHUNK_AREA
                                    );

                                    if colors_vec.len() != CHUNK_AREA * 4 {
                                        panic!(
                                            "Almost sent wrong size colors Vec: {} (expected {})",
                                            colors_vec.len(),
                                            CHUNK_AREA * 4
                                        );
                                    }

                                    let packet = Packet {
                                        packet_type: PacketType::SyncChunkPacket {
                                            chunk_x,
                                            chunk_y,
                                            pixels: pixels_vec,
                                            colors: colors_vec,
                                        },
                                    };
                                    // let buf = serde_json::to_string(&packet).unwrap().into_bytes();
                                    // let size_buf = serde_json::to_string(&(buf.len() as u32)).unwrap().into_bytes();
                                    let buf = bincode::serialize(&packet).unwrap();
                                    let size_buf = bincode::serialize(&(buf.len() as u32)).unwrap();

                                    c.0.set_nonblocking(false).unwrap();
                                    c.0.write_all(&size_buf).unwrap();
                                    c.0.flush().unwrap();
                                    c.0.write_all(&buf).unwrap();
                                    c.0.flush().unwrap();
                                    c.0.set_nonblocking(true).unwrap();

                                    // println!("Wrote SyncChunkPacket");
                                }
                            }
                        }
                    }

                    // TODO: come up with a good way to merge this loop with the one right above
                    if let Some(w) = &mut self.0.world {
                        for ci in unsafe { w.chunk_handler.manager.raw_mut().iter_mut() } {
                            if ci.1.state() == ChunkState::Active && ci.1.dirty {
                                ci.1.dirty = false;
                            }
                        }
                    }
                }

                self.0.fps_counter.tick_times.rotate_left(1);
                self.0.fps_counter.tick_times[self.0.fps_counter.tick_times.len() - 1] =
                    Instant::now().saturating_duration_since(st).as_nanos() as f32;

                if poll(Duration::from_millis(1)).unwrap() {
                    let event = read().unwrap();

                    match event {
                        Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers, .. })
                            if modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            break 'mainLoop;
                        },
                        Event::Key(KeyEvent { code, .. }) => {
                            match code {
                                KeyCode::Enter => {
                                    // handle
                                    let msg: String = input.drain(..).collect();
                                    info!(target: "", ">{}", msg);
                                    match command_handler.get_matches(msg.as_str()) {
                                        Ok(m) => {
                                            if m.subcommand_matches("shutdown").is_some() {
                                                break 'mainLoop;
                                            }
                                        },
                                        Err(e)
                                            if e.kind()
                                                == clap::error::ErrorKind::UnknownArgument =>
                                        {
                                            error!(target: "", "Found argument '{:?}' which wasn't expected, or isn't valid in this context.", e.context().find_map(|(k, v)| (k == ContextKind::InvalidArg).then_some(v)).unwrap());
                                        },
                                        Err(e)
                                            if e.kind() == clap::error::ErrorKind::DisplayHelp =>
                                        {
                                            info!(target: "", "{:?}", e.to_string());
                                        },
                                        Err(e) => {
                                            error!(target: "", "{}", e.to_string());
                                        },
                                    }
                                },
                                KeyCode::Char(c) => {
                                    input.push(c);
                                },
                                KeyCode::Backspace => {
                                    input.pop();
                                },
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                }

                let term_size = term.size().unwrap();
                term.backend_mut()
                    .set_cursor(2 + input.len() as u16, term_size.height - 2)
                    .unwrap();
                term.draw(|f| self.draw_terminal(f, &input, &mut tui_widget_state))
                    .unwrap();

                self.0.fps_counter.ticks += 1;
            }
            do_tick_next = can_tick
                && now.saturating_duration_since(prev_tick_time).as_nanos()
                    > 1_000_000_000 / u128::from(self.0.settings.tick_speed); // intended is 30 ticks per second

            // tick liquidfun

            let can_tick = self.0.settings.tick_physics;

            if do_tick_physics_next && can_tick {
                if now
                    .saturating_duration_since(prev_tick_physics_time)
                    .as_millis()
                    > 500
                {
                    warn!(target: "", "liquidfun 50+ ms behind, skipping some ticks to catch up...");
                    prev_tick_physics_time = now;
                } else {
                    prev_tick_physics_time = prev_tick_physics_time.add(Duration::from_nanos(
                        1_000_000_000 / u64::from(self.0.settings.tick_physics_speed),
                    ));
                }
                if let Some(w) = &mut self.0.world {
                    let st = Instant::now();
                    w.tick_physics(&self.0.settings);
                    physics_ticks += 1;

                    if physics_ticks % 10 == 0 {
                        // TODO: update for rapier/salva
                        // if let Some(particle_system) = w.lqf_world.get_particle_system_list() {
                        //     let particle_positions: &[Vec2] = particle_system.get_position_buffer();
                        //     let particle_velocities: &[Vec2] =
                        //         particle_system.get_velocity_buffer();
                        //     for c in &mut connections {
                        //         let packet = Packet {
                        //             packet_type: PacketType::SyncLiquidFunPacket {
                        //                 positions: particle_positions
                        //                     .iter()
                        //                     .map(|v2| PVec2 { x: v2.x, y: v2.y })
                        //                     .collect(),
                        //                 velocities: particle_velocities
                        //                     .iter()
                        //                     .map(|v2| PVec2 { x: v2.x, y: v2.y })
                        //                     .collect(),
                        //             },
                        //         };
                        //         // let buf = serde_json::to_string(&packet).unwrap().into_bytes();
                        //         // let size_buf = serde_json::to_string(&(buf.len() as u32)).unwrap().into_bytes();
                        //         let buf = bincode::serialize(&packet).unwrap();
                        //         let size_buf = bincode::serialize(&(buf.len() as u32)).unwrap();

                        //         c.0.set_nonblocking(false).unwrap();
                        //         c.0.write_all(&size_buf).unwrap();
                        //         c.0.flush().unwrap();
                        //         c.0.write_all(&buf).unwrap();
                        //         c.0.flush().unwrap();
                        //         c.0.set_nonblocking(true).unwrap();

                        //         // println!("Wrote SyncChunkPacket");
                        //     }
                        // }
                    }

                    self.0.fps_counter.tick_physics_times.rotate_left(1);
                    self.0.fps_counter.tick_physics_times
                        [self.0.fps_counter.tick_physics_times.len() - 1] =
                        Instant::now().saturating_duration_since(st).as_nanos() as f32;
                }
            }
            do_tick_physics_next = can_tick
                && now
                    .saturating_duration_since(prev_tick_physics_time)
                    .as_nanos()
                    > 1_000_000_000 / u128::from(self.0.settings.tick_physics_speed); // intended is 60 ticks per second

            // render

            let now = Instant::now();
            let delta = now.saturating_duration_since(last_frame);
            last_frame = now;
            if let Some(w) = &mut self.0.world {
                w.frame(delta); // this delta is more accurate than the one based on counter_last_frame
            }

            self.0.fps_counter.frames += 1;
            if now
                .saturating_duration_since(self.0.fps_counter.last_update)
                .as_millis()
                >= 1000
            {
                self.0.fps_counter.display_value = self.0.fps_counter.frames;
                self.0.fps_counter.frames = 0;
                self.0.fps_counter.tick_display_value = self.0.fps_counter.ticks;
                self.0.fps_counter.ticks = 0;
                self.0.fps_counter.last_update = now;

                // let nums: Vec<f32> = self.0.fps_counter.frame_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                // let avg_mspf: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                // let nums: Vec<f32> = self.0.fps_counter.tick_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                // let avg_mspt: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                // let nums: Vec<f32> = self.0.fps_counter.tick_physics_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                // let avg_mspt_phys: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                // println!("FPS: {}, TPS: {}, mspf: {:.2}, mspt: {:.2}, mspt_phys: {:.2}", self.0.fps_counter.display_value, ticks, avg_mspf, avg_mspt, avg_mspt_phys);
            }

            let time_nano = Instant::now()
                .saturating_duration_since(counter_last_frame)
                .as_nanos();
            self.0.fps_counter.frame_times.rotate_left(1);
            self.0.fps_counter.frame_times[self.0.fps_counter.frame_times.len() - 1] =
                time_nano as f32;

            profiling::finish_frame!();
            // sleep
            if !do_tick_next {
                profiling::scope!("sleep");
                // let now = Instant::now();

                // TODO: this sleep is sleeping for like 15ms at a time on my system; figure out what the correct way to handle loop timing is
                ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode

                // println!("slept {}ms", Instant::now().saturating_duration_since(now).as_millis());
            }
            counter_last_frame = Instant::now();
        }

        info!(target: "", "Shutting down...");
        let term_size = term.size().unwrap();
        term.backend_mut()
            .set_cursor(2 + input.len() as u16, term_size.height - 2)
            .unwrap();
        term.draw(|f| self.draw_terminal(f, &input, &mut tui_widget_state))
            .unwrap();

        std::thread::sleep(Duration::from_millis(500));

        term.clear().unwrap();
        term.set_cursor(0, 0).unwrap();

        Ok(())
    }

    #[profiling::function]
    fn tick(&mut self) {
        self.0.tick_time += 1;

        if let Some(w) = &mut self.0.world {
            w.tick(
                self.0.tick_time,
                &self.0.settings,
                self.0.registries.clone(),
                &self.0.file_helper,
            );
        }
    }

    fn draw_terminal<TB: Backend>(
        &mut self,
        frame: &mut Frame<TB>,
        input: &str,
        tui_widget_state: &mut TuiWidgetState,
    ) {
        let main_chunks = Layout::default()
            .constraints([Constraint::Min(0), Constraint::Length(20)].as_ref())
            .direction(tui::layout::Direction::Horizontal)
            .split(frame.size());

        // main left

        // main left - upper
        let main_left_chunks = Layout::default()
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(main_chunks[0]);

        frame.set_cursor(
            main_left_chunks[1].x + 2 + input.len() as u16,
            main_left_chunks[1].y + 1,
        );

        // let warning_style = Style::default().fg(tui::style::Color::Yellow);
        // let logs: Vec<ListItem> = (0..40).into_iter().map(|i| ListItem::new(
        //     vec![Spans::from(vec![
        //         Span::styled(format!("{:<9}", "thing"), warning_style),
        //         Span::raw(format!("abc {}", i)),
        //     ])]
        // )).collect();
        // let logs = List::new(logs).block(Block::default().borders(Borders::ALL).title("List"));
        // frame.render_widget(logs, main_left_chunks[0]);

        let tui_sm = TuiLoggerSmartWidget::default()
            .style_error(Style::default().fg(tui::style::Color::LightRed))
            .style_debug(Style::default().fg(tui::style::Color::Green))
            .style_warn(Style::default().fg(tui::style::Color::Yellow))
            .style_trace(Style::default().fg(tui::style::Color::Magenta))
            .style_info(Style::default().fg(tui::style::Color::White))
            .title_log("Log")
            .state(tui_widget_state);
        frame.render_widget(tui_sm, main_left_chunks[0]);

        // main left - lower

        let text = vec![Spans::from(vec![Span::raw(">"), Span::raw(input)])];
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Input", Style::default()));
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, main_left_chunks[1]);

        // main right

        let nums: Vec<&f32> = self
            .0
            .fps_counter
            .frame_times
            .iter()
            .filter(|n| **n != 0.0)
            .collect();
        let avg_ms_frame: f32 =
            nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

        let nums: Vec<&f32> = self
            .0
            .fps_counter
            .tick_times
            .iter()
            .filter(|n| **n != 0.0)
            .collect();
        let avg_ms_tick: f32 =
            nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

        let nums: Vec<&f32> = self
            .0
            .fps_counter
            .tick_physics_times
            .iter()
            .filter(|n| **n != 0.0)
            .collect();
        let avg_mspt_physics: f32 =
            nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

        let frames_style = Style::default();

        let ticks_style = match self.0.fps_counter.tick_display_value {
            0..=20 => Style::default().fg(tui::style::Color::LightRed),
            21..=27 => Style::default().fg(tui::style::Color::Yellow),
            _ => Style::default().fg(tui::style::Color::LightGreen),
        };

        let mspt_style = if avg_ms_tick < 37.04 {
            // 27 tps
            Style::default().fg(tui::style::Color::LightGreen)
        } else if avg_ms_tick < 47.62 {
            // 21 tps
            Style::default().fg(tui::style::Color::Yellow)
        } else {
            Style::default().fg(tui::style::Color::LightRed)
        };

        let mspt_physics_style = if avg_mspt_physics < 18.18 {
            // 55 tps
            Style::default().fg(tui::style::Color::LightGreen)
        } else if avg_mspt_physics < 20.0 {
            // 50 tps
            Style::default().fg(tui::style::Color::Yellow)
        } else {
            Style::default().fg(tui::style::Color::LightRed)
        };

        let text = vec![
            Spans::from(vec![
                Span::raw("FPS: "),
                Span::styled(
                    format!("{}", self.0.fps_counter.display_value),
                    frames_style,
                ),
            ]),
            Spans::from(vec![
                Span::raw("TPS: "),
                Span::styled(
                    format!("{}", self.0.fps_counter.tick_display_value),
                    ticks_style,
                ),
            ]),
            Spans::from(format!("mspf: {avg_ms_frame:.2}")),
            Spans::from(vec![
                Span::raw("mspt: "),
                Span::styled(format!("{avg_ms_tick:.2}"), mspt_style),
            ]),
            Spans::from(vec![
                Span::raw("avg_mspt_physics: "),
                Span::styled(format!("{avg_mspt_physics:.2}"), mspt_physics_style),
            ]),
        ];
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Stats", Style::default()));
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, main_chunks[1]);
    }
}
