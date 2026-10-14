package dev.starinin.rallynvx

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import org.json.JSONObject

/** A co-driver voice: one recorded clip per word (assets/voices/<id>/<clip>.wav, 16 kHz mono 16-bit) stitched into a call. */
class Voice(val id: String, val lang: String, val name: String, val license: String, val source: String)

class VoicePlayer(private val ctx: Context) {
    private val cache = HashMap<String, ShortArray>()
    private var track: AudioTrack? = null

    /** The voices of the assets (`voices/index.json`), empty when the build has none. */
    fun voices(): List<Voice> = try {
        val arr = JSONObject(ctx.assets.open("voices/index.json").bufferedReader().use { it.readText() }).getJSONArray("voices")
        (0 until arr.length()).map { arr.getJSONObject(it).let { o -> Voice(o.getString("id"), o.getString("lang"), o.getString("name"), o.getString("license"), o.getString("source")) } }
    } catch (e: Exception) { emptyList() }

    private fun clip(voice: String, id: String): ShortArray? = cache.getOrPut("$voice/$id") {
        try { WavReader.pcm16(ctx.assets.open("voices/$voice/$id.wav").use { it.readBytes() }) ?: ShortArray(0) } catch (e: Exception) { ShortArray(0) }
    }.takeIf { it.isNotEmpty() }

    /** Plays the clips one after another (a call replaces the one still playing); the volume is the media volume. */
    fun play(voice: String, clips: List<String>, gapMs: Int = 60) {
        // the id "-" is a pause between calls
        val parts = clips.mapNotNull { if (it == "-") ShortArray(16 * 450) else clip(voice, it) }
        if (parts.isEmpty()) return
        val gap = ShortArray(16 * gapMs)
        val pcm = ShortArray(parts.sumOf { it.size } + gap.size * (parts.size - 1))
        var at = 0
        parts.forEachIndexed { i, p -> p.copyInto(pcm, at); at += p.size; if (i < parts.size - 1) at += gap.size }
        stop()
        val t = AudioTrack.Builder()
            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build())
            .setAudioFormat(AudioFormat.Builder().setEncoding(AudioFormat.ENCODING_PCM_16BIT).setSampleRate(16000).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
            .setBufferSizeInBytes(pcm.size * 2).setTransferMode(AudioTrack.MODE_STATIC).build()
        t.write(pcm, 0, pcm.size)
        t.play()
        track = t
    }

    fun stop() { track?.let { try { it.stop() } catch (_: Exception) {}; it.release() }; track = null }
    fun release() { stop(); cache.clear() }
}

/** Reads a canonical 16-bit PCM wav (the clips are mono, 16 kHz). */
object WavReader {
    fun pcm16(b: ByteArray): ShortArray? {
        var p = 12   // after "RIFF" size "WAVE"
        while (p + 8 <= b.size) {
            val size = (b[p + 4].toInt() and 0xFF) or ((b[p + 5].toInt() and 0xFF) shl 8) or ((b[p + 6].toInt() and 0xFF) shl 16) or ((b[p + 7].toInt() and 0xFF) shl 24)
            if (b[p].toInt() == 'd'.code && b[p + 1].toInt() == 'a'.code && b[p + 2].toInt() == 't'.code && b[p + 3].toInt() == 'a'.code) {
                val n = minOf(size.coerceAtLeast(0), b.size - (p + 8)) / 2
                return ShortArray(n) { ((b[p + 8 + 2 * it].toInt() and 0xFF) or (b[p + 9 + 2 * it].toInt() shl 8)).toShort() }
            }
            p += 8 + size + (size and 1)
        }
        return null
    }
}
