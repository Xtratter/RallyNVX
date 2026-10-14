package dev.starinin.rallynvx

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WavReaderTest {
    private fun wav(samples: ShortArray): ByteArray {
        val data = ByteArray(samples.size * 2) { i -> val s = samples[i / 2].toInt(); (if (i % 2 == 0) s and 0xFF else (s shr 8) and 0xFF).toByte() }
        fun le(v: Int) = byteArrayOf(v.toByte(), (v shr 8).toByte(), (v shr 16).toByte(), (v shr 24).toByte())
        return "RIFF".toByteArray() + le(36 + data.size) + "WAVE".toByteArray() + "fmt ".toByteArray() + le(16) +
            byteArrayOf(1, 0, 1, 0) + le(16000) + le(32000) + byteArrayOf(2, 0, 16, 0) + "data".toByteArray() + le(data.size) + data
    }
    @Test fun readsTheSamples() {
        val s = shortArrayOf(0, 1, -1, 32767, -32768, 1234)
        assertArrayEquals(s, WavReader.pcm16(wav(s)))
    }
    @Test fun skipsOtherChunksAndRejectsGarbage() {
        val s = shortArrayOf(5, 6)
        val w = wav(s)
        // a LIST chunk before the data chunk
        val withList = w.copyOfRange(0, 36) + "LIST".toByteArray() + byteArrayOf(4, 0, 0, 0) + byteArrayOf(1, 2, 3, 4) + w.copyOfRange(36, w.size)
        assertArrayEquals(s, WavReader.pcm16(withList))
        assertNull(WavReader.pcm16(ByteArray(10)))
    }
}
