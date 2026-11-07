package com.hyperb.app

import android.content.Context

/** The Rust core (`crates/mobile`). See [Session] for the handle rules. */
object RustBridge {
    init {
        System.loadLibrary("hyperb_mobile")
    }

    /** Sets up logging, TLS and the identity kept in [filesDir]; returns its public key. */
    @JvmStatic external fun init(context: Context, filesDir: String): String

    /**
     * Starts a session, which goes to the history unless [reload]. A [link] (from another app)
     * cannot open local files. Throws [RuntimeException] if the address is invalid.
     */
    @JvmStatic external fun open(address: String, link: Boolean, reload: Boolean): Long
    @JvmStatic external fun cancel(handle: Long)
    @JvmStatic external fun free(handle: Long)

    /** Blocks until network events have been applied; false once the session is over. */
    @JvmStatic external fun await(handle: Long): Boolean
    @JvmStatic external fun snapshot(handle: Long): String

    /**
     * Input on element [id]: [value] (JSON) goes to its bound signal, then [event] runs. Returns
     * the URLs a button asked to open (`http`, `https`, `mailto` and hyperb addresses), as a JSON
     * array of `{"url", "app"}`, where `app` marks a hyperb address.
     */
    @JvmStatic external fun input(handle: Long, id: String, value: String?, event: String?): String

    /** The history and favorites, as JSON; see [Library]. */
    @JvmStatic external fun library(): String

    /** The demo apps, as a JSON array of `{"address", "title", "about"}`; see [Library.demos]. */
    @JvmStatic external fun demos(): String

    /** The `https` icon at [src], downloaded unless it is kept already; blocks. */
    @JvmStatic external fun icon(src: String): ByteArray?

    /** Adds or removes a favorite (a wasm app is then kept for offline use); returns the library. */
    @JvmStatic external fun setFavorite(address: String, on: Boolean): String

    /** Removes [address] from the history, or clears it if null; returns the library. */
    @JvmStatic external fun forget(address: String?): String

    /** Records that [address] has a shortcut, or no longer has one (a wasm app is then kept for offline use). */
    @JvmStatic external fun setShortcut(address: String, on: Boolean)

    /** Makes the apps in [addresses] (a JSON array) the ones with a shortcut, as the system lists them. */
    @JvmStatic external fun setShortcuts(addresses: String)

    /** Whether [address] runs from a kept module and a newer one is available; blocks. */
    @JvmStatic external fun checkUpdate(address: String): Boolean

    /** Makes the newer module [checkUpdate] found the one [address] runs from. */
    @JvmStatic external fun applyUpdate(address: String)
}
