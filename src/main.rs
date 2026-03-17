use bevy::DefaultPlugins;
use bevy::app::{App, Startup};
use bevy::camera::Camera2d;
use bevy::color::Color;
use bevy::color::Srgba;
use bevy::ecs::component::Component;
use bevy::ecs::system::Commands;
use bevy::math::Vec3;
use bevy::sprite::Text2d;
use bevy::text::{TextColor, TextFont};
use bevy::transform::components::Transform;
use bevy::utils::default;

#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text2d::new("@"),
        TextFont {
            font_size: 12.0,
            font: default(),
            ..default()
        },
        // TextColor(Color::WHITE),
        TextColor(Color::Srgba(Srgba {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        })),
        Transform::from_translation(Vec3::ZERO),
        Player,
    ));
}
