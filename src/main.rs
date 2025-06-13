use bevy::prelude::*;
use rand::prelude::*;

// Game constants
const PLAYER_SIZE: Vec2 = Vec2::new(50.0, 50.0);
const PLAYER_SPEED: f32 = 500.0;
const ENEMY_SIZE: Vec2 = Vec2::new(40.0, 40.0);
const ENEMY_SPEED: f32 = 300.0;
const ENEMY_SPAWN_TIME: f32 = 0.75; // Spawn a new enemy every 0.75 seconds

// --- Components ---
// Components are data that you attach to entities.

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Velocity(Vec2);

// --- Resources ---
// Resources are global data that can be accessed by any system.

#[derive(Resource)]
struct EnemySpawnTimer(Timer);

// Game state to control flow (e.g., Playing vs. GameOver)
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}
fn collide(
    pos_a: Vec3,
    size_a: Vec2,
    pos_b: Vec3,
    size_b: Vec2,
) -> bool {
    let a_min = pos_a.truncate() - size_a / 2.0;
    let a_max = pos_a.truncate() + size_a / 2.0;
    let b_min = pos_b.truncate() - size_b / 2.0;
    let b_max = pos_b.truncate() + size_b / 2.0;

    a_min.x < b_max.x && a_max.x > b_min.x &&
    a_min.y < b_max.y && a_max.y > b_min.y
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>() // Correctly initialize the game state
        .insert_resource(EnemySpawnTimer(Timer::from_seconds(
            ENEMY_SPAWN_TIME,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Playing), setup_game)
        .add_systems(
            Update,
            (
                player_movement,
                move_entities,
                enemy_spawner,
                check_collisions,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, restart_game.run_if(in_state(GameState::GameOver)))
        .add_systems(OnEnter(GameState::GameOver), game_over_message)
        .add_systems(OnExit(GameState::GameOver), despawn_all_entities)
        .run();
}

/// System to set up the 2D camera
fn setup_camera(mut commands: Commands) {
    // Spawning a 2D camera is now done by just spawning the component
    commands.spawn(Camera2d::default());
}

/// System to set up the initial game state (player)
fn setup_game(mut commands: Commands) {
    // Spawn player
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.2, 0.4, 0.8), // Use srgb for colors
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0.0, -250.0, 0.0),
                scale: PLAYER_SIZE.extend(1.0),
                ..default()
            },
            ..default()
        },
        Player,
        Velocity(Vec2::ZERO),
    ));
}

/// System to handle player input for movement
fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    if let Ok(mut player_velocity) = query.get_single_mut() {
        let mut direction = Vec2::ZERO;

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        // Normalize to ensure consistent speed in all directions and apply speed
        player_velocity.0 = direction.normalize_or_zero() * PLAYER_SPEED;
    }
}

/// A unified system to move all entities with a Velocity component and clamp the player to the screen.
fn move_entities(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity, Option<&Player>)>,
    window_query: Query<&Window>,
) {
    let window = window_query.single();
    let half_player_width = PLAYER_SIZE.x / 2.0;
    let x_min = -window.width() / 2.0 + half_player_width;
    let x_max = window.width() / 2.0 - half_player_width;

    for (mut transform, velocity, maybe_player) in &mut query {
        // Apply velocity to move the entity using the updated Time API
        transform.translation += velocity.0.extend(0.0) * time.delta().as_secs_f32();

        // If the entity is the player, clamp its position to the screen bounds
        if maybe_player.is_some() {
            transform.translation.x = transform.translation.x.clamp(x_min, x_max);
        }
    }
}

/// System to spawn new enemies periodically
fn enemy_spawner(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    window_query: Query<&Window>,
) {
    // Tick the timer
    spawn_timer.0.tick(time.delta());

    // If the timer just finished, spawn an enemy
    if spawn_timer.0.just_finished() {
        let window = window_query.single();
        let half_enemy_width = ENEMY_SIZE.x / 2.0;
        let x_spawn_range =
            -window.width() / 2.0 + half_enemy_width..window.width() / 2.0 - half_enemy_width;
        let y_spawn_pos = window.height() / 2.0;

        let mut rng = rand::thread_rng();

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.9, 0.2, 0.2), // Use srgb for colors
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(rng.gen_range(x_spawn_range), y_spawn_pos, 0.0),
                    scale: ENEMY_SIZE.extend(1.0),
                    ..default()
                },
                ..default()
            },
            Enemy,
            Velocity(Vec2::new(0.0, -ENEMY_SPEED)),
        ));
    }
}

/// System to check for collisions between the player and enemies
fn check_collisions(
    mut commands: Commands,
    player_query: Query<(&Transform, Entity), With<Player>>,
    enemy_query: Query<&Transform, With<Enemy>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok((player_transform, player_entity)) = player_query.get_single() {
        for enemy_transform in &enemy_query {
            if collide(
                player_transform.translation,
                player_transform.scale.truncate(),
                enemy_transform.translation,
                enemy_transform.scale.truncate(),
            ) {
                // Collision detected! Despawn player and end game.
                println!("Collision! Game Over.");
                commands.entity(player_entity).despawn();
                next_state.set(GameState::GameOver);
                break;
            }
        }
    }
}

/// System that shows the "Game Over" message using the modern Text2dBundle
fn game_over_message(mut commands: Commands) {
    commands.spawn((
        Text("Game Over!\nPress 'R' to Restart".to_string()),
        Transform::from_xyz(0.0, 0.0, 1.0),
        GlobalTransform::default(),
        Visibility::Visible,
    ));
}

/// System to restart the game
fn restart_game(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        next_state.set(GameState::Playing);
    }
}

/// System to despawn all entities (enemies and text) when restarting
fn despawn_all_entities(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Enemy>, With<Text>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}