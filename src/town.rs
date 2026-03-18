use std::f32::consts::{FRAC_PI_2, PI};
use std::fs;

use bevy::app::{Plugin, Startup};
use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Res};
use bevy::light::{DirectionalLight, GlobalAmbientLight};
use bevy::math::{Quat, Vec3};
use bevy::scene::SceneRoot;
use bevy::transform::components::Transform;
use serde::Deserialize;

use crate::collision::Collider;
use crate::vehicle::{Vehicle, VehicleState};

const ASSET_BASE: &str = "kenney_retro-urban-kit/Models/GLB format/";
const TOWN_CONFIG_PATH: &str = "assets/config/town.json";

pub struct TownPlugin;

impl Plugin for TownPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            Startup,
            (system_load_town_config, system_spawn_town_from_config).chain(),
        );
    }
}

#[derive(Resource, Deserialize, Clone)]
pub struct TownConfig {
    pub grid: Vec<Vec<CellType>>,
    pub props: Vec<PropSpawn>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Grass,
    RoadNs,
    RoadEw,
    RoadIntersection,
    BuildingZone,
    Parking,
    Park,
}

#[derive(Deserialize, Clone)]
pub struct PropSpawn {
    pub model: String,
    pub position: [f32; 3],
    pub yaw: f32,
}

fn load_scene(asset_server: &AssetServer, name: &str) -> SceneRoot {
    let path = format!("{ASSET_BASE}{name}#Scene0");
    SceneRoot(asset_server.load(path))
}

pub fn system_load_town_config(mut commands: Commands) {
    let data = fs::read_to_string(TOWN_CONFIG_PATH)
        .unwrap_or_else(|error| panic!("Failed to read {TOWN_CONFIG_PATH}: {error}"));
    let config: TownConfig = serde_json::from_str(&data)
        .unwrap_or_else(|error| panic!("Failed to parse {TOWN_CONFIG_PATH} as JSON: {error}"));

    commands.insert_resource(config);
}

pub fn system_spawn_town_from_config(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<TownConfig>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_rotation(Quat::from_euler(bevy::math::EulerRot::XYZ, -0.8, 0.2, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        affects_lightmapped_meshes: true,
    });

    let rows = config.grid.len();
    let max_cols = config
        .grid
        .iter()
        .map(std::vec::Vec::len)
        .max()
        .unwrap_or(0);
    let mut building_spawned = vec![vec![false; max_cols]; rows];

    for row in 0..rows {
        for col in 0..config.grid[row].len() {
            let cell = config.grid[row][col];
            let x = col as f32;
            let z = row as f32;

            match cell {
                CellType::Grass => {
                    spawn_tile(&mut commands, &asset_server, "grass.glb", x, z, 0.0);
                }
                CellType::RoadNs => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-straight.glb",
                        x,
                        z,
                        0.0,
                    );
                }
                CellType::RoadEw => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-straight.glb",
                        x,
                        z,
                        FRAC_PI_2,
                    );
                }
                CellType::RoadIntersection => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-center.glb",
                        x,
                        z,
                        0.0,
                    );
                }
                CellType::BuildingZone => {
                    spawn_tile(&mut commands, &asset_server, "grass.glb", x, z, 0.0);

                    if !building_spawned[row][col]
                        && col + 1 < config.grid[row].len()
                        && row + 1 < rows
                        && col + 1 < config.grid[row + 1].len()
                        && config.grid[row][col + 1] == CellType::BuildingZone
                        && config.grid[row + 1][col] == CellType::BuildingZone
                        && config.grid[row + 1][col + 1] == CellType::BuildingZone
                    {
                        building_spawned[row][col] = true;
                        building_spawned[row][col + 1] = true;
                        building_spawned[row + 1][col] = true;
                        building_spawned[row + 1][col + 1] = true;

                        let variant = ((row + col) % 2) as u8;
                        spawn_building(&mut commands, &asset_server, x, z, variant);
                    }
                }
                CellType::Parking => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-pavement.glb",
                        x,
                        z,
                        0.0,
                    );
                    spawn_vehicle(&mut commands, &asset_server, x, z, 0.0);
                }
                CellType::Park => {
                    spawn_tile(&mut commands, &asset_server, "grass.glb", x, z, 0.0);
                    let tree_model = match (row + col) % 3 {
                        0 => "tree-large.glb",
                        1 => "tree-park-large.glb",
                        _ => "tree-pine-large.glb",
                    };
                    commands.spawn((
                        load_scene(&asset_server, tree_model),
                        Transform::from_xyz(x, 0.0, z),
                    ));
                }
            }
        }
    }

    for prop in &config.props {
        commands.spawn((
            load_scene(&asset_server, &prop.model),
            Transform::from_xyz(prop.position[0], prop.position[1], prop.position[2])
                .with_rotation(Quat::from_rotation_y(prop.yaw)),
        ));
    }
}

fn spawn_tile(
    commands: &mut Commands,
    asset_server: &AssetServer,
    model: &str,
    x: f32,
    z: f32,
    angle_y: f32,
) {
    let mut transform = Transform::from_xyz(x, 0.0, z);
    if angle_y.abs() > 0.001 {
        transform.rotation = Quat::from_rotation_y(angle_y);
    }
    commands.spawn((load_scene(asset_server, model), transform));
}

fn spawn_building(
    commands: &mut Commands,
    asset_server: &AssetServer,
    col: f32,
    row: f32,
    variant: u8,
) {
    let (wall, wall_door, wall_window, roof) = if variant == 0 {
        (
            "wall-a.glb",
            "wall-a-door.glb",
            "wall-a-window.glb",
            "wall-a-roof.glb",
        )
    } else {
        (
            "wall-b.glb",
            "wall-b-door.glb",
            "wall-b-window.glb",
            "wall-b-roof.glb",
        )
    };

    let walls: [(f32, f32, f32, u8); 8] = [
        (col + 0.5, row, 0.0, 0),
        (col + 1.5, row, 0.0, 2),
        (col + 0.5, row + 2.0, PI, 1),
        (col + 1.5, row + 2.0, PI, 2),
        (col, row + 0.5, FRAC_PI_2, 0),
        (col, row + 1.5, FRAC_PI_2, 0),
        (col + 2.0, row + 0.5, -FRAC_PI_2, 2),
        (col + 2.0, row + 1.5, -FRAC_PI_2, 0),
    ];

    for (x, z, angle, model_type) in &walls {
        let model = match model_type {
            1 => wall_door,
            2 => wall_window,
            _ => wall,
        };
        commands.spawn((
            load_scene(asset_server, model),
            Transform::from_xyz(*x, 0.0, *z).with_rotation(Quat::from_rotation_y(*angle)),
        ));
    }

    for dx in 0..2_u32 {
        for dz in 0..2_u32 {
            commands.spawn((
                load_scene(asset_server, roof),
                Transform::from_xyz(col + dx as f32 + 0.5, 1.0, row + dz as f32 + 0.5),
            ));
        }
    }

    commands.spawn((
        Collider {
            half_extents: Vec3::new(1.0, 1.5, 1.0),
        },
        Transform::from_xyz(col + 1.0, 1.5, row + 1.0),
    ));
}

fn spawn_vehicle(commands: &mut Commands, asset_server: &AssetServer, x: f32, z: f32, angle: f32) {
    commands.spawn((
        load_scene(asset_server, "truck-green.glb"),
        Transform::from_xyz(x, 0.0, z).with_rotation(Quat::from_rotation_y(angle)),
        Vehicle {
            state: VehicleState::Parked,
            speed: 0.0,
            yaw: angle,
        },
    ));
}
