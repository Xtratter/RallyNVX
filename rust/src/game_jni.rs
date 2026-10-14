//! JNI entry points of the game (dev.starinin.raytrace.Native.game*) and its render thread.
use crate::game::{flight::FlightInput, input::Input};

/// Commands from Kotlin as an `Input` (non-finite axes become 0, axes are clamped).
pub fn merge_input(throttle: f32, steer: f32, brake: bool, handbrake: bool, brake_amt: f32) -> Input {
    Input { throttle, steer, brake, handbrake, brake_amt, reset: false }.sanitized()
}

/// Flight sticks from Kotlin as a `FlightInput` (non-finite axes become 0, axes are clamped).
pub fn merge_flight(yaw: f32, lift: f32, pitch: f32, roll: f32) -> FlightInput { FlightInput { yaw, lift, pitch, roll }.sanitized() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn flight_input_merge_sanitizes() {
        let f = merge_flight(f32::NAN, 3.0, -3.0, 0.5);
        assert_eq!((f.yaw, f.lift, f.pitch, f.roll), (0.0, 1.0, -1.0, 0.5));
    }
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

    use super::{merge_flight, merge_input};
    use crate::game::{app::{Game, STAT_LEN}, flight::FlightInput, input::Input};
    use crate::raster::Raster;

    enum GCmd { Input(Input), Flight(FlightInput), Reset, NewLevel(u64), Resize(u32, u32), Pause(bool), Restart, Abort, Quality(i32), InCity(i32), RenderMode(i32), Stop }

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
    pub extern "system" fn Java_dev_starinin_raytrace_Native_gameStart(mut env: JNIEnv, _c: JClass, surface: JObject, w: jint, h: jint, seed: jlong, profile: jni::objects::JString, mission: jint) {
        crate::init_logger();
        stop_engine();
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
                if !profile_path.is_empty() {
                    g.with_profile(std::path::PathBuf::from(&profile_path), seed as u64);
                    if mission >= 0 { g.start_mission(mission as usize); }
                }
                'run: loop {
                    loop {
                        match rx.try_recv() {
                            Ok(GCmd::Stop) => break 'run,
                            Ok(GCmd::Input(i)) => { let reset = g.input.reset; g.input = i; g.input.reset = reset; }
                            Ok(GCmd::Flight(f)) => g.flight = f,
                            Ok(GCmd::Reset) => g.input.reset = true,
                            Ok(GCmd::NewLevel(s)) => g.new_level(s),
                            Ok(GCmd::Resize(w, h)) => g.resize(w, h),
                            Ok(GCmd::Pause(p)) => g.set_paused(p),
                            Ok(GCmd::Quality(q)) => g.set_quality(q),
                            Ok(GCmd::RenderMode(m)) => g.set_hybrid(m == 1),
                            Ok(GCmd::InCity(m)) => { if m == 3 { g.place_near_water(); } else if m == 2 { g.place_in_city(); } else { g.place_near_city(); } }
                            Ok(GCmd::Restart) => g.restart_mission(),
                            Ok(GCmd::Abort) => g.abort_mission(),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => break 'run,
                        }
                    }
                    g.frame();
                }
            }));
            *st2.lock().unwrap() = [0.0; STAT_LEN];
            if let Err(e) = res {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                log::error!("game thread panicked, rendering stopped: {msg}");
            }
        }).expect("spawn");
        *ENGINE.lock().unwrap() = Some(Engine { tx, handle: Some(handle), stats });
    }

    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameStop(_e: JNIEnv, _c: JClass) { stop_engine(); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameResize(_e: JNIEnv, _c: JClass, w: jint, h: jint) { send(GCmd::Resize(w.max(1) as u32, h.max(1) as u32)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameInput(_e: JNIEnv, _c: JClass, throttle: jfloat, steer: jfloat, brake: jboolean, handbrake: jboolean, brake_amt: jfloat) {
        send(GCmd::Input(merge_input(throttle, steer, brake != 0, handbrake != 0, brake_amt)));
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameFlight(_e: JNIEnv, _c: JClass, yaw: jfloat, lift: jfloat, pitch: jfloat, roll: jfloat) {
        send(GCmd::Flight(merge_flight(yaw, lift, pitch, roll)));
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameInCity(_e: JNIEnv, _c: JClass, mode: jint) { send(GCmd::InCity(mode)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameQuality(_e: JNIEnv, _c: JClass, level: jint) { send(GCmd::Quality(level)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameRenderMode(_e: JNIEnv, _c: JClass, m: jint) { send(GCmd::RenderMode(m)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gamePause(_e: JNIEnv, _c: JClass, p: jboolean) { send(GCmd::Pause(p != 0)); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameRestart(_e: JNIEnv, _c: JClass) { send(GCmd::Restart); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameAbort(_e: JNIEnv, _c: JClass) { send(GCmd::Abort); }
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_raytrace_Native_missionsInfo(mut env: JNIEnv, _c: JClass, path: jni::objects::JString, fresh: jlong, refresh: jboolean) -> jfloatArray {
        let p: String = env.get_string(&path).map(|s| s.into()).unwrap_or_default();
        let v = crate::game::api::menu_info(std::path::Path::new(&p), fresh as u64, refresh != 0);
        let arr = env.new_float_array(v.len() as i32).unwrap();
        env.set_float_array_region(&arr, 0, &v).unwrap();
        arr.into_raw()
    }
    /// Audio thread: fills `buf` (PCM16 mono, 44.1 kHz) with the next samples. Never panics.
    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_raytrace_Native_audioRender(env: JNIEnv, _c: JClass, buf: jni::objects::JShortArray) {
        let n = env.get_array_length(&buf).unwrap_or(0).max(0) as usize;
        if n == 0 { return; }
        let mut v = vec![0i16; n];
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::audio::render_into(&mut v)));
        let _ = env.set_short_array_region(&buf, 0, &v);
    }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_audioVolume(_e: JNIEnv, _c: JClass, v: jfloat) { crate::audio::set_volume(v); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_audioMute(_e: JNIEnv, _c: JClass, m: jboolean) { crate::audio::set_muted(m != 0); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameReset(_e: JNIEnv, _c: JClass) { send(GCmd::Reset); }
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameNewLevel(_e: JNIEnv, _c: JClass, seed: jlong) { send(GCmd::NewLevel(seed as u64)); }

    #[no_mangle]
    pub extern "system" fn Java_dev_starinin_raytrace_Native_gameStats(env: JNIEnv, _c: JClass) -> jfloatArray {
        let s = ENGINE.lock().unwrap().as_ref().map(|e| *e.stats.lock().unwrap()).unwrap_or([0.0; STAT_LEN]);
        let arr = env.new_float_array(STAT_LEN as i32).unwrap();
        env.set_float_array_region(&arr, 0, &s).unwrap();
        arr.into_raw()
    }
}
