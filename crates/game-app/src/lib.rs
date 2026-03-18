use std::path::Path;

use bevy::DefaultPlugins;
use bevy::app::PluginGroup;
use bevy::app::{App, Startup, Update};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::image::ImagePlugin;

pub mod collision;
pub mod player;
pub mod town;
pub mod vehicle;
pub mod zombie;

pub fn run(map_path: &Path) -> Result<(), map_schema::MapError> {
    let map = map_schema::load_map(map_path)?;

    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(town::LoadedTown(map))
        .add_plugins(town::TownPlugin)
        .add_plugins(zombie::ZombiePlugin)
        .insert_resource(player::PlayerState::default())
        .insert_resource(player::LookAngles::default())
        .add_systems(Startup, player::system_spawn_player)
        .add_systems(
            Update,
            (
                player::system_cursor_grab,
                player::system_mouse_look,
                player::system_player_interact,
            )
                .chain(),
        )
        .add_systems(
            Update,
            player::system_player_move
                .run_if(player::is_on_foot)
                .after(player::system_player_interact),
        )
        .add_systems(
            Update,
            vehicle::system_vehicle_drive
                .run_if(player::is_in_vehicle)
                .after(player::system_player_interact),
        )
        .add_systems(
            Update,
            player::system_player_shoot
                .run_if(player::is_on_foot)
                .after(player::system_mouse_look),
        )
        .add_systems(Update, player::system_projectile_move)
        .run();

    Ok(())
}
