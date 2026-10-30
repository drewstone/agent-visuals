//! Offline renderer for generative 3D agent networks.
//!
//! Produces looping PNG frames of soft glowing agents with streaming
//! particle connections. Dark background, no labels, deterministic.

use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use image::{Rgb, RgbImage};

struct Options {
    width: u32,
    height: u32,
    frames: usize,
    out: PathBuf,
}

const USAGE: &str = "usage: agent-visuals [--width N] [--height N] [--frames N] <out-dir>";

fn parse_args() -> Result<Options, String> {
    let mut opts = Options {
        width: 1200,
        height: 630,
        frames: 60,
        out: PathBuf::new(),
    };
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| -> Result<u32, String> {
            args.next()
                .ok_or_else(|| format!("{name} needs a value"))?
                .parse()
                .map_err(|e| format!("{name}: {e}"))
        };
        match arg.as_str() {
            "--width" => opts.width = value("--width")?,
            "--height" => opts.height = value("--height")?,
            "--frames" => opts.frames = value("--frames")? as usize,
            "-h" | "--help" => return Err(USAGE.to_owned()),
            _ if arg.starts_with('-') => return Err(format!("unknown option {arg}\n{USAGE}")),
            _ if out.is_none() => out = Some(PathBuf::from(arg)),
            _ => return Err(format!("unexpected argument {arg}\n{USAGE}")),
        }
    }
    opts.out = out.ok_or_else(|| USAGE.to_owned())?;
    if opts.width == 0 || opts.height == 0 || opts.frames == 0 {
        return Err("dimensions and frame count must be positive".to_owned());
    }
    Ok(opts)
}

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
    fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

struct Agent {
    pos: Vec3,
    phase: f32,
    radius: f32,
    color: (u8, u8, u8),
}

struct Particle {
    from: usize,
    to: usize,
    t: f32,
    speed: f32,
}

fn seed_agents() -> Vec<Agent> {
    // Deterministic cluster of agents
    let positions = [
        (0.0, 0.0, 0.0),
        (1.2, 0.4, -0.3),
        (-0.8, 0.9, 0.2),
        (0.3, -1.1, 0.5),
        (-1.0, -0.5, -0.4),
        (0.7, 0.2, 0.8),
        (-0.4, 0.6, -0.7),
        (1.5, -0.3, 0.1),
    ];
    let colors = [
        (180, 220, 255),
        (120, 200, 255),
        (200, 180, 255),
        (160, 210, 240),
        (140, 190, 255),
        (190, 200, 255),
        (170, 215, 250),
        (150, 195, 245),
    ];
    positions
        .iter()
        .zip(colors.iter())
        .enumerate()
        .map(|(i, (p, c))| Agent {
            pos: Vec3::new(p.0, p.1, p.2),
            phase: i as f32 * 0.7,
            radius: 0.18 + (i as f32 * 0.03),
            color: *c,
        })
        .collect()
}

fn seed_particles(n_agents: usize) -> Vec<Particle> {
    let mut particles = Vec::new();
    let connections = [
        (0, 1), (0, 2), (0, 3), (0, 4),
        (1, 5), (1, 7), (2, 6), (3, 5),
        (4, 6), (5, 7), (2, 3), (4, 0),
    ];
    for (i, &(a, b)) in connections.iter().enumerate() {
        for k in 0..3 {
            particles.push(Particle {
                from: a,
                to: b,
                t: (k as f32) / 3.0 + (i as f32 * 0.1),
                speed: 0.4 + (i % 3) as f32 * 0.15,
            });
        }
    }
    particles
}

fn project(v: Vec3, w: f32, h: f32, time: f32) -> (f32, f32, f32) {
    // Simple orbiting camera
    let angle = time * 0.15;
    let cam_x = angle.cos() * 4.5;
    let cam_z = angle.sin() * 4.5;
    let cam_y = 0.8;
    let dx = v.x - cam_x;
    let dy = v.y - cam_y;
    let dz = v.z - cam_z;
    let depth = (dx * dx + dy * dy + dz * dz).sqrt();
    let scale = 280.0 / (depth + 1.0);
    let px = w * 0.5 + dx * scale;
    let py = h * 0.5 - dy * scale;
    (px, py, depth)
}

fn draw_glow(img: &mut RgbImage, cx: f32, cy: f32, radius: f32, color: (u8, u8, u8), intensity: f32) {
    let r = (radius * 18.0) as i32;
    let (w, h) = (img.width() as i32, img.height() as i32);
    for dy in -r..=r {
        for dx in -r..=r {
            let x = cx as i32 + dx;
            let y = cy as i32 + dy;
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let dist = ((dx * dx + dy * dy) as f32).sqrt() / (radius * 18.0);
            if dist > 1.0 {
                continue;
            }
            let falloff = (1.0 - dist).powi(2) * intensity;
            let pixel = img.get_pixel_mut(x as u32, y as u32);
            let add = |c: u8, a: f32| -> u8 { (c as f32 + a * 255.0).min(255.0) as u8 };
            *pixel = Rgb([
                add(pixel[0], falloff * color.0 as f32 / 255.0),
                add(pixel[1], falloff * color.1 as f32 / 255.0),
                add(pixel[2], falloff * color.2 as f32 / 255.0),
            ]);
        }
    }
}

fn render_frame(agents: &[Agent], particles: &mut [Particle], time: f32, w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(w, h, Rgb([8, 10, 18]));

    // Advance particles
    for p in particles.iter_mut() {
        p.t = (p.t + p.speed * 0.016) % 1.0;
    }

    // Draw connections / particles first (behind agents)
    for p in particles.iter() {
        let a = agents[p.from].pos;
        let b = agents[p.to].pos;
        let pos = a.mul(1.0 - p.t).add(b.mul(p.t));
        let (px, py, depth) = project(pos, w as f32, h as f32, time);
        let intensity = 0.35 / (depth * 0.15 + 0.4);
        draw_glow(&mut img, px, py, 0.06, (140, 200, 255), intensity);
    }

    // Draw agents
    for agent in agents {
        let bob = (time * 0.8 + agent.phase).sin() * 0.08;
        let pos = agent.pos.add(Vec3::new(0.0, bob, 0.0));
        let (px, py, depth) = project(pos, w as f32, h as f32, time);
        let intensity = 1.2 / (depth * 0.12 + 0.5);
        draw_glow(&mut img, px, py, agent.radius, agent.color, intensity);
    }

    img
}

fn main() -> ExitCode {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    std::fs::create_dir_all(&opts.out).unwrap_or_else(|e| {
        eprintln!("create dir: {e}");
        std::process::exit(1);
    });

    let agents = seed_agents();
    let mut particles = seed_particles(agents.len());

    for frame in 0..opts.frames {
        let time = frame as f32 / opts.frames as f32 * TAU;
        let img = render_frame(&agents, &mut particles, time, opts.width, opts.height);
        let path = opts.out.join(format!("{frame:04}.png"));
        if let Err(e) = img.save(&path) {
            eprintln!("save {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        eprint!("\rframe {}/{}", frame + 1, opts.frames);
    }
    eprintln!("\ndone");
    ExitCode::SUCCESS
}
