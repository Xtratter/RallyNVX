//! JNI entry points of the game (dev.starinin.rallynvx.Native.game*) and its render thread.
use crate::game::input::Input;

/// Commands from Kotlin as an `Input` (non-finite axes become 0, axes are clamped).
pub fn merge_input(throttle: f32, steer: f32, brake: bool, handbrake: bool, brake_amt: f32) -> Input {
    Input { throttle, steer, brake, handbrake, brake_amt, reset: false }.sanitized()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn merge_input_sanitizes_and_maps() {
        let i = merge_input(f32::NAN, 2.0, true, false, 0.4);
        assert_eq!((i.throttle, i.steer, i.brake, i.handbrake, i.reset, i.brake_amt), (0.0, 1.0, true, false, false, 0.4));
        assert_eq!(merge_input(0.0, 0.0, true, false, f32::NAN).brake_amt, 0.0);
        assert_eq!(merge_input(0.0, 0.0, true, false, 7.0).brake_amt, 1.0);
        assert_eq!(merge_input(0.0, 0.0, true, false, -2.0).brake_amt, 0.0);
    }
}

#[cfg(target_os = "android")]
mod android {
    use std::sync::mpsc::{channel, Sender, TryRecvError};
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    use jni::objects::{JClass, JObject};
    use jni::sys::{jboolean, jfloat, jfloatArray, jint, jlong};
    use jni::JNIEnv;
    use ndk::native_window::NativeWindow;

    use super::merge_input;
    use crate::game::{app::{Game, STAT_LEN}, input::Input};
    use crate::raster::Raster;

    enum GCmd { Input(Input), OnRoad(i32), Dust(i32), Autopilot(i32), Rally(i32), Signs(bool), Camera(f32, f32), CameraUp, CameraReset, TestTrack, Reverse(bool), Assists(u32), Tuning(i32, f32), Shift(i32), Manual(bool), Clutch(f32), GearCfg(i32, i32), Service, CameraFly(f32, f32), CameraView(i32, f32, f32), Reset, NewLevel(u64), Resize(u32, u32), Pause(bool), Quality(i32), InCity(i32), RenderMode(i32), AutoFallback(bool), GarageCar(i32), Stop }

    struct Engine { tx: Sender<GCmd>, handle: Option<JoinHandle<()>>, stats: Arc<Mutex<[f32; STAT_LEN]>> }

    static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);

    fn send(c: GCmd) {
        if let Some(e) = ENGINE.lock().unwrap().as_ref() { let _ = e.tx.send(c); }
    }

    fn stop_engine() {
        crate::audio::silence();
        let e = ENGINE.lock().unwrap().take();
        if let Some(mut e) = e {
            let _ = e.tx.send(GCmd::Stop);
            if let Some(h) = e.handle.take() { let _ = h.join(); }
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameStart(mut env: JNIEnv, _c: JClass, surface: JObject, w: jint, h: jint, seed: jlong, profile: jni::objects::JString, garage: jboolean) {
        crate::init_logger();
        stop_engine();
        crate::loading::begin();   // before the thread starts: the UI must not read the previous run's 100 %
        let win = unsafe { NativeWindow::from_surface(env.get_raw() as *mut _, surface.as_raw() as *mut _) };
        let Some(win) = win else { log::error!("ANativeWindow_fromSurface failed"); return; };
        let profile_path: String = env.get_string(&profile).map(|s| s.into()).unwrap_or_default();
        let (tx, rx) = channel::<GCmd>();
        let stats = Arc::new(Mutex::new([0.0f32; STAT_LEN]));
        let (st, st2) = (stats.clone(), stats.clone());
        let (w, h) = (w.max(1) as u32, h.max(1) as u32);
        let handle = std::thread::Builder::new().name("game".into()).spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                log::info!("game thread: creating renderer");
                let raster = Raster::new(win, w, h);
                let mut g = Game::new(raster, seed as u64, st);
                crate::loading::stage(crate::loading::CAR, 80);
                if !profile_path.is_empty() { g.with_profile(std::path::PathBuf::from(&profile_path)); }
                if garage != 0 { g.start_garage_selected(); }   // the garage scene
                crate::loading::stage(crate::loading::FIRST_FRAME, 92);
                let mut first = true;
                'run: loop {
                    loop {
                        match rx.try_recv() {
                            Ok(GCmd::Stop) => break 'run,
                            Ok(GCmd::Input(i)) => { let reset = g.input.reset; g.input = i; g.input.reset = reset; }
                            Ok(GCmd::OnRoad(m)) => { g.place_on_road(m); }
                            Ok(GCmd::Dust(l)) => g.set_dust(l),
                            Ok(GCmd::Autopilot(style)) => g.set_autopilot(style),
                            Ok(GCmd::Rally(stage)) => g.rally(stage),
                            Ok(GCmd::Signs(on)) => g.set_signs(on),
                            Ok(GCmd::Camera(y, p)) => g.camera_drag(y, p),
                            Ok(GCmd::CameraUp) => g.camera_release(),
                            Ok(GCmd::CameraReset) => g.camera_reset(),
                            Ok(GCmd::TestTrack) => g.test_track(),
                            Ok(GCmd::Reverse(r)) => g.set_reverse(r),
                            Ok(GCmd::Assists(m)) => g.set_assists(m),
                            Ok(GCmd::Tuning(i, v)) => g.set_tuning(i.max(0) as usize, v),
                            Ok(GCmd::Shift(d)) => g.shift(d),
                            Ok(GCmd::Manual(m)) => g.set_manual(m),
                            Ok(GCmd::Clutch(c)) => g.set_clutch(c),
                            Ok(GCmd::GearCfg(b, p)) => g.set_gear_cfg(b, p),
                            Ok(GCmd::Service) => g.service(),
                            Ok(GCmd::CameraFly(f, u)) => g.camera_fly(f, u),
                            Ok(GCmd::CameraView(v, d, h)) => g.camera_view(v, d, h),
                            Ok(GCmd::Reset) => g.input.reset = true,
                            Ok(GCmd::NewLevel(s)) => g.new_level(s),
                            Ok(GCmd::Resize(w, h)) => g.resize(w, h),
                            Ok(GCmd::Pause(p)) => g.set_paused(p),
                            Ok(GCmd::Quality(q)) => g.set_quality(q),
                            Ok(GCmd::RenderMode(m)) => g.set_hybrid(m == 1),
                            Ok(GCmd::AutoFallback(b)) => g.set_auto_fallback(b),
                            Ok(GCmd::GarageCar(i)) => g.set_garage_car(i),
                            Ok(GCmd::InCity(m)) => { if m == 3 { g.place_near_water(); } else if m == 2 { g.place_in_city(); } else { g.place_near_city(); } }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => break 'run,
                        }
                    }
                    g.frame();
                    if first { first = false; crate::loading::stage(crate::loading::FIRST_FRAME, 100); }
                }
            }));
            *st2.lock().unwrap() = [0.0; STAT_LEN];
            if let Err(e) = res {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                log::error!("game thread panicked, rendering stopped: {msg}");
                crate::loading::set_error(&msg);
                crate::loading::stage(crate::loading::FAILED, 0);
            }
        }).expect("spawn");
        *ENGINE.lock().unwrap() = Some(Engine { tx, handle: Some(handle), stats });
    }

    /// The start-up progress: `stage << 8 | percent` (100 % = the first frame is on the screen).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameLoadProgress(_e: JNIEnv, _c: JClass) -> jint { crate::loading::packed() as jint }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameStop(_e: JNIEnv, _c: JClass) { stop_engine(); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameResize(_e: JNIEnv, _c: JClass, w: jint, h: jint) { send(GCmd::Resize(w.max(1) as u32, h.max(1) as u32)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameInput(_e: JNIEnv, _c: JClass, throttle: jfloat, steer: jfloat, brake: jboolean, handbrake: jboolean, brake_amt: jfloat) {
        send(GCmd::Input(merge_input(throttle, steer, brake != 0, handbrake != 0, brake_amt)));
    }
    /// Debug/test: force the car lights (0 headlights, 1 + brake, 2 + reverse); negative = normal.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameLightsDebug(_e: JNIEnv, _c: JClass, mode: jint) {
        crate::game::carlights::set_debug(if mode >= 0 { Some(mode as u8) } else { None });
    }
    /// Debug/test: the airflow view (0 off, 1 on).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameAirflowDebug(_e: JNIEnv, _c: JClass, on: jint) {
        crate::game::airflow::set_debug(on > 0);
    }
    /// The Settings switch "Aerodynamics" (0 off, 1 on): the dust follows the airflow.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameAero(_e: JNIEnv, _c: JClass, on: jint) {
        crate::game::dust::set_aero(on > 0);
    }
    /// The weather the player asks for (0 auto, 1 clear, 2 cloudy, 3 rain, 4 heavy rain, 5 fog).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameWeather(_e: JNIEnv, _c: JClass, mode: jint) {
        crate::game::weather::set_mode(mode.clamp(0, 255) as u8);
    }
    /// The Settings option "Shadows" (0 high, 1 medium, 2 low, 3 off).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameShadows(_e: JNIEnv, _c: JClass, level: jint) {
        crate::raster::set_shadow_quality(level);
    }
    /// The Settings option "Motion blur" (0 off, 1 low, 2 high).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameMotionBlur(_e: JNIEnv, _c: JClass, level: jint) {
        crate::raster::set_blur(level);
    }
    /// The GPU dust options as bits: 1 particle pressure, 2 the car's exact shape, 4 world collisions, 8 the settled layer.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameDustOpts(_e: JNIEnv, _c: JClass, mask: jint) {
        crate::game::dust::set_opts(mask as u32);
    }
    /// Debug/test: GPU stage timings in the log every 90 frames (0 off, 1 on).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameGpuProf(_e: JNIEnv, _c: JClass, on: jint) {
        crate::raster::set_prof(on > 0);
    }
    /// Debug/test: the dust on the GPU (0 off, 1 on).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameDustGpuDebug(_e: JNIEnv, _c: JClass, on: jint) {
        crate::game::dust::set_gpu_debug(on > 0);
    }
    /// Debug/test: every level starts at this time of day (0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk); negative clears it.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameTimeOfDay(_e: JNIEnv, _c: JClass, phase: jfloat) {
        crate::game::daylight::set_debug_phase(if phase >= 0.0 { Some(phase) } else { None });
    }
    /// Tyre dust: 0 off, 1 normal, 2 lots.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameAutopilot(_e: JNIEnv, _c: JClass, style: jint) { send(GCmd::Autopilot(style)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameAutopilotTune(_e: JNIEnv, _c: JClass, lat: jfloat, brake: jfloat, offset: jfloat, speed: jfloat, slide: jfloat, edge: jfloat) { crate::game::autopilot::cfg::set_custom(lat, brake, offset, speed, slide, edge); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameRally(_e: JNIEnv, _c: JClass, stage: jint) { send(GCmd::Rally(stage)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameNoteText<'a>(mut e: JNIEnv<'a>, _c: JClass, ru: jboolean) -> jni::sys::jstring {
        let t = crate::game::app::NOTE_TEXT.lock().unwrap_or_else(|x| x.into_inner()).clone();
        e.new_string(if ru != 0 { t.1 } else { t.2 }).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameNoteClips<'a>(mut e: JNIEnv<'a>, _c: JClass) -> jni::sys::jstring {
        let t = crate::game::app::NOTE_TEXT.lock().unwrap_or_else(|x| x.into_inner()).clone();
        e.new_string(t.3).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameSigns(_e: JNIEnv, _c: JClass, on: jboolean) { send(GCmd::Signs(on != 0)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameDust(_e: JNIEnv, _c: JClass, level: jint) { send(GCmd::Dust(level)); }
    /// Debug/test: put the car on a road facing along it (1 liaison, 2 stage start, 3 stage middle, 4 stage end).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameOnRoad(_e: JNIEnv, _c: JClass, mode: jint) { send(GCmd::OnRoad(mode)); }
    /// The player drags on the screen: orbit the chase camera by (yaw, pitch) radians.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameCamera(_e: JNIEnv, _c: JClass, dyaw: jfloat, dpitch: jfloat) { send(GCmd::Camera(dyaw, dpitch)); }
    /// The free camera: fly `fwd` metres along the gaze and `up` metres up.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameCameraFly(_e: JNIEnv, _c: JClass, fwd: jfloat, up: jfloat) { send(GCmd::CameraFly(fwd, up)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameCameraUp(_e: JNIEnv, _c: JClass) { send(GCmd::CameraUp); }
    /// The manual transmission: a shift (+1 up, -1 down), the mode (0 automatic, 1 manual) and the clutch pedal (0 up .. 1 down).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameShift(_e: JNIEnv, _c: JClass, dir: jint) { send(GCmd::Shift(dir)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameGearMode(_e: JNIEnv, _c: JClass, manual: jint) { send(GCmd::Manual(manual != 0)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameService(_e: JNIEnv, _c: JClass) { send(GCmd::Service); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameGearCfg(_e: JNIEnv, _c: JClass, boxt: jint, profile: jint) { send(GCmd::GearCfg(boxt, profile)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameClutch(_e: JNIEnv, _c: JClass, pedal: jfloat) { send(GCmd::Clutch(pedal)); }
    /// The aids as bits (anti-lag, ABS, traction control, the blip, the flat shift, the limiter, the blow-off valve) and one part of the tuning (0 engine, 1 turbo, 2 brakes, 3 tyres, 4 suspension, 5 downforce, 6 gearing) as a multiplier.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameAssists(_e: JNIEnv, _c: JClass, mask: jint) { send(GCmd::Assists(mask as u32)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameTuning(_e: JNIEnv, _c: JClass, part: jint, v: jfloat) { send(GCmd::Tuning(part, v)); }
    /// The automatic's selector: 0 drive, 1 reverse.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameReverse(_e: JNIEnv, _c: JClass, on: jint) { send(GCmd::Reverse(on != 0)); }
    /// The message of the panic that stopped the game thread (empty when it did not).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameLastError<'a>(mut e: JNIEnv<'a>, _c: JClass) -> jni::objects::JString<'a> { e.new_string(crate::loading::last_error()).unwrap_or_default() }
    /// The test track: a flat sheet of tarmac.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameTestTrack(_e: JNIEnv, _c: JClass) { send(GCmd::TestTrack); }
    /// A double tap: the camera goes back to its default.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameCameraReset(_e: JNIEnv, _c: JClass) { send(GCmd::CameraReset); }
    /// The camera view: 0 chase, 1 bonnet, 2 bumper, 3 roof, 4 orbit, with its distance scale (zoom) and height offset (tilt).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameCameraView(_e: JNIEnv, _c: JClass, view: jint, dist: jfloat, height: jfloat) { send(GCmd::CameraView(view, dist, height)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameInCity(_e: JNIEnv, _c: JClass, mode: jint) { send(GCmd::InCity(mode)); }
    /// The app's cache folder, for the compiled pipelines (call before `gameStart`).
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameCacheDir(mut env: JNIEnv, _c: JClass, dir: jni::objects::JString) {
        let d: String = env.get_string(&dir).map(|s| s.into()).unwrap_or_default();
        crate::raster::set_cache_dir(&d);
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameQuality(_e: JNIEnv, _c: JClass, level: jint) { send(GCmd::Quality(level)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameRenderMode(_e: JNIEnv, _c: JClass, m: jint) { send(GCmd::RenderMode(m)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameAutoFallback(_e: JNIEnv, _c: JClass, on: jboolean) { send(GCmd::AutoFallback(on != 0)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameGarageCar(_e: JNIEnv, _c: JClass, i: jint) { send(GCmd::GarageCar(i)); }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_garageInfo(mut env: JNIEnv, _c: JClass, path: jni::objects::JString) -> jfloatArray {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        let v = crate::game::api::garage_info(std::path::Path::new(&p));
        let arr = env.new_float_array(v.len() as i32).unwrap();
        env.set_float_array_region(&arr, 0, &v).unwrap();
        arr.into_raw()
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_garageBuyCar(mut env: JNIEnv, _c: JClass, path: jni::objects::JString, i: jint) -> jboolean {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        crate::game::api::garage_buy_car(std::path::Path::new(&p), i.max(0) as usize) as jboolean
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_garageSelect(mut env: JNIEnv, _c: JClass, path: jni::objects::JString, i: jint) -> jboolean {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        crate::game::api::garage_select(std::path::Path::new(&p), i.max(0) as usize) as jboolean
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_garageWingAngle(mut env: JNIEnv, _c: JClass, path: jni::objects::JString, i: jint, angle: jint) -> jboolean {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        crate::game::api::garage_set_wing_angle(std::path::Path::new(&p), i.max(0) as usize, angle.max(0) as usize) as jboolean
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_garageWheelSet(mut env: JNIEnv, _c: JClass, path: jni::objects::JString, i: jint, set: jint) -> jboolean {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        crate::game::api::garage_set_wheel_set(std::path::Path::new(&p), i.max(0) as usize, set.max(0) as usize) as jboolean
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_garageBuyUpgrade(mut env: JNIEnv, _c: JClass, path: jni::objects::JString, i: jint, up: jint) -> jboolean {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        crate::game::api::garage_buy_upgrade(std::path::Path::new(&p), i.max(0) as usize, up.max(0) as usize) as jboolean
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gamePause(_e: JNIEnv, _c: JClass, p: jboolean) { send(GCmd::Pause(p != 0)); }
    /// Audio thread: fills `buf` (PCM16 interleaved stereo, 44.1 kHz) with the next samples. Never panics.
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_audioRender(env: JNIEnv, _c: JClass, buf: jni::objects::JShortArray) {
        let n = env.get_array_length(&buf).unwrap_or(0).max(0) as usize;
        if n == 0 { return; }
        let mut v = vec![0i16; n];
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::audio::render_into(&mut v)));
        let _ = env.set_short_array_region(&buf, 0, &v);
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_audioVolume(_e: JNIEnv, _c: JClass, v: jfloat) { crate::audio::set_volume(v); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_audioMute(_e: JNIEnv, _c: JClass, m: jboolean) { crate::audio::set_muted(m != 0); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameReset(_e: JNIEnv, _c: JClass) { send(GCmd::Reset); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameNewLevel(_e: JNIEnv, _c: JClass, seed: jlong) { send(GCmd::NewLevel(seed as u64)); }

    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameStats(env: JNIEnv, _c: JClass) -> jfloatArray {
        let s = ENGINE.lock().unwrap().as_ref().map(|e| *e.stats.lock().unwrap()).unwrap_or([0.0; STAT_LEN]);
        let arr = env.new_float_array(STAT_LEN as i32).unwrap();
        env.set_float_array_region(&arr, 0, &s).unwrap();
        arr.into_raw()
    }
}
