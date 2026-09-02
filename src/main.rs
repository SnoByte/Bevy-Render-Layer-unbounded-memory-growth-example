use bevy::{
    asset::RenderAssetUsages,
    camera::{
        Viewport,
        visibility::RenderLayers,
    },
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    image::ImageSampler,
    light::CascadeShadowConfigBuilder,
    material::OpaqueRendererMethod,
    mesh::{Indices, PrimitiveTopology},
    pbr::StandardMaterial,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::{Msaa, NoIndirectDrawing},
    },
    window::{PresentMode, WindowPlugin},
};

const TOGGLE_SECONDS: f32 = 0.5;
const TEST_LAYER: usize = 1;

#[derive(Resource, Clone, Copy)]
struct ShadowTestConfig {
    shadows_enabled: bool,
}

#[derive(Resource)]
struct CameraLayerToggle {
    timer: Timer,
    camera_on_test_layer: bool,
    completed_cycles: u64,
}

fn main() {
    let shadows_enabled = shadows_enabled_from_cli();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.025, 0.03, 0.04)))
        .insert_resource(ShadowTestConfig { shadows_enabled })
        .insert_resource(CameraLayerToggle {
            timer: Timer::from_seconds(TOGGLE_SECONDS, TimerMode::Repeating),
            camera_on_test_layer: false,
            completed_cycles: 0,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!(
                    "Bevy camera RenderLayers shadow leak repro — shadows {}",
                    if shadows_enabled { "ON" } else { "OFF" }
                ),
                resolution: (960, 480).into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_camera_layers_forever)
        .run();
}

fn shadows_enabled_from_cli() -> bool {
    let mut shadows_enabled = true;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--shadows" => shadows_enabled = true,
            "--no-shadows" => shadows_enabled = false,
            "-h" | "--help" => {
                println!(
                    "Camera RenderLayers shadow leak repro\n\n\
                     Usage:\n\
                       cargo run -- --shadows       # shadow maps enabled (default)\n\
                       cargo run -- --no-shadows    # shadow maps disabled control\n\n\
                     The test runs forever and toggles the existing RenderLayers value on\n\
                     both Camera3d entities between the default layer and layer {TEST_LAYER}."
                );
                std::process::exit(0);
            }
            other => panic!(
                "unknown argument {other:?}; expected --shadows, --no-shadows, or --help"
            ),
        }
    }

    shadows_enabled
}

fn region_mesh(x_min: f32, x_max: f32) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [x_min, -1.0, 0.0],
            [x_max, -1.0, 0.0],
            [x_max, 1.0, 0.0],
            [x_min, 1.0, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]))
}

fn setup(
    mut commands: Commands,
    config: Res<ShadowTestConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let opaque_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.12, 0.08),
        opaque_render_method: OpaqueRendererMethod::Forward,
        ..default()
    });

    let mut checker_pixels = Vec::with_capacity(8 * 8 * 4);
    for y in 0..8 {
        for x in 0..8 {
            let alpha = if (x / 2 + y / 2) % 2 == 0 { 255 } else { 0 };
            checker_pixels.extend_from_slice(&[30, 235, 90, alpha]);
        }
    }

    let mut checker = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        checker_pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    checker.sampler = ImageSampler::nearest();

    let mask_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(checker)),
        alpha_mode: AlphaMode::Mask(0.5),
        opaque_render_method: OpaqueRendererMethod::Forward,
        ..default()
    });

    // A lit receiver makes shadow corruption obvious.
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(1.6, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.25, 0.95),
            opaque_render_method: OpaqueRendererMethod::Forward,
            ..default()
        })),
        Transform::from_xyz(1.0, 0.0, -0.6),
        Name::new("blue shadow receiver"),
    ));

    // Both shadow casters are permanently on layer 1. Only camera RenderLayers mutate.
    commands.spawn((
        Mesh3d(meshes.add(region_mesh(-1.8, -0.2))),
        MeshMaterial3d(opaque_material),
        RenderLayers::layer(TEST_LAYER),
        Name::new("opaque layer-1 shadow caster"),
    ));
    commands.spawn((
        Mesh3d(meshes.add(region_mesh(0.2, 1.8))),
        MeshMaterial3d(mask_material),
        RenderLayers::layer(TEST_LAYER),
        Name::new("masked layer-1 shadow caster"),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: config.shadows_enabled,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.25, 0.35, 0.0)),
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 3.0,
            maximum_distance: 20.0,
            num_cascades: 4,
            ..default()
        }
        .build(),
        Name::new("four-cascade directional light"),
    ));

    // Preserve the point/spot shadow paths from the known reproducer.
    commands.spawn((
        PointLight {
            intensity: 35_000.0,
            range: 12.0,
            shadow_maps_enabled: config.shadows_enabled,
            ..default()
        },
        Transform::from_xyz(-3.0, 2.5, 3.5),
        Name::new("point shadow light"),
    ));
    commands.spawn((
        SpotLight {
            intensity: 45_000.0,
            range: 15.0,
            shadow_maps_enabled: config.shadows_enabled,
            inner_angle: 0.45,
            outer_angle: 0.8,
            ..default()
        },
        Transform::from_xyz(3.0, -2.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("spot shadow light"),
    ));

    // Preserve the two-camera direct/indirect split from the reproducing test.
    // RenderLayers exists from startup; the test only mutates its value in place.
    commands.spawn((
        Camera3d::default(),
        Camera {
            viewport: Some(Viewport {
                physical_position: UVec2::ZERO,
                physical_size: UVec2::new(480, 480),
                ..default()
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::default(),
        NoIndirectDrawing,
        Msaa::Off,
        DepthPrepass,
        DeferredPrepass,
        Name::new("direct camera"),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            viewport: Some(Viewport {
                physical_position: UVec2::new(480, 0),
                physical_size: UVec2::new(480, 480),
                ..default()
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::default(),
        Msaa::Off,
        DepthPrepass,
        DeferredPrepass,
        Name::new("indirect camera"),
    ));

    println!(
        "camera RenderLayers shadow leak repro started; shadows_enabled={}, toggle_seconds={}, test_layer={}; test runs forever",
        config.shadows_enabled, TOGGLE_SECONDS, TEST_LAYER
    );
}

fn toggle_camera_layers_forever(
    time: Res<Time>,
    mut state: ResMut<CameraLayerToggle>,
    mut cameras: Query<&mut RenderLayers, With<Camera3d>>,
) {
    if !state.timer.tick(time.delta()).just_finished() {
        return;
    }

    state.camera_on_test_layer = !state.camera_on_test_layer;

    for mut layers in &mut cameras {
        *layers = if state.camera_on_test_layer {
            RenderLayers::layer(TEST_LAYER)
        } else {
            RenderLayers::default()
        };
    }

    if !state.camera_on_test_layer {
        state.completed_cycles += 1;
    }

    println!(
        "camera RenderLayers toggled: camera_layer={}, completed_cycles={}",
        if state.camera_on_test_layer { TEST_LAYER } else { 0 },
        state.completed_cycles
    );
}
