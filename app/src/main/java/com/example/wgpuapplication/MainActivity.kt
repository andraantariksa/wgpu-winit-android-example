package com.example.wgpuapplication

import android.os.Bundle
import com.google.androidgamesdk.GameActivity

class MainActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
    }

    companion object {
        init {
            System.loadLibrary("wgpu_android_lib")
        }
    }
}