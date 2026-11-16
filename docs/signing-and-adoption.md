# Signing and repository-hosting milestone

PRYN does not currently operate signed APT or RPM repositories, and the
initial GitHub release packages are unsigned. SHA-256 checksums are published
for integrity checking, but they are not a publisher signature.

## Adoption trigger

Start the signed-distribution project when either condition is met:

1. PRYN records at least **500 combined public downloads per month for three
   consecutive months**, counting crates.io downloads and GitHub release asset
   downloads while excluding known CI artifacts where measurable; or
2. at least three independent downstream distribution maintainers or
   organizations request a maintained package repository or adopt PRYN in a
   published package feed.

This is a download/adoption signal, not a count of unique people. PRYN will not
add telemetry just to count users. Public registry and release metrics are
imperfect proxies and can include repeated downloads, CI, mirrors, and bots.

## Work triggered by the milestone

- Sign release artifacts or add GitHub artifact attestations.
- Establish and document a protected offline/limited-access signing-key
  process, backups, rotation, and revocation.
- Operate a signed APT repository with signed Release metadata.
- Operate a signed RPM repository with signed repository metadata and packages.
- Document key fingerprints and installation of repository signing keys.
- Automate signing only after the trust boundary and key storage are reviewed.

AUR's source checksum and package review model remains separate from APT/RPM
repository signing. Do not claim an unsigned GitHub checksum is equivalent to
a signature.
