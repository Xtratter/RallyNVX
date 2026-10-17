package dev.starinin.rallynvx

import android.app.Application
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import java.io.File

/** What goes into a report the player can copy and paste: the device, the app, the last game error and the tail of the app's own log. */
object Diag {
    fun header(ctx: Context): String {
        val v = try { ctx.packageManager.getPackageInfo(ctx.packageName, 0).versionName } catch (_: Exception) { "?" }
        return "RallyNVX $v · ${Build.MANUFACTURER} ${Build.MODEL} · Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT}) · ${Build.SUPPORTED_ABIS.joinToString()}"
    }

    /** The tail of this app's own log (an app may read its own lines without any permission). */
    fun logTail(lines: Int = 250): String = try {
        val p = Runtime.getRuntime().exec(arrayOf("logcat", "-d", "-t", lines.toString()))
        val text = p.inputStream.bufferedReader().readText()
        p.destroy(); text.takeLast(24000)
    } catch (e: Exception) { "(no log: ${e.message})" }

    fun report(ctx: Context): String = buildString {
        append(header(ctx)).append('\n')
        val err = try { Native.gameLastError() } catch (_: Throwable) { "" }
        if (err.isNotEmpty()) append("Game error: ").append(err).append('\n')
        append("--- log ---\n").append(logTail())
    }

    fun copy(ctx: Context, text: String) {
        (ctx.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("RallyNVX", text))
    }

    fun crashFile(ctx: Context) = File(ctx.filesDir, "last_crash.txt")
}

/** Keeps the stack trace of a crash in a file so that the next start can show it (and offer to copy it). */
class RallyApp : Application() {
    override fun onCreate() {
        super.onCreate()
        val old = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { t, e ->
            try { Diag.crashFile(this).writeText(Diag.header(this) + "\nThread: ${t.name}\n" + android.util.Log.getStackTraceString(e)) } catch (_: Throwable) {}
            old?.uncaughtException(t, e)
        }
    }
}
