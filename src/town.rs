use std::f32::consts::{FRAC_PI_2, PI};

use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::ecs::system::{Commands, Res};
use bevy::light::{DirectionalLight, GlobalAmbientLight};
use bevy::math::{Quat, Vec3};
use bevy::scene::SceneRoot;
use bevy::transform::components::Transform;

use crate::collision::Collider;
use crate::vehicle::{Vehicle, VehicleState};

const ASSET_BASE: &str = "kenney_retro-urban-kit/Models/GLB format/";

// Grid cell types:
// 0 = grass
// 1 = road N-S
// 2 = road E-W
// 3 = road intersection
// 4 = building zone (grass tile underneath, building assembled on 2x2 blocks)
// 5 = parking (pavement + vehicle spawn)
// 6 = park (grass + trees)
const GRID: [[u8; 8]; 8] = [
    [4, 4, 0, 1, 0, 0, 6, 6],
    [4, 4, 0, 1, 0, 0, 6, 6],
    [0, 0, 0, 1, 0, 0, 0, 0],
    [2, 2, 2, 3, 2, 2, 2, 2],
    [0, 0, 0, 1, 0, 0, 0, 0],
    [4, 4, 0, 1, 0, 4, 4, 0],
    [4, 4, 0, 1, 5, 4, 4, 0],
    [0, 0, 0, 1, 0, 0, 0, 0],
];

fn load_scene(asset_server: &AssetServer, name: &str) -> SceneRoot {
    let path = format!("{ASSET_BASE}{name}#Scene0");
    SceneRoot(asset_server.load(path))
}

pub fn system_spawn_town(mut commands: Commands, asset_server: Res<AssetServer>) {
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

    let mut building_spawned = [[false; 8]; 8];

    for row in 0..8_usize {
        for col in 0..8_usize {
            let cell = GRID[row][col];
            let x = col as f32;
            let z = row as f32;

            match cell {
                0 => {
                    spawn_tile(&mut commands, &asset_server, "grass.glb", x, z, 0.0);
                }
                1 => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-straight.glb",
                        x,
                        z,
                        0.0,
                    );
                }
                2 => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-straight.glb",
                        x,
                        z,
                        FRAC_PI_2,
                    );
                }
                3 => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-center.glb",
                        x,
                        z,
                        0.0,
                    );
                }
                4 => {
                    spawn_tile(&mut commands, &asset_server, "grass.glb", x, z, 0.0);

                    if !building_spawned[row][col]
                        && col + 1 < 8
                        && row + 1 < 8
                        && GRID[row][col + 1] == 4
                        && GRID[row + 1][col] == 4
                        && GRID[row + 1][col + 1] == 4
                    {
                        building_spawned[row][col] = true;
                        building_spawned[row][col + 1] = true;
                        building_spawned[row + 1][col] = true;
                        building_spawned[row + 1][col + 1] = true;

                        let variant = ((row + col) % 2) as u8;
                        spawn_building(&mut commands, &asset_server, x, z, variant);
                    }
                }
                5 => {
                    spawn_tile(
                        &mut commands,
                        &asset_server,
                        "road-asphalt-pavement.glb",
                        x,
                        z,
                        0.0,
                    );
                    spawn_vehicle(&mut commands, &asset_server, x, z, 0.0);
                    spawn_vehicle(&mut commands, &asset_server, x, z + 0.4, PI);
                }
                6 => {
                    spawn_tile(&mut commands, &asset_server, "grass.glb", x, z, 0.0);
                    // Scatter trees in park cells
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
                _ => {}
            }
        }
    }

    spawn_props(&mut commands, &asset_server);
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

    // Wall positions: (x, z, rotation, model_index)
    // 0 = plain wall, 1 = door, 2 = window
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

fn spawn_props(commands: &mut Commands, asset_server: &AssetServer) {
    for row in [0.0_f32, 2.0, 4.5, 7.0] {
        commands.spawn((
            load_scene(asset_server, "detail-light-single.glb"),
            Transform::from_xyz(3.6, 0.0, row),
        ));
    }

    for col in [0.5_f32, 2.0, 5.0, 7.0] {
        commands.spawn((
            load_scene(asset_server, "detail-light-single.glb"),
            Transform::from_xyz(col, 0.0, 3.6),
        ));
    }

    commands.spawn((
        load_scene(asset_server, "detail-bench.glb"),
        Transform::from_xyz(5.5, 0.0, 1.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
    ));
    commands.spawn((
        load_scene(asset_server, "detail-bench.glb"),
        Transform::from_xyz(5.5, 0.0, 2.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
    ));
}
