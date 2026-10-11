package dev.starinin.raytrace

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack

/** Streams the procedural sound from [Native.audioRender] to an AudioTrack on a daemon thread. */
class AudioEngine(private val ctx: Context) {
    private var track: AudioTrack? = null
    private var thread: Thread? = null
    @Volatile private var alive = false
    private val am = ctx.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private var focus: AudioFocusRequest? = null
    private val prefs = ctx.getSharedPreferences("raytrace", Context.MODE_PRIVATE)

    var muted: Boolean
        get() = prefs.getBoolean("muted", false)
        set(v) { prefs.edit().putBoolean("muted", v).apply(); Native.audioMute(v) }

    @Synchronized fun start() {
        if (alive) return
        Native.audioMute(muted)
        val rate = 44100
        val min = AudioTrack.getMinBufferSize(rate, AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_16BIT)
        if (min <= 0) return
        val t = AudioTrack.Builder()
            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
            .setAudioFormat(AudioFormat.Builder().setSampleRate(rate).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
            .setBufferSizeInBytes(maxOf(min * 2, 4096)).setTransferMode(AudioTrack.MODE_STREAM).build()
        track = t
        val req = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).build())
            .setOnAudioFocusChangeListener { change ->
                when (change) {
                    AudioManager.AUDIOFOCUS_LOSS, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> track?.pause()
                    AudioManager.AUDIOFOCUS_GAIN -> track?.play()
                }
            }.build()
        focus = req
        am.requestAudioFocus(req)
        alive = true
        t.play()
        thread = Thread({
            val buf = ShortArray(1024)
            while (alive) {
                Native.audioRender(buf)
                if (t.write(buf, 0, buf.size) < 0) break
            }
        }, "audio").apply { isDaemon = true; start() }
    }

    @Synchronized fun stop() {
        alive = false
        thread?.join(500); thread = null
        track?.let { try { it.stop() } catch (_: IllegalStateException) {}; it.release() }
        track = null
        focus?.let { am.abandonAudioFocusRequest(it) }; focus = null
    }
}
