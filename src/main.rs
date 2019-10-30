use generic_array::typenum::U2;
use sdfu::SDF;

mod animation;
mod camera;
mod film;
mod filter;
mod hitable;
mod integrator;
mod material;
mod math;
mod ray;
mod sdf;
mod spectrum;
mod sphere;
mod world;

use camera::{CameraHandle, CameraStore, ThinLensCamera};
use film::{ChannelKind, Film};
use filter::{BlackmanHarrisFilter, BoxFilter};
use hitable::HitableStore;
use integrator::PathTracingIntegrator;
use material::{Dielectric, MaterialStore, Sky};
use math::{Extent2u, Vec3, Wec3};
use sdf::TracedSDF;
use spectrum::{Srgb, WSrgb};
use sphere::Sphere;
use world::World;

use std::time::Instant;

use wide::f32x4;

const RES: (usize, usize) = (1280, 720);
const SAMPLES: usize = 32;

fn setup() -> (CameraHandle, World) {
    let mut materials = MaterialStore::new();
    let ground = materials.add_material(Dielectric::new(
        WSrgb::splat(Srgb::new(0.25, 0.2, 0.35)),
        f32x4::from(0.0),
    ));

    let sky = materials.add_material(Sky {});

    let mut hitables = HitableStore::new();
    hitables.push(Box::new(Sphere::new(
        Vec3::new(0.0, -200.5, -1.0),
        200.0,
        ground,
    )));

    hitables.push(Box::new(Sphere::new(Vec3::new(0.0, 0.0, 0.0), 300.0, sky)));
    // hitables.push(Box::new(TracedSDF::new(
    //     sdfu::Sphere::<f32>::new(0.45)
    //         .subtract(sdfu::Box::new(Vec3::new(0.25, 0.25, 1.5)))
    //         .union_smooth(
    //             sdfu::Sphere::<f32>::new(0.3).translate(Vec3::new(0.3, 0.3, 0.0)),
    //             0.1,
    //         )
    //         .union_smooth(
    //             sdfu::Sphere::<f32>::new(0.3).translate(Vec3::new(-0.3, 0.3, 0.0)),
    //             0.1,
    //         )
    //         .subtract(
    //             sdfu::Box::new(Vec3::new(0.125, 0.125, 1.5)).translate(Vec3::new(-0.3, 0.3, 0.0)),
    //         )
    //         .subtract(
    //             sdfu::Box::new(Vec3::new(0.125, 0.125, 1.5)).translate(Vec3::new(0.3, 0.3, 0.0)),
    //         )
    //         .subtract(sdfu::Box::new(Vec3::new(1.5, 0.1, 0.1)).translate(Vec3::new(0.0, 0.3, 0.0)))
    //         .subtract(sdfu::Box::new(Vec3::new(0.2, 2.0, 0.2)))
    //         .translate(Vec3::new(-0.2, 0.0, -1.0)),
    //     Sphere::new(
    //         TransformSequence::new(Vec3::new(-0.2, 0.0, -1.0), Quat::default()),
    //         1.0,
    //         ground,
    //     ),
    //     checkerboard,
    // )));
    hitables.push(Box::new(Sphere::new(
        Vec3::new(0.0, 0.0, -1.0),
        1.0,
        ground,
    )));
    // hitables.push(Box::new(Sphere::new(
    //     TransformSequence::new(Vec3::new(1.0, -0.25, -1.0), Quat::default()),
    //     0.25,
    //     gold,
    // )));
    // hitables.push(Box::new(Sphere::new(
    //     TransformSequence::new(Vec3::new(-0.8, -0.2, -0.5), Quat::default()),
    //     0.3,
    //     silver,
    // )));
    // hitables.push(Box::new(Sphere::new(
    //     TransformSequence::new(
    //         |t: f32| -> Vec3 {
    //             Vec3::new(
    //                 0.2 - (t * std::f32::consts::PI).sin() * 0.15,
    //                 -0.4,
    //                 -0.35 - (t * std::f32::consts::PI).cos() * 0.15,
    //             )
    //         },
    //         Quat::default(),
    //     ),
    //     0.1,
    //     glass,
    // )));
    // hitables.push(Box::new(Sphere::new(
    //     TransformSequence::new(Vec3::new(-0.25, -0.375, -0.15), Quat::default()),
    //     0.125,
    //     glass_rough,
    // )));
    // hitables.push(Box::new(Sphere::new(
    //     TransformSequence::new(
    //         |t: f32| -> Vec3 {
    //             Vec3::new(
    //                 -0.5 + (2.0 * t * std::f32::consts::PI).cos() * 1.5,
    //                 -0.375,
    //                 -0.5 - (2.0 * t * std::f32::consts::PI).sin() * 1.5,
    //             )
    //         },
    //         Quat::default(),
    //     ),
    //     0.125,
    //     gold_rough,
    // )));

    let camera = ThinLensCamera::new(
        RES.0 as f32 / RES.1 as f32,
        60.0,
        0.035,
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(-0.2, 0.0, -0.7),
    );

    let mut cameras = CameraStore::new();

    let camera = cameras.add_camera(Box::new(camera));

    (
        camera,
        World {
            materials,
            hitables,
            cameras,
        },
    )
}

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build_global()
        .unwrap();

    let (camera, world) = setup();

    let mut film = Film::<U2>::new(
        &[ChannelKind::Color, ChannelKind::Background],
        Extent2u::new(RES.0, RES.1),
    )
    .unwrap();

    let frame_rate = 24;
    let frame_range = 0..1;
    let shutter_speed = 1.0 / 24.0;

    let filter = BlackmanHarrisFilter::new(2.0);
    // let filter = BoxFilter::default();
    let integrator = PathTracingIntegrator { max_bounces: 3 };

    for frame in frame_range {
        let start = Instant::now();

        let frame_start = frame as f32 * (1.0 / frame_rate as f32);
        let frame_end = frame_start + shutter_speed;

        film.render_frame_into(
            &world,
            camera,
            &integrator,
            &filter,
            Extent2u::new(16, 16),
            frame_start..frame_end,
            SAMPLES,
        );

        let time = Instant::now() - start;
        let time_secs = time.as_secs();
        let time_millis = time.subsec_millis();

        println!(
            "Done in {} seconds.",
            time_secs as f32 + time_millis as f32 / 1000.0
        );

        println!("Post processing image...");

        film.save_to(
            &[ChannelKind::Color],
            "renders",
            format!("frame{}_blackman", frame),
            false,
        )
        .unwrap();
    }
}
