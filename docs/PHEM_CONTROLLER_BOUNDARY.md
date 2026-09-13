# PHEM controller boundary

Pinned upstream: `Florin9doi/PHEM@7f2268390e1e60bd2e51c7dff617a05ca84c51f5` (tag `v1.44`).

The pinned README and `app/build.gradle`/`app/src/main/jni/Android.mk` establish an Android Gradle/JNI application. PHEM's Android session, Palm ROM, and input/preferences code do not define a desktop controller-profile format. The adapter refuses all desktop serialization; native Android integration and runtime verification are separate work.
