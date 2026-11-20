
# Required by NewPipeExtractor signature interpreter (upstream rules).
-keep class org.mozilla.javascript.** { *; }
-keep class org.mozilla.classfile.ClassFileWriter
-dontwarn org.mozilla.javascript.tools.**

# The extractor explicitly uses Rhino's interpreted, safe-standard-object mode.
# Its optional JVM-only optimizer and Java bean JSON adapter are not used on Android.
# These absent JDK APIs are also excluded by NewPipe's Android packaging rules.
-dontwarn java.beans.**
-dontwarn jdk.dynalink.**
# NewPipe's Protobuf-lite descriptors look up generated field names reflectively.
-keepclassmembers class * extends com.google.protobuf.GeneratedMessageLite { <fields>; }
-keep class org.schabi.newpipe.extractor.timeago.patterns.** { *; }
