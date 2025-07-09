package com.vpilotalert
import android.content.ContentResolver
import android.database.Cursor
import android.media.RingtoneManager
import android.net.Uri
import android.provider.MediaStore
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import android.media.MediaPlayer
import android.media.AudioManager

class AlarmSoundsModule(reactContext: ReactApplicationContext) : ReactContextBaseJavaModule(reactContext) {
    private var mediaPlayer: MediaPlayer? = null

    override fun getName(): String {
        return "AlarmSounds"
    }

    @ReactMethod
    fun getAlarmSounds(promise: Promise) {
        try {
            val ringtoneManager = RingtoneManager(reactApplicationContext)
            ringtoneManager.setType(RingtoneManager.TYPE_ALL)
            val cursor: Cursor = ringtoneManager.cursor

            val soundsList = JSONArray()
            while (cursor.moveToNext()) {
                val title = cursor.getString(RingtoneManager.TITLE_COLUMN_INDEX)
                val uri: Uri = ringtoneManager.getRingtoneUri(cursor.position)

                val filePath = getRealPathFromURI(uri)

                val sound = JSONObject()
                sound.put("title", title)
                sound.put("uri", filePath ?: uri.toString())
                soundsList.put(sound)
            }

            promise.resolve(soundsList.toString())
        } catch (e: Exception) {
            promise.reject("ERROR", e.message)
        }
    }

    private fun getRealPathFromURI(uri: Uri): String? {
        val contentResolver: ContentResolver = reactApplicationContext.contentResolver
        val cursor = contentResolver.query(uri, null, null, null, null) ?: return null
        return cursor.use {
            if (it.moveToFirst()) {
                val index = it.getColumnIndex("_data")
                if (index != -1) {
                    return@use it.getString(index)
                }
            }
            null
        }
    }

    @ReactMethod
    fun playSound(uri: String) {
        stopSound()
        mediaPlayer = MediaPlayer().apply {
            setDataSource(uri)
            setAudioStreamType(AudioManager.STREAM_ALARM)
            isLooping = true
            prepare()
            start()
        }
    }

    @ReactMethod
    fun stopSound() {
        mediaPlayer?.let {
            if (it.isPlaying) {
                it.stop()
                it.release()
            }
        }
        mediaPlayer = null
    }

    @ReactMethod(isBlockingSynchronousMethod = true)
    fun isPlaying(): Boolean {
        mediaPlayer?.let {
            return it.isPlaying
        }
        return false
    }
}
