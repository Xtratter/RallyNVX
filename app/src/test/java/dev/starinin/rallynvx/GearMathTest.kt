package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class GearMathTest {
    @Test fun labels() { assertEquals("R", GearMath.label(-1)); assertEquals("N", GearMath.label(0)); assertEquals("4", GearMath.label(4)) }
    @Test fun modes() { assertEquals("МКПП", GearMath.mode(true, false)); assertEquals("auto", GearMath.mode(false, true)) }
    @Test fun revBar() { assertEquals(0f, GearMath.revFraction(-5f), 0f); assertEquals(1f, GearMath.revFraction(9000f), 0f); assertEquals(0.5f, GearMath.revFraction(3900f), 1e-6f); assertEquals(0f, GearMath.revFraction(Float.NaN), 0f) }
    @Test fun clutchPedalFollowsTheFinger() { assertEquals(0f, GearMath.clutchFromTouch(0f, 100f), 0f); assertEquals(1f, GearMath.clutchFromTouch(100f, 100f), 0f); assertTrue(GearMath.clutchFromTouch(50f, 100f) in 0.4f..0.6f) }
}
