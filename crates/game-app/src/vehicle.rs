use bevy::ecs::component::Component;
use bevy::ecs::query::{With, Without};
use bevy::ecs::system::{Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::{Quat, Vec3};
use bevy::time::Time;
use bevy::transform::components::Transform;

use crate::collision::{Collider, resolve_movement};
use crate::player::{LookAngles, Player, PlayerState};

const MAX_SPEED: f32 = 12.0;
const REVERSE_MAX_SPEED: f32 = 4.0;
const ACCELERATION: f32 = 6.0;
const BRAKE_FORCE: f32 = 8.0;
const FRICTION: f32 = 4.0;
const TURN_RATE: f32 = 2.0;
const VEHICLE_HALF_EXTENTS: Vec3 = Vec3::new(0.6, 0.5, 1.2);

#[derive(Component)]
pub struct Vehicle {
    pub state: VehicleState,
    pub speed: f32,
    pub yaw: f32,
}

#[derive(PartialEq, Eq)]
pub enum VehicleState {
    Parked,
    Occupied,
}

pub fn system_vehicle_drive(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut angles: ResMut<LookAngles>,
    state: Res<PlayerState>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>, Without<Collider>)>,
    mut vehicle_query: Query<(&mut Transform, &mut Vehicle), (Without<Player>, Without<Collider>)>,
    collider_query: Query<(&Transform, &Collider), (Without<Player>, Without<Vehicle>)>,
) {
    let Some(vehicle_entity) = state.vehicle_entity else {
        return;
    };
    let Ok((mut vehicle_transform, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) else {
        return;
    };
    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    if keyboard.pressed(KeyCode::KeyW) {
        vehicle.speed = (vehicle.speed + ACCELERATION * dt).min(MAX_SPEED);
    }
    if keyboard.pressed(KeyCode::KeyS) {
        vehicle.speed = (vehicle.speed - BRAKE_FORCE * dt).max(-REVERSE_MAX_SPEED);
    }

    if !keyboard.pressed(KeyCode::KeyW) && !keyboard.pressed(KeyCode::KeyS) {
        let friction_delta = FRICTION * dt;
        if vehicle.speed > 0.0 {
            vehicle.speed = (vehicle.speed - friction_delta).max(0.0);
        } else {
            vehicle.speed = (vehicle.speed + friction_delta).min(0.0);
        }
    }

    let speed_ratio = vehicle.speed.abs() / MAX_SPEED;
    let turn_delta = TURN_RATE * dt * speed_ratio;
    if keyboard.pressed(KeyCode::KeyA) {
        vehicle.yaw += turn_delta;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        vehicle.yaw -= turn_delta;
    }

    let forward = Vec3::new(-vehicle.yaw.sin(), 0.0, -vehicle.yaw.cos());
    let displacement = forward * vehicle.speed * dt;

    let mut colliders_vec: Vec<(Vec3, Vec3)> = Vec::with_capacity(128);
    for (col_transform, collider) in &collider_query {
        if colliders_vec.len() >= 128 {
            break;
        }
        colliders_vec.push((col_transform.translation, collider.half_extents));
    }

    let old_pos = vehicle_transform.translation;
    let new_pos = resolve_movement(old_pos, displacement, VEHICLE_HALF_EXTENTS, &colliders_vec);

    let expected = old_pos + displacement;
    if (new_pos.x - expected.x).abs() > 0.001 || (new_pos.z - expected.z).abs() > 0.001 {
        vehicle.speed = 0.0;
    }

    vehicle_transform.translation = Vec3::new(new_pos.x, 0.0, new_pos.z);
    vehicle_transform.rotation = Quat::from_rotation_y(vehicle.yaw);

    player_transform.translation = vehicle_transform.translation;
    player_transform.rotation = Quat::from_rotation_y(vehicle.yaw);
    angles.yaw = vehicle.yaw;
}
