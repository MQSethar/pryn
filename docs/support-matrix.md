# Support matrix (initial development stage)

PRYN targets Linux. The current code uses standard Linux sysfs/procfs interfaces and does not yet integrate with all distribution-specific service managers.

| Environment | Build target | Runtime checks |
|---|---|---|
| Arch Linux x86_64 | Development target | Basic sysfs discovery; test on real hardware |
| Ubuntu x86_64 | Development target | Basic sysfs discovery; test on real hardware |
| Other systemd distributions | Not yet validated | Best effort only |
| Non-systemd distributions | Not yet validated | Best effort only |
| aarch64 Linux | Not yet validated | Not yet claimed |

A successful compile on a distribution is not proof that every system-information provider works there. Update this matrix only after actual environment testing.
