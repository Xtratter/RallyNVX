package dev.starinin.rallynvx

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PaceStyleTest {
    @Test fun colorsRunFromGreenForEasyToRedForAHairpin() {
        assertEquals(PaceStyle.GREEN, PaceStyle.color(6)); assertEquals(PaceStyle.GREEN, PaceStyle.color(4))
        assertEquals(PaceStyle.YELLOW, PaceStyle.color(3)); assertEquals(PaceStyle.YELLOW, PaceStyle.color(2))
        assertEquals(PaceStyle.ORANGE, PaceStyle.color(1)); assertEquals(PaceStyle.RED, PaceStyle.color(0))
    }
    @Test fun theArrowBendsMoreForSharperCorners() {
        val bends = (6 downTo 1).map { PaceStyle.bend(it) }
        assertTrue(bends.zipWithNext().all { it.first < it.second })
        assertTrue(PaceStyle.bend(6) < 20f && PaceStyle.bend(1) > 90f)
    }
    @Test fun distancesAreRoundedLikeTheCoDriverSaysThem() {
        assertEquals(70, PaceStyle.roundDistance(73f)); assertEquals(100, PaceStyle.roundDistance(120f))
        assertEquals(150, PaceStyle.roundDistance(140f)); assertEquals(400, PaceStyle.roundDistance(430f))
    }
}
