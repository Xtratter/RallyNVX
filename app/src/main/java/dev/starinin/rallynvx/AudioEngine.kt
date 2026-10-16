package dev.starinin.rallynvx

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.Process

/** Streams the procedural stereo sound from [Native.audioRender] to an AudioTrack on a daemon thread. */
class AudioEngine(ctx: Context) {
    private var track: AudioTrack? = null
    private var thread: Thread? = null
    @Volatile private var alive = false
    @Volatile private var paused = false
    private val am = ctx.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private var focus: AudioFocusRequest? = null
    private val prefs = ctx.getSharedPreferences("rallynvx", Context.MODE_PRIVATE)

    /** Muting also gives the audio focus back (the user's music must not be paused by a muted game). */
    var muted: Boolean
        get() = prefs.getBoolean("muted", false)
        set(v) {
            prefs.edit().putBoolean("muted", v).apply(); Native.audioMute(v)
            synchronized(this) { if (alive) { if (v) abandonFocus() else requestFocus() } }
        }

    private fun requestFocus() {
        if (focus != null) return
        val req = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).build())
            .setOnAudioFocusChangeListener { change ->
                when (change) {
                    AudioManager.AUDIOFOCUS_LOSS, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> {
                        paused = true
                        try { track?.pause(); track?.flush() } catch (_: IllegalStateException) {}
                    }
                    AudioManager.AUDIOFOCUS_GAIN -> { paused = false; try { track?.play() } catch (_: IllegalStateException) {} }
                }
            }.build()
        focus = req
        am.requestAudioFocus(req)
    }

    private fun abandonFocus() { focus?.let { am.abandonAudioFocusRequest(it) }; focus = null; paused = false; try { track?.play() } catch (_: IllegalStateException) {} }

    @Synchronized fun start() {
        if (alive) return
        Native.audioMute(muted)
        val rate = 44100
        val min = AudioTrack.getMinBufferSize(rate, AudioFormat.CHANNEL_OUT_STEREO, AudioFormat.ENCODING_PCM_16BIT)
        if (min <= 0) return
        val t = try {
            AudioTrack.Builder()
                .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
                .setAudioFormat(AudioFormat.Builder().setSampleRate(rate).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_STEREO).build())
                .setBufferSizeInBytes(maxOf(min * 4, 8192)).setTransferMode(AudioTrack.MODE_STREAM).build()
        } catch (e: Exception) { return }   // no sound is better than a crash
        if (t.state != AudioTrack.STATE_INITIALIZED) { t.release(); return }
        track = t
        paused = false
        if (!muted) requestFocus()
        try { t.play() } catch (e: IllegalStateException) { t.release(); track = null; abandonFocus(); return }
        alive = true
        thread = Thread({
            Process.setThreadPriority(Process.THREAD_PRIORITY_AUDIO)
            val buf = ShortArray(2048)   // 1024 stereo frames, interleaved
            try {
                while (alive) {
                    if (paused) { Thread.sleep(20); continue }
                    Native.audioRender(buf)
                    if (t.write(buf, 0, buf.size) < 0) alive = false   // e.g. the audio server died: start() may be called again
                }
            } catch (_: Exception) { alive = false }
        }, "audio").apply { isDaemon = true; start() }
    }

    @Synchronized fun stop() {
        alive = false
        // unblock a pending write first, then wait for the thread
        track?.let { try { it.pause(); it.flush(); it.stop() } catch (_: IllegalStateException) {} }
        thread?.join(300); thread = null
        track?.release(); track = null
        focus?.let { am.abandonAudioFocusRequest(it) }; focus = null
    }
}
