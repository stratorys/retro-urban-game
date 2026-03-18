mod collision;
mod error;
mod player;
mod town;
mod vehicle;

use bevy::DefaultPlugins;
use bevy::app::{App, Startup, Update};
use bevy::ecs::schedule::IntoScheduleConfigs;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(player::PlayerState::default())
        .insert_resource(player::LookAngles::default())
        .add_systems(
            Startup,
            (town::system_spawn_town, player::system_spawn_player),
        )
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
        .run();
}
