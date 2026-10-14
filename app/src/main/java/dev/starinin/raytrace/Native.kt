package dev.starinin.raytrace

import android.view.Surface

object Native {
    init { System.loadLibrary("raytrace") }
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
    /** 0 raster, 1 hybrid (PCSS soft shadows, terrain ray shadows, ambient occlusion). */
    @JvmStatic external fun gameRenderMode(mode: Int)
    /** With it on, the game drops from hybrid to raster after four slow seconds (see game::rendermode). */
    @JvmStatic external fun gameAutoFallback(on: Boolean)
    /** Debug placement: 1 = next to the first city, 2 = at its centre. */
    /** The player drags on the screen: orbit the chase camera by (yaw, pitch) radians; [gameCameraUp] when the finger goes up. */
    @JvmStatic external fun gameCamera(dyaw: Float, dpitch: Float)
    @JvmStatic external fun gameCameraUp()
    /** Debug/test: put the car on a road facing along it (1 liaison, 2 stage start, 3 stage middle, 4 stage end). */
    @JvmStatic external fun gameOnRoad(mode: Int)
    @JvmStatic external fun gameInCity(mode: Int)
    /** Debug/test: start every level at this time of day (0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk); negative = off. */
    @JvmStatic external fun gameTimeOfDay(phase: Float)
    /** Debug/test: force the car lights (0 headlights, 1 + brake, 2 + reverse); negative = normal. */
    @JvmStatic external fun gameLightsDebug(mode: Int)
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
    /** 16 floats: [0..5] fps, gpu ms, w, h, scale, path; [8..15] per-pass GPU ms (trace, gi_trace, gi_temporal, gi_atrous, temporal, atrous, composite, present). */
    @JvmStatic external fun stats(): FloatArray
}
