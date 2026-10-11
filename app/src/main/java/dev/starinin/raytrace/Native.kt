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
    @JvmStatic external fun gameStart(surface: Surface, w: Int, h: Int, seed: Long)
    @JvmStatic external fun gameStop()
    @JvmStatic external fun gameResize(w: Int, h: Int)
    @JvmStatic external fun gameInput(throttle: Float, steer: Float, brake: Boolean, handbrake: Boolean)
    @JvmStatic external fun gameReset()
    @JvmStatic external fun gameNewLevel(seed: Long)
    /** 16 floats: [0] fps, [1] km/h, [2] seed (low 24 bits), [3] frame ms. */
    @JvmStatic external fun gameStats(): FloatArray
    /** 16 floats: [0..5] fps, gpu ms, w, h, scale, path; [8..15] per-pass GPU ms (trace, gi_trace, gi_temporal, gi_atrous, temporal, atrous, composite, present). */
    @JvmStatic external fun stats(): FloatArray
}
