package com.nexus.nexus_mobile

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.security.KeyStore
import java.util.Base64
import java.util.UUID
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Nexus native security surface (AUD-040).
 *
 * The previous MainActivity was a stock FlutterActivity with NO native
 * security implementation while the pubspec advertised passkeys,
 * biometrics, and secure storage. This activity binds the REAL
 * documented platform surface behind the `nexus.mobile/security`
 * MethodChannel:
 *
 *   - biometricCapability -> {"available": bool}
 *     via androidx.biometric.BiometricManager (BIOMETRIC_STRONG).
 *   - biometricVerify      -> {"verified": bool, "correlation": uuid}
 *     via a crypto-based BiometricPrompt (Cipher tied to a Keystore
 *     key with setUserAuthenticationRequired(true)). The prompt MUST
 *     complete with a user-authentication-bound cipher before verified
 *     can be true - a cancelled/failed prompt is a denial.
 *   - secureStore/Read/Delete -> Keystore-backed AES/GCM store keyed
 *     per entry. FAILS CLOSED: any exception, missing key, or
 *     unauthenticated state returns the honest negative result and
 *     NEVER fabricates stored/verified success.
 *
 * Hardware verification is NOT asserted from Dart: the platform
 * reports its own capability and the shell treats "no biometric
 * hardware" as an honest Unavailable.
 */
class MainActivity : FlutterActivity() {
    private val channelName = "nexus.mobile/security"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            channelName,
        ).setMethodCallHandler { call: MethodCall, result: MethodChannel.Result ->
            handle(call, result)
        }
    }

    private fun handle(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "biometricCapability" -> {
                val manager = BiometricManager.from(this)
                val available =
                    manager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) ==
                        BiometricManager.BIOMETRIC_SUCCESS
                result.success(mapOf("available" to available))
            }
            "biometricVerify" -> {
                val reason = call.argument<String>("reason") ?: ""
                val correlation = UUID.randomUUID().toString()
                verifyBiometric(reason, correlation, result)
            }
            "secureStore" -> {
                val key = call.argument<String>("key") ?: ""
                val value = call.argument<String>("value") ?: ""
                if (key.isEmpty()) {
                    result.success(mapOf("stored" to false))
                    return
                }
                result.success(mapOf("stored" to storeSecret(key, value)))
            }
            "secureRead" -> {
                val key = call.argument<String>("key") ?: ""
                result.success(mapOf("value" to readSecret(key)))
            }
            "secureDelete" -> {
                val key = call.argument<String>("key") ?: ""
                result.success(mapOf("deleted" to deleteSecret(key)))
            }
            else -> result.notImplemented()
        }
    }

    private fun verifyBiometric(
        reason: String,
        correlation: String,
        result: MethodChannel.Result,
    ) {
        val manager = BiometricManager.from(this)
        val canAuth =
            manager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) ==
                BiometricManager.BIOMETRIC_SUCCESS
        if (!canAuth) {
            // No biometric hardware/enrolled: honest denial.
            result.success(mapOf("verified" to false, "correlation" to ""))
            return
        }
        val cipher = createAuthenticationCipher()
        if (cipher == null) {
            result.success(mapOf("verified" to false, "correlation" to ""))
            return
        }
        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle("Nexus verification")
            .setSubtitle(reason.ifEmpty { "Verify to approve this action" })
            .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
            .setNegativeButtonText("Cancel")
            .build()
        val prompt = BiometricPrompt(
            this,
            ContextCompat.getMainExecutor(this),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result0: BiometricPrompt.AuthenticationResult) {
                    // Success is ONLY reported after the user
                    // authenticated AND the cipher is
                    // user-authentication-bound (CryptoObject).
                    val authenticated = result0.cryptoObject != null
                    result.success(
                        mapOf(
                            "verified" to authenticated,
                            "correlation" to if (authenticated) correlation else "",
                        ),
                    )
                }

                override fun onAuthenticationError(
                    errorCode: Int,
                    errString: CharSequence,
                ) {
                    result.success(mapOf("verified" to false, "correlation" to ""))
                }

                override fun onAuthenticationFailed() {
                    result.success(mapOf("verified" to false, "correlation" to ""))
                }
            },
        )
        prompt.authenticate(promptInfo, BiometricPrompt.CryptoObject(cipher))
    }

    private fun createAuthenticationCipher(): Cipher? {
        return try {
            val key = getOrCreateKey("nexus_auth_key", requireAuth = true)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            cipher
        } catch (e: Exception) {
            null
        }
    }

    private fun getOrCreateKey(alias: String, requireAuth: Boolean): SecretKey {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val existing = keyStore.getKey(alias, null) as? SecretKey
        if (existing != null) {
            return existing
        }
        val generator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore",
        )
        generator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(requireAuth)
                .build(),
        )
        return generator.generateKey()
    }

    private fun storeSecret(key: String, value: String): Boolean {
        return try {
            val secretKey = getOrCreateKey("nexus_store_$key", requireAuth = false)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, secretKey)
            val encrypted = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
            val iv = cipher.iv
            getSharedPreferences("nexus_secure_store", MODE_PRIVATE)
                .edit()
                .putString("$key:iv", Base64.getEncoder().encodeToString(iv))
                .putString("$key:ct", Base64.getEncoder().encodeToString(encrypted))
                .commit()
        } catch (e: Exception) {
            false
        }
    }

    private fun readSecret(key: String): String? {
        return try {
            val secretKey = getOrCreateKey("nexus_store_$key", requireAuth = false)
            val prefs = getSharedPreferences("nexus_secure_store", MODE_PRIVATE)
            val iv = prefs.getString("$key:iv", null) ?: return null
            val ct = prefs.getString("$key:ct", null) ?: return null
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(
                Cipher.DECRYPT_MODE,
                secretKey,
                GCMParameterSpec(128, Base64.getDecoder().decode(iv)),
            )
            String(cipher.doFinal(Base64.getDecoder().decode(ct)), Charsets.UTF_8)
        } catch (e: Exception) {
            null
        }
    }

    private fun deleteSecret(key: String): Boolean {
        return try {
            getSharedPreferences("nexus_secure_store", MODE_PRIVATE)
                .edit()
                .remove("$key:iv")
                .remove("$key:ct")
                .commit()
        } catch (e: Exception) {
            false
        }
    }
}
