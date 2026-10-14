package dev.starinin.rallynvx

import org.junit.Assert.assertEquals
import org.junit.Test

class RallyDataTest {
    @Test fun timeFormatsLikeAStopwatch() {
        assertEquals("0:00.0", RallyData.time(0f))
        assertEquals("0:09.9", RallyData.time(9.99f))
        assertEquals("1:23.4", RallyData.time(83.45f))
        assertEquals("10:05.0", RallyData.time(605f))
    }
    @Test fun brokenValuesShowZero() {
        assertEquals("0:00.0", RallyData.time(-3f))
        assertEquals("0:00.0", RallyData.time(Float.NaN))
        assertEquals("0:00.0", RallyData.time(Float.POSITIVE_INFINITY))
    }
    @Test fun everyRallyHasASeedAndAName() {
        assertEquals(4, RallyData.SEEDS.size)
        assertEquals(RallyData.SEEDS.size, RallyData.SEEDS.toSet().size)
    }
}
