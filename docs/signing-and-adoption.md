# Signing and repository-hosting milestone

PRYN v0.1.1 does not operate signed APT or RPM repositories, and its initial release packages are unsigned. SHA-256 checksums are published for integrity checking, but they are not publisher signatures.

## Adoption trigger

Start the signed-distribution project when either condition is met:

1. At least **500 combined public downloads per month for three consecutive months**, counting crates.io and GitHub release downloads while excluding known CI artifacts where measurable; or
2. At least three independent downstream distribution maintainers or organizations request a maintained package repository or adopt PRYN in a published package feed.

This is an adoption signal, not a count of unique users. Public registry and release metrics are imperfect proxies and can include repeated downloads, CI, mirrors, and bots. PRYN will not add telemetry just to count users.

## Work triggered by the milestone

- Sign release artifacts or add GitHub artifact attestations.
- Establish a protected signing-key process, including restricted access, offline backups, rotation, and revocation.
- Operate a signed APT repository with signed Release metadata.
- Operate a signed RPM repository with signed repository metadata and packages.
- Document key fingerprints and how users install the repository signing key.
- Automate signing only after the trust boundary and key storage are reviewed.

AUR package source checksums and package review are a separate trust mechanism. Do not describe an unsigned GitHub checksum as equivalent to a digital signature.
