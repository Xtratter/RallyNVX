package dev.starinin.rallynvx

/** The handbrake button: a tap latches the handbrake on and the next one releases it; the label shows the state with a dot, like the autopilot's. */
object HandbrakeMath {
    fun label(on: Boolean, en: Boolean): String = (if (on) "● " else "") + (if (en) "Handbrake" else "Ручник")
}
