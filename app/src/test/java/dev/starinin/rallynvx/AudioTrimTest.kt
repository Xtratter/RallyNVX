package dev.starinin.rallynvx

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.abs
import kotlin.math.sin

class AudioTrimTest {
    private fun tone(n: Int, amp: Int, rate: Int = 16000) = ShortArray(n) { (amp * sin(2 * Math.PI * 300 * it / rate)).toInt().toShort() }

    @Test fun trimsTheSilenceAroundTheVoice() {
        val x = ShortArray(8000) + tone(8000, 6000) + ShortArray(8000)   // 0.5 s of nothing, 0.5 s of voice, 0.5 s of nothing
        val y = AudioTrim.clean(x)
        assertTrue("the clip keeps about the voice plus the padding: ${y.size}", y.size in 8000..9500)
    }
    @Test fun matchesTheLoudnessAndNeverClips() {
        val quiet = AudioTrim.clean(tone(8000, 800)); val loud = AudioTrim.clean(tone(8000, 30000))
        fun rms(a: ShortArray) = Math.sqrt(a.sumOf { it.toDouble() * it } / a.size)
        assertEquals(rms(quiet), rms(loud), rms(quiet) * 0.2)
        assertTrue(loud.all { abs(it.toInt()) <= 32767 * 0.96 })
    }
    @Test fun silenceAndEmptyGiveNothing() {
        assertEquals(0, AudioTrim.clean(ShortArray(0)).size)
        assertEquals(0, AudioTrim.clean(ShortArray(4000)).size)
    }
    @Test fun theEdgesAreFaded() {
        val y = AudioTrim.clean(tone(8000, 8000))
        assertTrue(abs(y.first().toInt()) < 200 && abs(y.last().toInt()) < 200)
    }
}
