use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use bevy::DefaultPlugins;
use bevy::app::{App, Plugin, PluginGroup, Startup, Update};
use bevy::asset::AssetServer;
use bevy::camera::Camera;
use bevy::camera::Camera3d;
use bevy::ecs::component::Component;
use bevy::ecs::query::With;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Local, Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseWheel};
use bevy::light::{DirectionalLight, GlobalAmbientLight};
use bevy::math::{Quat, Vec3};
use bevy::scene::SceneRoot;
use bevy::time::Time;
use bevy::transform::components::{GlobalTransform, Transform};
use bevy::window::{PresentMode, PrimaryWindow, Window, WindowPlugin};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use map_schema::{CellType, PROP_PALETTE, PropSpawn, TILE_PALETTE, TownMap, load_map, save_map};

const ASSET_BASE: &str = "kenney_retro-urban-kit/Models/GLB format/";

#[derive(Debug, Clone, Copy)]
pub struct NewMapArgs {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum EditorError {
    MissingMap {
        path: PathBuf,
    },
    NewMapAlreadyExists {
        path: PathBuf,
    },
    InvalidNewDimensions,
    Map(map_schema::MapError),
}

impl Display for EditorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMap { path } => write!(
                f,
                "Map file {} does not exist. Use --new --width <w> --height <h> to create one.",
                path.display()
            ),
            Self::NewMapAlreadyExists { path } => write!(
                f,
                "Map file {} already exists. Pass --overwrite with --new to replace in-memory map.",
                path.display()
            ),
            Self::InvalidNewDimensions => {
                write!(f, "--width and --height must both be > 0 when --new is used")
            }
            Self::Map(error) => write!(f, "{error}"),
        }
    }
}

impl Error for EditorError {}

impl From<map_schema::MapError> for EditorError {
    fn from(value: map_schema::MapError) -> Self {
        Self::Map(value)
    }
}

pub fn run(map_path: &Path, new_map: Option<NewMapArgs>, overwrite: bool) -> Result<(), EditorError> {
    let map = if map_path.exists() {
        if let Some(new_args) = new_map {
            if !overwrite {
                return Err(EditorError::NewMapAlreadyExists {
                    path: map_path.to_path_buf(),
                });
            }
            create_empty_map(new_args)?
        } else {
            load_map(map_path)?
        }
    } else if let Some(new_args) = new_map {
        create_empty_map(new_args)?
    } else {
        return Err(EditorError::MissingMap {
            path: map_path.to_path_buf(),
        });
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Retro Urban Map Editor".to_owned(),
                present_mode: PresentMode::AutoVsync,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(EguiPlugin::default())
        .insert_resource(EditorState::new(map, map_path.to_path_buf()))
        .add_plugins(MapEditorPlugin)
        .run();

    Ok(())
}

fn create_empty_map(new_args: NewMapArgs) -> Result<TownMap, EditorError> {
    if new_args.width == 0 || new_args.height == 0 {
        return Err(EditorError::InvalidNewDimensions);
    }

    let row = vec![CellType::Grass; new_args.width as usize];
    Ok(TownMap {
        grid: vec![row; new_args.height as usize],
        props: Vec::new(),
    })
}

pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, system_setup_editor)
            .add_systems(EguiPrimaryContextPass, system_editor_ui)
            .add_systems(
                Update,
                (
                    system_cursor_from_mouse,
                    system_cursor_move,
                    system_camera_view_controls,
                    system_apply_action,
                    system_rebuild_visuals,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
struct EditorVisual;

#[derive(Component)]
struct EditorCamera;

#[derive(Resource)]
struct EditorState {
    map: TownMap,
    map_path: PathBuf,
    cursor_col: usize,
    cursor_row: usize,
    selected_tile: usize,
    selected_prop: usize,
    mode: EditMode,
    prop_yaw: f32,
    prop_height: f32,
    last_placed_cell: Option<(usize, usize)>,
    dirty_visuals: bool,
    status: String,
}

impl EditorState {
    fn new(map: TownMap, map_path: PathBuf) -> Self {
        Self {
            map,
            map_path,
            cursor_col: 0,
            cursor_row: 0,
            selected_tile: 0,
            selected_prop: 0,
            mode: EditMode::Tiles,
            prop_yaw: 0.0,
            prop_height: 0.0,
            last_placed_cell: None,
            dirty_visuals: true,
            status: "Ready".to_owned(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditMode {
    Tiles,
    Props,
}

fn system_setup_editor(mut commands: Commands, state: Res<EditorState>) {
    let width = state.map.grid.first().map_or(1.0, |r| r.len() as f32);
    let height = state.map.grid.len() as f32;

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_rotation(Quat::from_euler(bevy::math::EulerRot::XYZ, -0.9, 0.4, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: bevy::color::Color::WHITE,
        brightness: 420.0,
        affects_lightmapped_meshes: true,
    });

    commands.spawn((
        EditorCamera,
        Camera3d::default(),
        Transform::from_xyz(width * 0.5, 25.0, height * 1.1).looking_at(
            Vec3::new(width * 0.5, 0.0, height * 0.5),
            Vec3::Y,
        ),
    ));
}

fn system_editor_ui(mut egui_ctx: EguiContexts, mut state: ResMut<EditorState>) {
    let Ok(ctx) = egui_ctx.ctx_mut() else {
        return;
    };

    egui::TopBottomPanel::top("editor_top_panel").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            let selected_label = match state.mode {
                EditMode::Tiles => format!("{:?}", TILE_PALETTE[state.selected_tile]),
                EditMode::Props => PROP_PALETTE[state.selected_prop].to_owned(),
            };
            ui.label(format!(
                "Map: {} | Cursor: ({}, {}) | Selected: {}",
                state.map_path.display(),
                state.cursor_col,
                state.cursor_row,
                selected_label
            ));

            if ui
                .selectable_label(state.mode == EditMode::Tiles, "Tile mode")
                .clicked()
            {
                state.mode = EditMode::Tiles;
            }
            if ui
                .selectable_label(state.mode == EditMode::Props, "Prop mode")
                .clicked()
            {
                state.mode = EditMode::Props;
            }

            if ui.button("Save").clicked() {
                match save_map(&state.map_path, &state.map) {
                    Ok(()) => state.status = format!("Saved {}", state.map_path.display()),
                    Err(error) => state.status = format!("Save failed: {error}"),
                }
            }
        });

        ui.separator();

        match state.mode {
            EditMode::Tiles => {
                ui.horizontal_wrapped(|ui| {
                    for (idx, tile) in TILE_PALETTE.iter().enumerate() {
                        let selected = idx == state.selected_tile;
                        if ui.selectable_label(selected, format!("{tile:?}")).clicked() {
                            state.selected_tile = idx;
                        }
                    }
                });
                ui.label("Controls: Mouse moves cursor, hold Left Click paints, Enter paints once, Backspace resets.");
            }
            EditMode::Props => {
                ui.horizontal_wrapped(|ui| {
                    for (idx, prop) in PROP_PALETTE.iter().enumerate() {
                        let selected = idx == state.selected_prop;
                        if ui.selectable_label(selected, *prop).clicked() {
                            state.selected_prop = idx;
                        }
                    }
                });
                ui.label("Controls: Mouse moves cursor, hold Left Click places props, Enter places once, Delete removes in cell.");
                ui.label("Q/E rotate prop yaw, R/F (or PageUp/PageDown) change height.");
                ui.label(format!(
                    "Current prop yaw: {:.2} rad | height: {:.2}",
                    state.prop_yaw, state.prop_height
                ));
            }
        }

        ui.separator();
        ui.label("View: mouse wheel zoom, WASD pan, V reset view.");
        ui.label(format!("Status: {}", state.status));
    });
}

fn system_cursor_from_mouse(
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut state: ResMut<EditorState>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    let origin = ray.origin;
    let direction = ray.direction.as_vec3();
    if direction.y.abs() < 0.0001 {
        return;
    }
    let t = -origin.y / direction.y;
    if t < 0.0 {
        return;
    }
    let world_hit = origin + direction * t;

    let max_row = state.map.grid.len().saturating_sub(1);
    let max_col = state
        .map
        .grid
        .first()
        .map_or(0, |row| row.len().saturating_sub(1));
    let col = world_hit.x.round().clamp(0.0, max_col as f32) as usize;
    let row = world_hit.z.round().clamp(0.0, max_row as f32) as usize;

    if col != state.cursor_col || row != state.cursor_row {
        state.cursor_col = col;
        state.cursor_row = row;
        state.dirty_visuals = true;
    }
}

fn system_cursor_move(keyboard: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>) {
    let max_row = state.map.grid.len().saturating_sub(1);
    let max_col = state
        .map
        .grid
        .first()
        .map_or(0, |row| row.len().saturating_sub(1));

    let mut changed = false;

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        state.cursor_row = state.cursor_row.saturating_sub(1);
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        state.cursor_row = (state.cursor_row + 1).min(max_row);
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        state.cursor_col = state.cursor_col.saturating_sub(1);
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        state.cursor_col = (state.cursor_col + 1).min(max_col);
        changed = true;
    }

    if keyboard.just_pressed(KeyCode::KeyQ) {
        state.prop_yaw += std::f32::consts::FRAC_PI_8;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        state.prop_yaw -= std::f32::consts::FRAC_PI_8;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::PageUp) || keyboard.just_pressed(KeyCode::KeyR) {
        state.prop_height += 0.1;
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::PageDown) || keyboard.just_pressed(KeyCode::KeyF) {
        state.prop_height -= 0.1;
        changed = true;
    }

    if changed {
        state.dirty_visuals = true;
    }
}

fn system_camera_view_controls(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut wheel_events: bevy::ecs::message::MessageReader<MouseWheel>,
    mut camera_q: Query<&mut Transform, With<EditorCamera>>,
    state: Res<EditorState>,
) {
    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    let mut moved = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        moved.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        moved.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        moved.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        moved.x += 1.0;
    }
    if moved != Vec3::ZERO {
        transform.translation += moved.normalize_or_zero() * 12.0 * time.delta_secs();
    }

    let mut wheel_delta = 0.0;
    for event in wheel_events.read() {
        wheel_delta += event.y;
    }
    if wheel_delta.abs() > 0.001 {
        transform.translation.y = (transform.translation.y - wheel_delta * 1.2).clamp(6.0, 80.0);
    }

    if keyboard.just_pressed(KeyCode::KeyV) {
        let width = state.map.grid.first().map_or(1.0, |r| r.len() as f32);
        let height = state.map.grid.len() as f32;
        *transform = Transform::from_xyz(width * 0.5, 25.0, height * 1.1).looking_at(
            Vec3::new(width * 0.5, 0.0, height * 0.5),
            Vec3::Y,
        );
    }
}

fn system_apply_action(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut repeat_cooldown: Local<f32>,
    mut state: ResMut<EditorState>,
) {
    *repeat_cooldown = (*repeat_cooldown - time.delta_secs()).max(0.0);
    match state.mode {
        EditMode::Tiles => {
            if keyboard.just_pressed(KeyCode::Enter) || mouse.pressed(MouseButton::Left) {
                let row = state.cursor_row;
                let col = state.cursor_col;
                let selected = state.selected_tile;
                let cell = TILE_PALETTE[selected];
                let held = mouse.pressed(MouseButton::Left);
                if !held || state.last_placed_cell != Some((row, col)) {
                    if state.map.grid[row][col] != cell {
                        state.map.grid[row][col] = cell;
                        state.dirty_visuals = true;
                        state.status = "Tile painted".to_owned();
                    }
                    state.last_placed_cell = Some((row, col));
                }
            }
            if keyboard.just_pressed(KeyCode::Backspace) {
                let row = state.cursor_row;
                let col = state.cursor_col;
                state.map.grid[row][col] = CellType::Grass;
                state.dirty_visuals = true;
                state.status = "Tile reset to Grass".to_owned();
            }
        }
        EditMode::Props => {
            let place_once = keyboard.just_pressed(KeyCode::Enter);
            let place_hold = mouse.pressed(MouseButton::Left) && *repeat_cooldown <= 0.0;
            if place_once || place_hold {
                let col = state.cursor_col as f32;
                let row = state.cursor_row as f32;
                let prop_height = state.prop_height;
                let prop_yaw = state.prop_yaw;
                let model = PROP_PALETTE[state.selected_prop].to_owned();
                state.map.props.push(PropSpawn {
                    model,
                    position: [col, prop_height, row],
                    yaw: prop_yaw,
                });
                state.dirty_visuals = true;
                state.status = "Prop placed".to_owned();
                if place_hold {
                    *repeat_cooldown = 0.08;
                }
            }
            if keyboard.just_pressed(KeyCode::Delete) {
                let cell_col = state.cursor_col as f32;
                let cell_row = state.cursor_row as f32;
                let before = state.map.props.len();
                state.map.props.retain(|prop| {
                    let dx = prop.position[0] - cell_col;
                    let dz = prop.position[2] - cell_row;
                    dx.abs() > 0.5 || dz.abs() > 0.5
                });
                if state.map.props.len() != before {
                    state.dirty_visuals = true;
                    state.status = "Removed props in cursor cell".to_owned();
                }
            }
        }
    }

    if mouse.just_released(MouseButton::Left) {
        state.last_placed_cell = None;
    }
}

fn load_scene(asset_server: &AssetServer, name: &str) -> SceneRoot {
    let path = format!("{ASSET_BASE}{name}#Scene0");
    SceneRoot(asset_server.load(path))
}

fn tile_to_model(cell: CellType) -> (&'static str, f32) {
    match cell {
        CellType::Grass => ("grass.glb", 0.0),
        CellType::RoadNs => ("road-asphalt-straight.glb", 0.0),
        CellType::RoadEw => ("road-asphalt-straight.glb", std::f32::consts::FRAC_PI_2),
        CellType::RoadIntersection => ("road-asphalt-center.glb", 0.0),
        CellType::BuildingZone => ("grass.glb", 0.0),
        CellType::Parking => ("road-asphalt-pavement.glb", 0.0),
        CellType::Park => ("grass.glb", 0.0),
    }
}

fn system_rebuild_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<EditorState>,
    old_visuals: Query<bevy::ecs::entity::Entity, bevy::ecs::query::With<EditorVisual>>,
) {
    if !state.dirty_visuals {
        return;
    }

    for entity in &old_visuals {
        commands.entity(entity).despawn();
    }

    for (row_idx, row) in state.map.grid.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let (model, yaw) = tile_to_model(*cell);
            let mut transform = Transform::from_xyz(col_idx as f32, 0.0, row_idx as f32);
            if yaw.abs() > 0.001 {
                transform.rotation = Quat::from_rotation_y(yaw);
            }

            commands.spawn((EditorVisual, load_scene(&asset_server, model), transform));

            if *cell == CellType::BuildingZone {
                commands.spawn((
                    EditorVisual,
                    load_scene(&asset_server, "wall-a.glb"),
                    Transform::from_xyz(col_idx as f32 + 0.5, 0.0, row_idx as f32 + 0.5),
                ));
            }
            if *cell == CellType::Park {
                commands.spawn((
                    EditorVisual,
                    load_scene(&asset_server, "tree-park-large.glb"),
                    Transform::from_xyz(col_idx as f32, 0.0, row_idx as f32),
                ));
            }
            if *cell == CellType::Parking {
                commands.spawn((
                    EditorVisual,
                    load_scene(&asset_server, "truck-green.glb"),
                    Transform::from_xyz(col_idx as f32, 0.0, row_idx as f32),
                ));
            }
        }
    }

    for prop in &state.map.props {
        commands.spawn((
            EditorVisual,
            load_scene(&asset_server, &prop.model),
            Transform::from_xyz(prop.position[0], prop.position[1], prop.position[2])
                .with_rotation(Quat::from_rotation_y(prop.yaw)),
        ));
    }

    commands.spawn((
        EditorVisual,
        load_scene(&asset_server, "detail-block.glb"),
        Transform::from_xyz(state.cursor_col as f32, 0.02, state.cursor_row as f32)
            .with_scale(Vec3::splat(0.35)),
    ));

    // Preview currently selected element at the cursor so placement is visible before applying.
    match state.mode {
        EditMode::Tiles => {
            let selected = TILE_PALETTE[state.selected_tile];
            let (model, yaw) = tile_to_model(selected);
            let mut transform =
                Transform::from_xyz(state.cursor_col as f32, 0.12, state.cursor_row as f32)
                    .with_scale(Vec3::splat(1.02));
            if yaw.abs() > 0.001 {
                transform.rotation = Quat::from_rotation_y(yaw);
            }
            commands.spawn((EditorVisual, load_scene(&asset_server, model), transform));
        }
        EditMode::Props => {
            let model = PROP_PALETTE[state.selected_prop];
            commands.spawn((
                EditorVisual,
                load_scene(&asset_server, model),
                Transform::from_xyz(
                    state.cursor_col as f32,
                    state.prop_height + 0.08,
                    state.cursor_row as f32,
                )
                .with_rotation(Quat::from_rotation_y(state.prop_yaw)),
            ));
        }
    }

    state.dirty_visuals = false;
}
