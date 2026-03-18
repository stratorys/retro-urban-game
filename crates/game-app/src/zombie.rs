use bevy::app::{Plugin, Startup, Update};
use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::{With, Without};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::math::Vec3;
use bevy::scene::SceneRoot;
use bevy::time::Time;
use bevy::transform::components::Transform;

use crate::collision::{Collider, resolve_movement};
use crate::player::{Player, Projectile};
use crate::vehicle::Vehicle;

const ZOMBIE_HIT_RANGE: f32 = 0.45;
const ZOMBIE_HALF_EXTENTS: Vec3 = Vec3::new(0.25, 0.9, 0.25);
const PROJECTILE_HIT_RADIUS: f32 = 0.35;
const MALE_ZOMBIE_SPEED: f32 = 1.6;
const FEMALE_ZOMBIE_SPEED: f32 = 2.0;
const MALE_ZOMBIE_SCENE: &str =
    "kenney_mini-characters/Models/GLB format/character-male-a.glb#Scene0";
const FEMALE_ZOMBIE_SCENE: &str =
    "kenney_mini-characters/Models/GLB format/character-female-a.glb#Scene0";

pub struct ZombiePlugin;

impl Plugin for ZombiePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Startup, system_spawn_zombies)
            .add_systems(
                Update,
                (system_zombie_chase, system_projectile_hit_zombie).chain(),
            );
    }
}

#[derive(Component)]
pub struct Zombie {
    pub kind: ZombieKind,
    pub health: f32,
}

#[derive(Clone, Copy)]
pub enum ZombieKind {
    Male,
    Female,
}

pub fn system_spawn_zombies(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let male_scene = SceneRoot(asset_server.load(MALE_ZOMBIE_SCENE));
    let female_scene = SceneRoot(asset_server.load(FEMALE_ZOMBIE_SCENE));

    let spawn_points = [
        (Vec3::new(2.0, 0.0, 2.0), ZombieKind::Male),
        (Vec3::new(14.0, 0.0, 2.0), ZombieKind::Female),
        (Vec3::new(2.0, 0.0, 14.0), ZombieKind::Female),
        (Vec3::new(14.0, 0.0, 14.0), ZombieKind::Male),
        (Vec3::new(8.0, 0.0, 3.0), ZombieKind::Male),
        (Vec3::new(8.0, 0.0, 13.0), ZombieKind::Female),
    ];

    for (spawn, kind) in spawn_points {
        let (scene, health) = match kind {
            ZombieKind::Male => (male_scene.clone(), 50.0),
            ZombieKind::Female => (female_scene.clone(), 40.0),
        };
        commands.spawn((
            Zombie { kind, health },
            Collider {
                half_extents: ZOMBIE_HALF_EXTENTS,
            },
            scene,
            Transform::from_translation(spawn).with_scale(Vec3::splat(1.2)),
        ));
    }
}

pub fn system_zombie_chase(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Zombie>)>,
    mut zombie_query: Query<(&mut Transform, &Zombie), With<Zombie>>,
    collider_query: Query<(&Transform, &Collider), (Without<Zombie>, Without<Player>, Without<Vehicle>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let target = player_transform.translation;

    let mut colliders_vec: Vec<(Vec3, Vec3)> = Vec::with_capacity(256);
    for (col_transform, collider) in &collider_query {
        if colliders_vec.len() >= 256 {
            break;
        }
        colliders_vec.push((col_transform.translation, collider.half_extents));
    }

    for (mut zombie_transform, zombie) in &mut zombie_query {
        let to_player = target - zombie_transform.translation;
        let flat = Vec3::new(to_player.x, 0.0, to_player.z);
        let distance = flat.length();

        if distance <= ZOMBIE_HIT_RANGE {
            continue;
        }

        let direction = flat.normalize_or_zero();
        let speed = match zombie.kind {
            ZombieKind::Male => MALE_ZOMBIE_SPEED,
            ZombieKind::Female => FEMALE_ZOMBIE_SPEED,
        };
        let displacement = direction * speed * time.delta_secs();

        let new_pos = resolve_movement(
            zombie_transform.translation,
            displacement,
            ZOMBIE_HALF_EXTENTS,
            &colliders_vec,
        );

        zombie_transform.translation = Vec3::new(new_pos.x, 0.0, new_pos.z);

        if direction.length_squared() > 0.0001 {
            zombie_transform.look_to(direction, Vec3::Y);
        }
    }
}

pub fn system_projectile_hit_zombie(
    mut commands: Commands,
    mut zombie_query: Query<(Entity, &Transform, &mut Zombie)>,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
) {
    for (projectile_entity, projectile_transform) in &projectile_query {
        let mut hit_target: Option<Entity> = None;

        for (zombie_entity, zombie_transform, mut zombie) in &mut zombie_query {
            let distance = projectile_transform
                .translation
                .distance(zombie_transform.translation + Vec3::Y * 0.8);
            if distance <= PROJECTILE_HIT_RADIUS {
                zombie.health -= 25.0;
                if zombie.health <= 0.0 {
                    commands.entity(zombie_entity).despawn();
                }
                hit_target = Some(zombie_entity);
                break;
            }
        }

        if hit_target.is_some() {
            commands.entity(projectile_entity).despawn();
        }
    }
}
