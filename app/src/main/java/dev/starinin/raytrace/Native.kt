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
    /** profile: path of profile.json ("" = none); mission: index in the profile's current list, -1 = free drive. */
    @JvmStatic external fun gameStart(surface: Surface, w: Int, h: Int, seed: Long, profile: String, mission: Int)
    /** Aircraft sticks (air missions): yaw/roll right-positive, lift up-positive, pitch forward-positive; all -1..1. */
    @JvmStatic external fun gameFlight(yaw: Float, lift: Float, pitch: Float, roll: Float)
    /** City draw distance by quality: 0 Low (300 m), 1 Medium (400 m), 2 High (450 m). */
    @JvmStatic external fun gameQuality(level: Int)
    /** Debug placement: 1 = next to the first city, 2 = at its centre (an aircraft 120 m above it). */
    @JvmStatic external fun gameInCity(mode: Int)
    @JvmStatic external fun gamePause(p: Boolean)
    @JvmStatic external fun gameRestart()
    @JvmStatic external fun gameAbort()
    /** [money, round, profile_seed, golds] + 6 x [kind(0 cargo/1 race/2 rings/3 sling/4 recon), seed, difficulty, length_m, time_limit, reward_base, best_medal, best_time, aircraft(0 none/1 heli/2 drone), urban(0/1)]. */
    @JvmStatic external fun missionsInfo(path: String, fresh: Long, refresh: Boolean): FloatArray
    @JvmStatic external fun gameStop()
    @JvmStatic external fun gameResize(w: Int, h: Int)
    @JvmStatic external fun gameInput(throttle: Float, steer: Float, brake: Boolean, handbrake: Boolean)
    /** Fills [buf] with PCM16 mono 44.1 kHz samples (procedural sound, see audio::). */
    @JvmStatic external fun audioRender(buf: ShortArray)
    @JvmStatic external fun audioMute(muted: Boolean)
    @JvmStatic external fun gameReset()
    @JvmStatic external fun gameNewLevel(seed: Long)
    /** 64 floats, layout in game::app::fill_state: [0] fps, [1] km/h, [2] seed, [4] phase, [5] time, [6] limit, [7] integrity, [8] distance, [9] bearing, [10]/[11] checkpoint, [12] result, [13] medal, [14] payout, [15] failure, [16] money, [17] kind 0..5, [19] altitude, [20] aircraft 0/1/2, [21] rope taut, [22] target height above the aircraft, [23] rescue: people aboard, [24] rescue: boarding progress 0..1; failure [15] 4 = crashed. */
    @JvmStatic external fun gameStats(): FloatArray
    /** 16 floats: [0..5] fps, gpu ms, w, h, scale, path; [8..15] per-pass GPU ms (trace, gi_trace, gi_temporal, gi_atrous, temporal, atrous, composite, present). */
    @JvmStatic external fun stats(): FloatArray
}
