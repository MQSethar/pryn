PRYN quality-gate follow-up

1. Import AudioAvailability in the audio-server unit-test module.
2. Compile the test-only evaluate_snapshot helper only under cfg(test), removing
   the dead-code warning from normal library and release builds.

No runtime audio or Bluetooth behavior is changed by this patch.
