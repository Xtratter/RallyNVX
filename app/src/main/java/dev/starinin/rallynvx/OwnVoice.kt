package dev.starinin.rallynvx

import android.annotation.SuppressLint
import android.content.Context
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import java.io.ByteArrayOutputStream
import java.io.File
import kotlin.math.abs
import kotlin.math.sqrt
import kotlin.math.tanh

/** Cleans a recorded clip like the studio clips are cleaned: silence trimmed, loudness matched, edges faded. */
object AudioTrim {
    fun clean(x: ShortArray, rate: Int = 16000): ShortArray {
        if (x.isEmpty()) return x
        val peak = x.maxOf { abs(it.toInt()) }.coerceAtLeast(1)
        val thr = (0.03f * peak).toInt()
        val first = x.indexOfFirst { abs(it.toInt()) > thr }
        val last = x.indexOfLast { abs(it.toInt()) > thr }
        if (first < 0) return ShortArray(0)
        val from = (first - rate / 50).coerceAtLeast(0)
        val to = (last + rate / 20).coerceAtMost(x.size - 1)
        val cut = FloatArray(to - from + 1) { x[from + it] / 32768f }
        val rms = sqrt(cut.sumOf { (it * it).toDouble() } / cut.size).toFloat().coerceAtLeast(1e-4f)
        val gain = 0.11f / rms
        val fade = rate * 8 / 1000
        return ShortArray(cut.size) { i ->
            var v = tanh(cut[i] * gain) * 0.95f
            if (i < fade) v *= i / fade.toFloat()
            if (cut.size - 1 - i < fade) v *= (cut.size - 1 - i) / fade.toFloat()
            (v * 32767f).toInt().toShort()
        }
    }
}

/** The player's own voice: one recorded clip per phrase and language in the app's files, 16 kHz mono 16-bit wav. */
object OwnVoice {
    fun dir(c: Context, lang: String) = File(c.filesDir, "own_voice/$lang").apply { mkdirs() }
    fun file(c: Context, lang: String, clip: String) = File(dir(c, lang), "$clip.wav")
    fun has(c: Context, lang: String, clip: String) = file(c, lang, clip).let { it.exists() && it.length() > 100 }
    fun count(c: Context, lang: String, clips: Collection<String>) = clips.count { has(c, lang, it) }
    fun delete(c: Context, lang: String, clip: String) { file(c, lang, clip).delete() }
    fun read(c: Context, lang: String, clip: String): ShortArray? = if (has(c, lang, clip)) WavReader.pcm16(file(c, lang, clip).readBytes()) else null

    fun write(c: Context, lang: String, clip: String, pcm: ShortArray) {
        val data = ByteArray(pcm.size * 2) { i -> val s = pcm[i / 2].toInt(); (if (i % 2 == 0) s and 0xFF else (s shr 8) and 0xFF).toByte() }
        fun le(v: Int) = byteArrayOf(v.toByte(), (v shr 8).toByte(), (v shr 16).toByte(), (v shr 24).toByte())
        file(c, lang, clip).writeBytes("RIFF".toByteArray() + le(36 + data.size) + "WAVE".toByteArray() + "fmt ".toByteArray() + le(16) +
            byteArrayOf(1, 0, 1, 0) + le(16000) + le(32000) + byteArrayOf(2, 0, 16, 0) + "data".toByteArray() + le(data.size) + data)
    }
}

/** Records the microphone (16 kHz mono) until `stop()`; needs the RECORD_AUDIO permission. */
class ClipRecorder {
    private var rec: AudioRecord? = null
    private var thread: Thread? = null
    private val out = ByteArrayOutputStream()
    @Volatile private var running = false

    @SuppressLint("MissingPermission")
    fun start(): Boolean {
        val min = AudioRecord.getMinBufferSize(16000, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
        if (min <= 0) return false
        val r = AudioRecord(MediaRecorder.AudioSource.MIC, 16000, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT, min * 2)
        if (r.state != AudioRecord.STATE_INITIALIZED) { r.release(); return false }
        out.reset(); rec = r; running = true
        r.startRecording()
        thread = Thread {
            val buf = ByteArray(min)
            while (running) { val n = r.read(buf, 0, buf.size); if (n > 0) synchronized(out) { out.write(buf, 0, n) } }
        }.also { it.start() }
        return true
    }

    /** Stops and returns the raw samples. */
    fun stop(): ShortArray {
        running = false
        thread?.join(500)
        rec?.let { try { it.stop() } catch (_: Exception) {}; it.release() }
        rec = null
        val b = synchronized(out) { out.toByteArray() }
        return ShortArray(b.size / 2) { ((b[2 * it].toInt() and 0xFF) or (b[2 * it + 1].toInt() shl 8)).toShort() }
    }
}
