package dev.starinin.rallynvx

/**
 * Camera views and the two-finger gestures (the view table mirrors `CamView` in rust/src/game/camera_follow.rs): the names, the limits, and how a pinch and a
 * vertical two-finger move change the distance scale (zoom in the bonnet and bumper views) and the height offset (gaze tilt there).
 */
object CamMath {
    const val VIEWS = 5
    const val DIST_MIN = 0.5f
    const val DIST_MAX = 2.5f
    private val EN = arrayOf("Behind", "Bonnet", "Bumper", "Roof", "Orbit")
    private val RU = arrayOf("Сзади", "Капот", "Бампер", "Сверху", "Обзор")

    fun name(view: Int, en: Boolean) = (if (en) EN else RU)[view.coerceIn(0, VIEWS - 1)]
    fun next(view: Int) = (view.coerceIn(0, VIEWS - 1) + 1) % VIEWS
    /** The bonnet (1) and bumper (2) views ride on the car: their "height" is a gaze tilt and their "distance" a zoom. */
    fun rigid(view: Int) = view == 1 || view == 2

    /** The finger's camera drag can be inverted: bit 1 = horizontally, bit 2 = vertically (0 none, 3 both). */
    const val INVERT_MODES = 4
    fun invertName(mode: Int, en: Boolean) = (if (en) arrayOf("None", "Horizontal", "Vertical", "Both") else arrayOf("Нет", "По горизонтали", "По вертикали", "Обе"))[mode.coerceIn(0, INVERT_MODES - 1)]
    /** The signs (horizontal, vertical) that multiply the finger's drag for an inversion mode. */
    fun dragSigns(mode: Int): Pair<Float, Float> = Pair(if (mode and 1 != 0) -1f else 1f, if (mode and 2 != 0) -1f else 1f)

    fun heightMin(view: Int) = if (rigid(view)) -1f else -3f
    fun heightMax(view: Int) = if (rigid(view)) 1f else 6f

    fun clampDist(d: Float) = if (d.isFinite()) d.coerceIn(DIST_MIN, DIST_MAX) else 1f
    fun clampHeight(view: Int, h: Float) = if (h.isFinite()) h.coerceIn(heightMin(view), heightMax(view)) else 0f

    /** The fingers moved apart (ratio > 1) or together (< 1). */
    fun pinchFor(view: Int, dist: Float, ratio: Float): Float {
        if (!ratio.isFinite() || ratio <= 0f) return clampDist(dist)
        // outside views: the fingers moving together (ratio < 1) pull the camera away, apart bring it closer; the rigid views zoom in when the fingers spread
        return clampDist(if (rigid(view)) dist * ratio else dist / ratio)
    }

    /** Both fingers moved up by `dy` pixels (negative = up on the screen): the camera rises, or the gaze tilts up. */
    fun height(view: Int, h: Float, dyPx: Float, density: Float): Float {
        if (!dyPx.isFinite()) return clampHeight(view, h)
        val perPx = if (rigid(view)) 0.004f else 0.035f   // gaze units / metres per pixel at 2.6 density
        return clampHeight(view, h - dyPx * perPx * (2.6f / density.coerceAtLeast(0.5f)))
    }
}
