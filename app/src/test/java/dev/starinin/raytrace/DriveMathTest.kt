package dev.starinin.raytrace

import org.junit.Assert.*
import org.junit.Test
import kotlin.math.abs

class DriveMathTest {
    private val eps = 1e-3f

    @Test fun wheelAngleQuadrants() {
        assertEquals(0f, DriveMath.wheelAngleDeg(110f, 100f, 100f, 100f), eps)     // right
        assertEquals(90f, DriveMath.wheelAngleDeg(100f, 90f, 100f, 100f), eps)     // above (screen y smaller)
        assertEquals(180f, abs(DriveMath.wheelAngleDeg(90f, 100f, 100f, 100f)), eps)
        assertEquals(-90f, DriveMath.wheelAngleDeg(100f, 110f, 100f, 100f), eps)   // below
    }

    @Test fun wheelFollowsAClockwiseDragAndLimits() {
        val w = DriveMath.Wheel()
        w.down(90f)                      // finger on top
        w.move(60f); w.move(30f)         // moving clockwise on screen (angle with y up decreases)
        assertEquals(60f, w.angle, eps)
        assertTrue(w.steer(1) > 0f)
        w.move(-100f)                    // far beyond the lock
        assertEquals(DriveMath.WHEEL_MAX, w.angle, eps)
        w.move(30f); w.move(150f)        // counter-clockwise past the limit the other way
        assertTrue(w.angle < DriveMath.WHEEL_MAX)
    }

    @Test fun wheelDoesNotJumpAcrossTheSeam() {
        val w = DriveMath.Wheel()
        w.down(170f); w.move(-170f)      // 20 degrees counter-clockwise across +-180, not 340 the other way
        assertEquals(-20f, w.angle, eps)
        w.move(170f)                     // and back
        assertEquals(0f, w.angle, eps)
    }

    @Test fun wheelReturnsToCentreAtItsOwnPace() {
        val w = DriveMath.Wheel()
        w.down(90f); w.move(0f); w.move(-60f)       // 150 clockwise, clamped to 120
        assertEquals(120f, w.angle, eps)
        w.step(1f)                                   // still held: no return
        assertEquals(120f, w.angle, eps)
        w.up()
        w.step(0.1f); assertEquals(80f, w.angle, eps)
        w.step(1f); assertEquals(0f, w.angle, 0f)    // never overshoots
        w.step(0.5f); assertEquals(0f, w.angle, 0f)
    }

    @Test fun steeringCurveIsOddMonotonicAndSensitivityScales() {
        fun at(deg: Float, s: Int) = DriveMath.Wheel().also { it.down(0f); it.move(-deg) }.steer(s)
        assertEquals(0f, at(0f, 1), 0f)
        assertEquals(1f, at(120f, 1), eps)
        assertEquals(-at(60f, 1), DriveMath.Wheel().also { it.down(0f); it.move(60f) }.steer(1), eps)
        var prev = 0f
        for (d in 10..120 step 10) { val v = at(d.toFloat(), 1); assertTrue(v > prev); prev = v }
        assertEquals(0.7f, at(120f, 0), eps)
        assertEquals(1f, at(96f, 2), eps)            // saturates before full lock
        assertTrue(at(30f, 2) > at(30f, 1) && at(30f, 1) > at(30f, 0))
        assertEquals(0f, DriveMath.Wheel().steer(99), 0f)   // bad sensitivity index is clamped, never crashes
    }

    @Test fun pedalPressureByTouchHeight() {
        assertEquals(0f, DriveMath.pedal(-1f, 100f), 0f)
        assertEquals(0f, DriveMath.pedal(101f, 100f), 0f)
        assertEquals(0.15f, DriveMath.pedal(0f, 100f), eps)
        assertEquals(1f, DriveMath.pedal(100f, 100f), eps)
        assertTrue(DriveMath.pedal(50f, 100f) in 0.5f..0.6f)
        assertEquals(0f, DriveMath.pedal(5f, 0f), 0f)
        assertEquals(0f, DriveMath.pedal(Float.NaN, 100f), 0f)
    }

    @Test fun brakePedalBrakesWhileMovingAndReversesWhenStopped() {
        val moving = DriveMath.brakeMap(0.6f, 20f)
        assertTrue(moving.brake); assertEquals(0.6f, moving.amt, eps); assertEquals(0f, moving.throttle, 0f)
        val stopped = DriveMath.brakeMap(0.6f, 1f)
        assertFalse(stopped.brake); assertEquals(-0.6f, stopped.throttle, eps)
        assertFalse(DriveMath.brakeMap(0.6f, 3f).brake)         // exactly the threshold reverses
        assertTrue(DriveMath.brakeMap(0.6f, 3.01f).brake)
        assertEquals(DriveMath.BrakeCmd(false, 0f, 0f), DriveMath.brakeMap(0f, 50f))
        assertEquals(DriveMath.BrakeCmd(false, 0f, 0f), DriveMath.brakeMap(Float.NaN, 50f))
    }

    @Test fun tiltDeadZoneScaleCalibrationAndSmoothing() {
        fun settle(t: DriveMath.Tilt, roll: Float, s: Int = 1): Float { var v = 0f; repeat(120) { v = t.update(roll, 1f / 60f, s) }; return v }
        val t = DriveMath.Tilt()
        assertEquals(0f, settle(t, 2f), 0f)                      // inside the dead zone
        assertTrue(settle(t, 15f) in 0.35f..0.55f)
        assertEquals(1f, settle(t, 40f), 0f)
        assertEquals(-1f, settle(t, -40f), 0f)
        t.calibrate(20f)
        assertEquals(0f, settle(t, 20f), 0f)                     // the new pose is the centre
        assertTrue(settle(t, 35f) > 0.3f)
        assertTrue(settle(t, 15f) < 0f)
        val c = DriveMath.Tilt(); var prev = 0f
        repeat(60) { val v = c.update(25f, 1f / 60f, 1); assertTrue(v >= prev - 1e-6f && v <= 1f); prev = v }
        assertEquals(0f, DriveMath.Tilt().update(Float.NaN, 0.016f, 1), 0f)
        assertEquals(0f, DriveMath.Tilt().update(10f, Float.NaN, 1), 0f)
    }

    @Test fun preferenceStepsCycle() {
        assertEquals(1, DriveMath.cycle(0, 3)); assertEquals(0, DriveMath.cycle(2, 3)); assertEquals(0, DriveMath.cycle(4, 5))
        assertEquals(5, DriveMath.VOLUME.size)
        assertEquals(0f, DriveMath.VOLUME.first(), 0f); assertEquals(1f, DriveMath.VOLUME.last(), 0f)
    }
}

class DriveMathReviewTest {
    @Test fun reversingContinuesWhenTheSignedSpeedIsNegative() {
        val c = DriveMath.brakeMap(0.8f, -12f)        // rolling backwards fast: the pedal keeps reversing, it never brakes
        assertFalse(c.brake); assertEquals(-0.8f, c.throttle, 1e-3f)
    }
    @Test fun tiltResetForgetsTheCentreAndTheSmoothing() {
        val t = DriveMath.Tilt()
        t.calibrate(25f); repeat(120) { t.update(60f, 1f / 60f, 1) }
        t.reset()
        assertEquals(0f, t.update(0f, 1f / 60f, 1), 0f)
        repeat(120) { t.update(0f, 1f / 60f, 1) }
        assertEquals(0f, t.update(0f, 1f / 60f, 1), 0f)     // the old centre is gone: 0 degrees is the centre again
    }
}

class DriveMathButtonsTest {
    @Test fun buttonsRampToFullLockAndSpringBack() {
        val b = DriveMath.Buttons()
        assertEquals(0f, b.step(0.1f, false, false, 1), 0f)
        assertEquals(0.4f, b.step(0.1f, false, true, 1), 1e-4f)        // 4 per second at normal sensitivity
        repeat(10) { b.step(0.1f, false, true, 1) }
        assertEquals(1f, b.value, 0f)                                  // never beyond full lock
        b.step(0.05f, false, false, 1); assertEquals(0.6f, b.value, 1e-4f)
        b.step(1f, false, false, 1); assertEquals(0f, b.value, 0f)     // returns to centre without overshoot
    }
    @Test fun leftIsNegativeBothCancelAndSensitivityScalesTheRamp() {
        val b = DriveMath.Buttons()
        b.step(0.1f, true, false, 1); assertTrue(b.value < 0f)
        b.step(1f, true, true, 1); assertEquals(0f, b.value, 0f)       // both pressed: centre
        val slow = DriveMath.Buttons().also { it.step(0.1f, false, true, 0) }.value
        val fast = DriveMath.Buttons().also { it.step(0.1f, false, true, 2) }.value
        assertTrue(slow < 0.4f && fast > 0.4f)
        val r = DriveMath.Buttons(); r.step(0.2f, false, true, 1); r.step(0.1f, true, false, 1)   // reversing direction passes through the centre
        assertTrue(r.value < 0.8f)
        assertEquals(0f, DriveMath.Buttons().step(Float.NaN, false, true, 1), 0f)
        assertEquals(0f, DriveMath.Buttons().step(-1f, false, true, 1), 0f)
    }
}
