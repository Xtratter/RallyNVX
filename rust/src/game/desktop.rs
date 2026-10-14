//! Desktop game mode: `raytrace-desktop --game [seed]`. Keys: W/Up throttle, S/Down reverse, A/D or Left/Right steer,
//! Space brake, Left Shift handbrake, R reset, N new random level, Esc quit.
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use super::app::{Game, STAT_LEN};
use crate::raster::Raster;

fn random_seed() -> u64 {
    super::app::level_seed(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64))
}

struct State { window: Arc<Window>, game: Game, keys: HashSet<KeyCode>, autodrive: bool }

struct App { seed: u64, state: Option<State> }

impl State {
    fn new(el: &ActiveEventLoop, seed: u64) -> State {
        let attrs = Window::default_attributes().with_title("Raytrace - Game").with_inner_size(LogicalSize::new(1280.0, 720.0));
        let window = Arc::new(el.create_window(attrs).expect("create window"));
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN, ..Default::default() });
        let surface = instance.create_surface(window.clone()).expect("create_surface");
        let raster = Raster::from_surface(instance, surface, Box::new(window.clone()), size.width.max(1), size.height.max(1));
        let mut game = Game::new(raster, seed, Arc::new(Mutex::new([0.0; STAT_LEN])));
        if let Some(p) = std::env::var_os("RAYTRACE_SURFACES") { crate::game::grip::load_override(std::path::Path::new(&p)); }
        if let Some(r) = std::env::var("RAYTRACE_CITY_RADIUS").ok().and_then(|s| s.parse::<f32>().ok()) { game.set_city_radius(r); }
        if std::env::var("RAYTRACE_AT_CITY").map_or(false, |v| v == "1") { game.place_near_city(); }
        State { window, game, keys: HashSet::new(), autodrive: std::env::var("RAYTRACE_AUTODRIVE").map_or(false, |v| v == "1") }
    }

    fn held(&self, ks: &[KeyCode]) -> f32 { ks.iter().any(|k| self.keys.contains(k)) as u32 as f32 }

    fn redraw(&mut self) {
        let throttle = self.held(&[KeyCode::KeyW, KeyCode::ArrowUp]) - self.held(&[KeyCode::KeyS, KeyCode::ArrowDown]);
        let steer = self.held(&[KeyCode::KeyD, KeyCode::ArrowRight]) - self.held(&[KeyCode::KeyA, KeyCode::ArrowLeft]);
        self.game.input.throttle = if self.autodrive { 1.0 } else { throttle };
        // CI smoke: with RAYTRACE_AUTODRIVE the car drives straight ahead with the throttle held
        self.game.input.steer = if self.autodrive { 0.0 } else { steer };
        self.game.input.brake = self.keys.contains(&KeyCode::Space);
        self.game.input.handbrake = self.keys.contains(&KeyCode::ShiftLeft);
        self.game.frame();
        self.window.set_title(&format!("Raytrace - Game | {:.0} km/h | {:.0} fps | seed {}", self.game.speed_kmh(), self.game.fps(), self.game.seed()));
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.state.is_none() { self.state = Some(State::new(el, self.seed)); }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        let Some(s) = self.state.as_mut() else { return };
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(sz) => s.game.resize(sz.width, sz.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(k) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        if !event.repeat {
                            match k {
                                KeyCode::Escape => el.exit(),
                                KeyCode::KeyR => s.game.input.reset = true,
                                KeyCode::KeyN => s.game.new_level(random_seed()),
                                _ => {}
                            }
                        }
                        s.keys.insert(k);
                    } else {
                        s.keys.remove(&k);
                    }
                }
            }
            WindowEvent::Focused(false) => s.keys.clear(),
            WindowEvent::RedrawRequested => s.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(s) = &self.state { s.window.request_redraw(); }
    }
}

/// Runs the game until the window closes. Seed: argument, else `RAYTRACE_SEED`, else random.
pub fn run(seed: Option<u64>) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let seed = seed.or_else(|| std::env::var("RAYTRACE_SEED").ok().and_then(|s| s.parse().ok())).map(super::app::level_seed).unwrap_or_else(random_seed);
    log::info!("game seed {seed}");
    let el = EventLoop::new().expect("event loop");
    el.set_control_flow(ControlFlow::Poll);
    el.run_app(&mut App { seed, state: None }).expect("run");
}
