mod collision;
mod player;
mod town;
mod vehicle;
mod zombie;

use bevy::DefaultPlugins;
use bevy::app::PluginGroup;
use bevy::app::{App, Startup, Update};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::image::ImagePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
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
}
