use std::f32::consts::PI;

use bevy::asset::AssetServer;
use bevy::camera::Camera3d;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::MessageReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseMotion};
use bevy::math::{Quat, Vec2, Vec3};
use bevy::scene::SceneRoot;
use bevy::time::Time;
use bevy::transform::components::Transform;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::collision::{Collider, resolve_movement};
use crate::vehicle::{Vehicle, VehicleState};

const MOUSE_SENSITIVITY: f32 = 0.003;
const MOVE_SPEED: f32 = 5.0;
const PLAYER_HEIGHT: f32 = 1.7;
const PITCH_LIMIT: f32 = 1.4;
const INTERACT_RANGE: f32 = 2.5;
const PLAYER_HALF_EXTENTS: Vec3 = Vec3::new(0.2, 0.85, 0.2);
const FPS_WEAPON_SCENE: &str = "kenney_blaster-kit_2.1/Models/GLB format/blaster-a.glb#Scene0";

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Resource)]
pub struct PlayerState {
    pub mode: PlayerMode,
    pub vehicle_entity: Option<Entity>,
}

#[derive(PartialEq, Eq)]
pub enum PlayerMode {
    OnFoot,
    InVehicle,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            mode: PlayerMode::OnFoot,
            vehicle_entity: None,
        }
    }
}

#[derive(Resource, Default)]
pub struct LookAngles {
    pub yaw: f32,
    pub pitch: f32,
}

pub fn is_on_foot(state: Res<PlayerState>) -> bool {
    state.mode == PlayerMode::OnFoot
}

pub fn is_in_vehicle(state: Res<PlayerState>) -> bool {
    state.mode == PlayerMode::InVehicle
}

pub fn system_spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let weapon_scene = SceneRoot(asset_server.load(FPS_WEAPON_SCENE));

    commands
        .spawn((Player, Transform::from_xyz(8.0, 0.0, 8.0)))
        .with_children(|parent| {
            parent
                .spawn((
                    PlayerCamera,
                    Camera3d::default(),
                    Transform::from_xyz(0.0, PLAYER_HEIGHT, 0.0),
                ))
                .with_children(|camera| {
                    // First-person weapon viewmodel.
                    camera.spawn((
                        weapon_scene.clone(),
                        Transform::from_xyz(0.18, -0.18, -0.45)
                            .with_rotation(Quat::from_rotation_y(PI))
                            .with_scale(Vec3::splat(0.35)),
                    ));
                });
        });
}

pub fn system_cursor_grab(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(mut options) = cursor_options.single_mut() else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        options.grab_mode = CursorGrabMode::Locked;
        options.visible = false;
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        options.grab_mode = CursorGrabMode::None;
        options.visible = true;
    }
}

pub fn system_mouse_look(
    state: Res<PlayerState>,
    mut angles: ResMut<LookAngles>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    angles.pitch = (angles.pitch - delta.y * MOUSE_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);

    if let Ok(mut cam_transform) = camera_query.single_mut() {
        cam_transform.rotation = Quat::from_rotation_x(angles.pitch);
    }

    if state.mode == PlayerMode::OnFoot {
        angles.yaw -= delta.x * MOUSE_SENSITIVITY;
        if let Ok(mut player_transform) = player_query.single_mut() {
            player_transform.rotation = Quat::from_rotation_y(angles.yaw);
        }
    }
}

pub fn system_player_move(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    angles: Res<LookAngles>,
    mut player_query: Query<&mut Transform, With<Player>>,
    collider_query: Query<(&Transform, &Collider), Without<Player>>,
) {
    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };

    let forward = Vec3::new(-angles.yaw.sin(), 0.0, -angles.yaw.cos());
    let right = Vec3::new(forward.z, 0.0, -forward.x);

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction += right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction -= right
    }

    if direction == Vec3::ZERO {
        return;
    }

    let direction = direction.normalize_or_zero();
    let displacement = direction * MOVE_SPEED * time.delta_secs();

    // Collect colliders (bounded to 128)
    let mut colliders_vec: Vec<(Vec3, Vec3)> = Vec::with_capacity(128);
    for (col_transform, collider) in &collider_query {
        if colliders_vec.len() >= 128 {
            break;
        }
        colliders_vec.push((col_transform.translation, collider.half_extents));
    }

    let new_pos = resolve_movement(
        player_transform.translation,
        displacement,
        PLAYER_HALF_EXTENTS,
        &colliders_vec,
    );

    player_transform.translation = Vec3::new(new_pos.x, 0.0, new_pos.z);
}

pub fn system_player_interact(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<PlayerState>,
    mut angles: ResMut<LookAngles>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    mut vehicle_query: Query<(&mut Transform, &mut Vehicle, Entity), Without<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };

    match state.mode {
        PlayerMode::OnFoot => {
            let mut nearest: Option<(Entity, f32)> = None;
            for (vehicle_transform, vehicle, entity) in &vehicle_query {
                if vehicle.state != VehicleState::Parked {
                    continue;
                }
                let distance = player_transform
                    .translation
                    .distance(vehicle_transform.translation);
                if distance < INTERACT_RANGE && nearest.is_none_or(|(_, d)| distance < d) {
                    nearest = Some((entity, distance));
                }
            }

            if let Some((vehicle_entity, _)) = nearest
                && let Ok((vehicle_transform, mut vehicle, _)) =
                    vehicle_query.get_mut(vehicle_entity)
            {
                vehicle.state = VehicleState::Occupied;
                state.mode = PlayerMode::InVehicle;
                state.vehicle_entity = Some(vehicle_entity);

                player_transform.translation = vehicle_transform.translation;
                player_transform.rotation = Quat::from_rotation_y(vehicle.yaw);
                angles.yaw = vehicle.yaw;
            }
        }
        PlayerMode::InVehicle => {
            if let Some(vehicle_entity) = state.vehicle_entity
                && let Ok((vehicle_transform, mut vehicle, _)) =
                    vehicle_query.get_mut(vehicle_entity)
            {
                vehicle.state = VehicleState::Parked;
                vehicle.speed = 0.0;

                // Place player to the left of the vehicle
                let left = Vec3::new(-vehicle.yaw.cos(), 0.0, vehicle.yaw.sin());
                player_transform.translation = vehicle_transform.translation + left * 2.0;
                player_transform.translation.y = 0.0;
            }
            state.mode = PlayerMode::OnFoot;
            state.vehicle_entity = None;
        }
    }
}
