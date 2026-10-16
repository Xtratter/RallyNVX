package dev.starinin.rallynvx

import android.view.Surface

object Native {
    init { System.loadLibrary("rallynvx") }
    @JvmStatic external fun start(surface: Surface, w: Int, h: Int)
    @JvmStatic external fun stop()
    @JvmStatic external fun resize(w: Int, h: Int)
    @JvmStatic external fun setParam(id: Int, v: Float)
    @JvmStatic external fun orbit(dx: Float, dy: Float)
    @JvmStatic external fun zoom(f: Float)
    @JvmStatic external fun sticks(lx: Float, ly: Float, rx: Float, ry: Float)
    @JvmStatic external fun resetCamera()
    /** Parses a scene file (JSON) and switches to it; returns "" on success or a readable error. */
    @JvmStatic external fun loadScene(json: String): String
    /** profile: path of profile.json ("" = none: the default car); garage = the turntable garage scene instead of the free-drive world. */
    @JvmStatic external fun gameStart(surface: Surface, w: Int, h: Int, seed: Long, profile: String, garage: Boolean)
    /** City draw distance by quality: 0 Low (300 m), 1 Medium (400 m), 2 High (450 m). */
    @JvmStatic external fun gameQuality(level: Int)
    /** Garage: shows car [index] (vehicles file order) in the garage scene (start the engine with garage = true). */
    @JvmStatic external fun gameGarageCar(index: Int)
    /** `[money, selected_index, car_count]` then per car `[owned, price, upgrade_cost, engine, tyres, suspension, brakes]`. */
    @JvmStatic external fun garageInfo(path: String): FloatArray
    @JvmStatic external fun garageBuyCar(path: String, index: Int): Boolean
    @JvmStatic external fun garageSelect(path: String, index: Int): Boolean
    /** upgrade 0 engine, 1 tyres, 2 suspension, 3 brakes */
    @JvmStatic external fun garageBuyUpgrade(path: String, index: Int, upgrade: Int): Boolean
    /** Sets the wing angle of car [index]: 0 low drag, 1 mid, 2 high downforce. */
    @JvmStatic external fun garageWingAngle(path: String, index: Int, angle: Int): Boolean
    /** Sets the wheel set of car [index]: 0 tarmac, 1 gravel, 2 snow (the look and the grip by surface). */
    @JvmStatic external fun garageWheelSet(path: String, index: Int, set: Int): Boolean
    /** 0 raster, 1 hybrid (PCSS soft shadows, terrain ray shadows, ambient occlusion). */
    @JvmStatic external fun gameRenderMode(mode: Int)
    /** With it on, the game drops from hybrid to raster after four slow seconds (see game::rendermode). */
    @JvmStatic external fun gameAutoFallback(on: Boolean)
    /** Debug placement: 1 = next to the first city, 2 = at its centre. */
    /** The player drags on the screen: orbit the chase camera by (yaw, pitch) radians; [gameCameraUp] when the finger goes up. */
    @JvmStatic external fun gameCamera(dyaw: Float, dpitch: Float)
    /** The free camera: fly `fwd` metres along the gaze and `up` metres up. */
    @JvmStatic external fun gameCameraFly(fwd: Float, up: Float)
    @JvmStatic external fun gameCameraUp()
    /** The camera view (0 behind, 1 bonnet, 2 bumper, 3 roof, 4 orbit), its distance scale or zoom and its height offset or tilt. */
    @JvmStatic external fun gameCameraView(view: Int, dist: Float, height: Float)
    /** Debug/test: put the car on a road facing along it (1 liaison, 2 stage start, 3 stage middle, 4 stage end). */
    @JvmStatic external fun gameOnRoad(mode: Int)
    /** Tyre dust: 0 off, 1 normal, 2 lots. */
    @JvmStatic external fun gameDust(level: Int)
    @JvmStatic external fun gameAutopilot(style: Int)
    @JvmStatic external fun gameRally(stage: Int)
    @JvmStatic external fun gameSigns(on: Boolean)
    @JvmStatic external fun gameNoteText(ru: Boolean): String
    /** The clip ids of the last pace note, comma separated (see tools/voices/clips.json). */
    @JvmStatic external fun gameNoteClips(): String
    @JvmStatic external fun gameAutopilotTune(lat: Float, brake: Float, offset: Float, speed: Float, slide: Float, edge: Float)
    @JvmStatic external fun gameInCity(mode: Int)
    /** Debug/test: start every level at this time of day (0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk); negative = off. */
    @JvmStatic external fun gameTimeOfDay(phase: Float)
    /** Debug/test: force the car lights (0 headlights, 1 + brake, 2 + reverse); negative = normal. */
    @JvmStatic external fun gameLightsDebug(mode: Int)
    @JvmStatic external fun gameAirflowDebug(on: Int)
    @JvmStatic external fun gameDustGpuDebug(on: Int)
    @JvmStatic external fun gameAero(on: Int)
    /** The weather: 0 auto (the level's seed), 1 clear, 2 cloudy, 3 rain, 4 heavy rain, 5 fog; it moves smoothly and the ground dries slowly. */
    @JvmStatic external fun gameWeather(mode: Int)
    /** Motion blur: 0 off, 1 low, 2 high. */
    @JvmStatic external fun gameMotionBlur(level: Int)
    /** The shadow quality: 0 high (soft, blocker search), 1 medium, 2 low, 3 off. */
    @JvmStatic external fun gameShadows(level: Int)
    /** The GPU dust options as bits: 1 particle pressure, 2 the car's exact shape, 4 world collisions, 8 the settled layer. */
    @JvmStatic external fun gameDustOpts(mask: Int)
    @JvmStatic external fun gameGpuProf(on: Int)
    @JvmStatic external fun gamePause(p: Boolean)
    @JvmStatic external fun gameStop()
    @JvmStatic external fun gameResize(w: Int, h: Int)
    @JvmStatic external fun gameInput(throttle: Float, steer: Float, brake: Boolean, handbrake: Boolean, brakeAmt: Float)
    /** Fills [buf] with PCM16 mono 44.1 kHz samples (procedural sound, see audio::). */
    @JvmStatic external fun audioRender(buf: ShortArray)
    @JvmStatic external fun audioMute(muted: Boolean)
    /** Master volume 0..1. */
    @JvmStatic external fun audioVolume(v: Float)
    @JvmStatic external fun gameReset()
    @JvmStatic external fun gameNewLevel(seed: Long)
    /** 64 floats, layout in game::app::fill_state: [0] fps, [1] km/h, [2] seed, [4] phase, [5] time, [6] limit, [7] integrity, [8] distance, [9] bearing, [10]/[11] checkpoint, [12] result, [13] medal, [14] payout, [15] failure, [16] money, [17] kind 0 cargo/1 race, [25] signed speed, [26] hybrid, [27] auto fallback; [19]..[24] unused. */
    @JvmStatic external fun gameStats(): FloatArray
    @JvmStatic external fun gameLoadProgress(): Int   // stage << 8 | percent of the start-up
    /** 16 floats: [0..5] fps, gpu ms, w, h, scale, path; [8..15] per-pass GPU ms (trace, gi_trace, gi_temporal, gi_atrous, temporal, atrous, composite, present). */
    @JvmStatic external fun stats(): FloatArray
}
