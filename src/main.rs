#![deny(clippy::all)]
#![deny(clippy::cargo)]
#![warn(clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::expect_fun_call)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::let_underscore_drop)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_inception)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::similar_names)]

mod game;

use std::fs::File;
use std::str::FromStr;
use std::thread;

use backtrace::Backtrace;
use clap::crate_authors;
use clap::crate_name;
use clap::crate_version;
use clap::App;
use clap::Arg;
use game::Game;
use liquidfun::box2d::collision::shapes::polygon_shape::PolygonShape;
use liquidfun::box2d::dynamics::body::BodyDef;
use liquidfun::box2d::dynamics::body::BodyType;
use liquidfun::box2d::dynamics::fixture::FixtureDef;
use log::error;
use log::info;
use log::warn;
use log::LevelFilter;
use simplelog::CombinedLogger;
use simplelog::ConfigBuilder;
use simplelog::TermLogger;
use simplelog::TerminalMode;
use simplelog::WriteLogger;
use specs::Builder;
use specs::WorldExt;
use tui::backend::Backend;
use tui::backend::CrosstermBackend;
use tui::Terminal;

use crate::game::client::render::Fonts;
use crate::game::client::render::Renderer;
use crate::game::client::world::ClientChunk;
use crate::game::client::world::ClientWorld;
use crate::game::client::Client;
use crate::game::common::world::entity::GameEntity;
use crate::game::common::world::entity::Hitbox;
use crate::game::common::world::entity::Persistent;
use crate::game::common::world::entity::PhysicsEntity;
use crate::game::common::world::entity::Player;
use crate::game::common::world::entity::PlayerMovementMode;
use crate::game::common::world::AutoTarget;
use crate::game::common::world::B2BodyComponent;
use crate::game::common::world::Camera;
use crate::game::common::world::CollisionFlags;
use crate::game::common::world::Loader;
use crate::game::common::world::Position;
use crate::game::common::world::Target;
use crate::game::common::world::TargetStyle;
use crate::game::common::world::Velocity;
use crate::game::common::world::LIQUIDFUN_SCALE;
use crate::game::common::FileHelper;
use crate::game::server::world::ServerChunk;

#[allow(clippy::needless_pass_by_value)]
fn is_type<T: FromStr>(val: &str) -> Result<(), String>
where
    <T as std::str::FromStr>::Err: std::string::ToString,
{
    match val.parse::<T>() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[profiling::function]
fn main() -> Result<(), String> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debugging features"),
        )
        .arg(
            Arg::new("no-tick")
                .long("no-tick")
                .help("Turn off simulation by default"),
        )
        .arg(
            Arg::new("connect")
                .short('c')
                .long("connect")
                .takes_value(true)
                .value_name("IP:PORT")
                .help("Connect to a server automatically"),
        )
        .arg(
            Arg::new("game-dir")
                .long("game-dir")
                .takes_value(true)
                .value_name("PATH")
                .default_value("./gamedir/")
                .help("Set the game directory"),
        )
        .arg(
            Arg::new("assets-dir")
                .long("assets-dir")
                .takes_value(true)
                .value_name("PATH")
                .default_value("./gamedir/assets/")
                .help("Set the assets directory"),
        )
        .subcommand(
            App::new("server").about("Run dedicated server").arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .takes_value(true)
                    .value_name("PORT")
                    .default_value("6673")
                    .validator(is_type::<u16>)
                    .help("The port to run the server on"),
            ),
        )
        .get_matches();

    let file_helper = FileHelper::new(
        matches.value_of("game-dir").unwrap().into(),
        matches.value_of("assets-dir").unwrap().into(),
    );

    if !file_helper.game_path("").exists() {
        info!("game dir missing, creating it...");
        std::fs::create_dir_all(file_helper.game_path("")).expect("Failed to create game dir:");
    }

    if !file_helper.asset_path("").exists() {
        info!("asset dir missing, creating it...");
        std::fs::create_dir_all(file_helper.asset_path("")).expect("Failed to create asset dir:");
    }

    let server = matches.subcommand_matches("server").is_some();
    let client = !server;

    if server {
        crossterm::terminal::enable_raw_mode().unwrap();

        let stdout = std::io::stdout();
        let backend = CrosstermBackend::new(stdout);

        let mut terminal = Terminal::new(backend).unwrap();

        std::panic::set_hook(Box::new(|info| {
            let stdout = std::io::stdout();
            let mut backend = CrosstermBackend::new(stdout);
            backend.clear().unwrap();
            backend.set_cursor(0, 0).unwrap();

            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");
            let bt = Backtrace::new();

            let mut c: tui::buffer::Cell = tui::buffer::Cell::default();
            c.set_symbol(
                format!(
                    "thread '{}' {}\nSee latest.log for more details. Backtrace:\n{:?}\n",
                    name, info, bt
                )
                .as_str(),
            );
            c.set_fg(tui::style::Color::LightRed);
            let t: (u16, u16, _) = (0, 0, &c);

            backend.draw(std::iter::once(t)).unwrap();
            backend.flush().unwrap();

            error!("thread '{}' {}", name, info);
            error!(
                "See server_latest.log for more details. Backtrace:\n{:?}",
                bt
            );
        }));

        let res = std::panic::catch_unwind(move || {
            println!("Starting server...");
            let mut game: Game<ServerChunk> = Game::new(file_helper);

            if let Some(w) = &mut game.world {
                let body_def = BodyDef {
                    body_type: BodyType::DynamicBody,
                    fixed_rotation: true,
                    gravity_scale: 0.0,
                    bullet: true,
                    ..BodyDef::default()
                };
                let body = w.lqf_world.create_body(&body_def);
                let mut dynamic_box = PolygonShape::new();
                dynamic_box.set_as_box(12.0 / LIQUIDFUN_SCALE / 2.0, 20.0 / LIQUIDFUN_SCALE / 2.0);
                let mut fixture_def = FixtureDef::new(&dynamic_box);
                fixture_def.density = 1.5;
                fixture_def.friction = 0.3;
                fixture_def.filter.category_bits = CollisionFlags::PLAYER.bits();
                fixture_def.filter.mask_bits =
                    (CollisionFlags::RIGIDBODY | CollisionFlags::ENTITY).bits();
                body.create_fixture(&fixture_def);

                let _player = w
                    .ecs
                    .create_entity()
                    .with(Player { movement: PlayerMovementMode::Free })
                    .with(GameEntity)
                    .with(PhysicsEntity {
                        on_ground: false,
                        gravity: 0.2,
                        edge_clip_distance: 2.0,
                        collision: true,
                        collide_with_sand: true,
                    })
                    .with(Persistent)
                    .with(Position { x: 0.0, y: -20.0 })
                    .with(Velocity { x: 0.0, y: 0.0 })
                    .with(Hitbox { x1: -6.0, y1: -10.0, x2: 6.0, y2: 10.0 })
                    .with(Loader)
                    .with(B2BodyComponent::of(body))
                    .build();
            };

            println!("Starting main loop...");
            match game.run(&matches, &mut terminal) {
                Ok(_) => {}
                Err(e) => panic!("Server encountered a fatal error: {}", e),
            }
        });

        if res.is_err() {
            println!("Server crashed, exiting...");
        } else {
            println!("Server shut down successfully.");
        }
    } else if client {
        let debug = matches.is_present("debug");

        if !file_helper.game_path("logs/").exists() {
            info!("logs dir missing, creating it...");
            std::fs::create_dir_all(file_helper.game_path("logs/"))
                .expect("Failed to create logs dir:");
        }

        CombinedLogger::init(vec![
            TermLogger::new(
                if debug {
                    LevelFilter::Trace
                } else {
                    LevelFilter::Info
                },
                ConfigBuilder::new()
                    .set_location_level(if debug {
                        LevelFilter::Error
                    } else {
                        LevelFilter::Off
                    })
                    .set_level_padding(simplelog::LevelPadding::Right)
                    .set_target_level(LevelFilter::Off)
                    .set_time_to_local(true)
                    .build(),
                TerminalMode::Mixed,
                simplelog::ColorChoice::Auto,
            ),
            WriteLogger::new(
                LevelFilter::Trace,
                ConfigBuilder::new()
                    .set_location_level(LevelFilter::Error)
                    .set_level_padding(simplelog::LevelPadding::Right)
                    .set_target_level(LevelFilter::Off)
                    .set_time_to_local(true)
                    .build(),
                File::create(file_helper.game_path("logs/client_latest.log")).unwrap(),
            ),
        ])
        .unwrap();

        std::panic::set_hook(Box::new(|info| {
            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");

            error!("thread '{}' {}", name, info);
            error!(
                "See client_latest.log for more details. Backtrace:\n{:?}",
                Backtrace::new()
            );
        }));

        info!("Starting client...");

        // TODO: come up with a better way to handle this sdl's lifetime
        let sdl = Renderer::init_sdl().unwrap();
        info!("Starting init...");

        let mut r = Renderer::create(&sdl, &file_helper).expect("Renderer::create failed"); // want to panic

        let pixel_operator2 = sdl
            .sdl_ttf
            .load_font(
                file_helper.asset_path("font/pixel_operator/PixelOperator.ttf"),
                16,
            )
            .unwrap();
        let f = Some(Fonts { pixel_operator: pixel_operator2 });
        r.fonts = f;

        info!("Finished init.");

        let mut game: Game<ClientChunk> = Game::new(file_helper);

        if let Some(w) = &mut game.world {
            game.client = Some(Client::new());

            let body_def = BodyDef {
                body_type: BodyType::DynamicBody,
                fixed_rotation: true,
                gravity_scale: 0.0,
                bullet: true,
                ..BodyDef::default()
            };
            let body = w.lqf_world.create_body(&body_def);
            let mut dynamic_box = PolygonShape::new();
            dynamic_box.set_as_box(12.0 / LIQUIDFUN_SCALE / 2.0, 20.0 / LIQUIDFUN_SCALE / 2.0);
            let mut fixture_def = FixtureDef::new(&dynamic_box);
            fixture_def.density = 1.5;
            fixture_def.friction = 0.3;
            fixture_def.filter.category_bits = CollisionFlags::PLAYER.bits();
            fixture_def.filter.mask_bits =
                (CollisionFlags::RIGIDBODY | CollisionFlags::ENTITY).bits();
            body.create_fixture(&fixture_def);

            let player = w
                .ecs
                .create_entity()
                .with(Player { movement: PlayerMovementMode::Free })
                .with(GameEntity)
                .with(PhysicsEntity {
                    on_ground: false,
                    gravity: 0.5,
                    edge_clip_distance: 2.0,
                    collision: true,
                    collide_with_sand: true,
                })
                .with(Persistent)
                .with(Position { x: 0.0, y: -20.0 })
                .with(Velocity { x: 0.0, y: 0.0 })
                .with(Hitbox { x1: -6.0, y1: -10.0, x2: 6.0, y2: 10.0 })
                .with(Loader)
                .with(B2BodyComponent::of(body))
                .build();

            let _camera = w
                .ecs
                .create_entity()
                .with(Camera)
                .with(Position { x: 0.0, y: 0.0 })
                .with(Velocity { x: 0.0, y: 0.0 })
                .with(AutoTarget {
                    target: Target::Entity(player),
                    offset: (0.0, 0.0),
                    style: TargetStyle::Locked,
                })
                .build();

            game.client.as_mut().unwrap().world = Some(ClientWorld { local_entity: Some(player) });
        };

        info!("Starting main loop...");
        game.run(&sdl, Some(&mut r), &matches);
        info!("Goodbye!");
    }

    Ok(())
}
