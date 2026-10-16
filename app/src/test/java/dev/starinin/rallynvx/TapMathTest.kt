package dev.starinin.rallynvx

import org.junit.Assert.*
import org.junit.Test

class TapMathTest {
    private fun tap(d: DoubleTap, t: Long, x: Float = 100f, y: Float = 100f): Boolean { d.down(t, x, y); return d.up(t + 60, x + 2, y) }
    @Test fun twoQuickTapsMakeADoubleTap() { val d = DoubleTap(20f); assertFalse(tap(d, 0)); assertTrue(tap(d, 200)) }
    @Test fun aSlowSecondTapIsAFreshFirstOne() { val d = DoubleTap(20f); assertFalse(tap(d, 0)); assertFalse(tap(d, 1000)); assertTrue(tap(d, 1200)) }
    @Test fun aDragIsNoTap() { val d = DoubleTap(20f); assertFalse(tap(d, 0)); d.down(200, 100f, 100f); assertFalse(d.up(260, 220f, 100f)); assertFalse(tap(d, 300)) }
    @Test fun aLongPressIsNoTap() { val d = DoubleTap(20f); d.down(0, 5f, 5f); assertFalse(d.up(600, 5f, 5f)); assertFalse(tap(d, 700)) }
    @Test fun tapsFarApartDoNotPair() { val d = DoubleTap(20f); assertFalse(tap(d, 0, 100f)); assertFalse(tap(d, 150, 900f)) }
    @Test fun aThirdTapStartsOver() { val d = DoubleTap(20f); tap(d, 0); assertTrue(tap(d, 150)); assertFalse(tap(d, 300)) }
}
