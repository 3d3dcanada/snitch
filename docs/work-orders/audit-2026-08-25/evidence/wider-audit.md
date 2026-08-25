# Wider adversarial audit

The known backlog was not treated as the boundary. These failures were reproduced independently;
the most consequential are first.

## Findings and repairs

1. **The signing UX implied more current C2PA meaning than the manifest provides.** A live detailed
   report showed the creator's organization falsely recorded as both claim generator and
   `softwareAgent`, with a hard-coded version `1.0`. It now records `snitch 0.1.0`. The output,
   README, help, and bundled skills now call this development-grade and distinguish a valid asset
   binding from trusted identity. The remaining `stds.schema-org.CreativeWork` assertion has been
   deprecated since C2PA 2.0 and is gathered rather than created; current C2PA 2.4 and CAWG identity
   support are deliberately left as a feature proposal. Commits: `51a7b3d`, `ab82253`.
2. **Write failures could look successful and leave bad artifacts.** Locked, zero-byte, and text
   inputs printed failure but returned zero; some copied outputs survived. Writes now use a
   validated temporary sibling, publish only after every operation succeeds, and aggregate a
   nonzero status. Commits: `970e738`, `ef57fb9`, `a4ec575`.
3. **Stamping could counterfeit a file type.** WebP and TIFF requests produced JPEG bytes named
   `.webp` and `.tiff`; a missing logo was reported as success. Stamping now refuses anything but
   JPEG/PNG and validates logo/font and numeric parameters before copying. Commit: `ef57fb9`.
4. **Malformed input was reported as a clean image.** `snitch zero.jpg`, a UTF-8 text file named
   `.jpg`, and a missing file all exited zero and printed reassuring negative findings. Inspection
   now validates ExifTool's result and reports structured per-file errors. Commit: `a4ec575`.
5. **Metadata removal could alter display semantics while passing the pixel check.** ICC profiles,
   EXIF orientation, Adobe APP14 CMYK transforms, transparency, and APNG control chunks are not
   private comments. The stripper now preserves them and validates container structure. Commit:
   `886c41f`.
6. **Credential validity was not reflected in process status.** A tampered file printed `Invalid`
   but `credit --verify` exited zero; unsigned files also exited zero. Both now fail. Absence,
   invalidity, unavailability, and detection-without-validation remain separate states. Commits:
   `4e33d38`, `b1bcb39`.
7. **Filesystem-shaped inputs reached tools with surprising semantics.** A directory made ExifTool
   recurse, in-place symlinks were replaced, and relative filenames beginning with `-` were parsed
   as ExifTool or c2patool options. All commands now require regular files; mutating commands refuse
   in-place symlinks; external image/signing paths are absolute. Commits: `726e0ee`, `1f1a3f8`,
   `06cf3d8`.
8. **Signing invented a camera origin.** The previous default used `digitalCapture` for arbitrary
   files. Signing now requires an explicit controlled source choice. Commit: `4e33d38`.
9. **A valid binding was displayed as trusted authorship.** The interface exposed the certificate
   issuer as if it were an authenticated signer. It now says `certificate issuer` and prominently
   reports an untrusted identity. Commit: `5b0a04b`.
10. **Dependency and packaging paths were not release-clean.** Missing ExifTool could raise a
    traceback from `credit`; `python -m snitch` was absent; package license metadata emitted build
    deprecation warnings. These now fail cleanly or build without warnings. Commits: `8bc217d`,
    `a4ec575`, `df4a12f`.

## Residual risks

- A hard kill can strand a uniquely named temporary file. It did not replace the target in the
  observed SIGKILL test. A recovery/cleanup command is not implemented.
- The 100 MP proof still peaked at about 408 MiB despite the 75% reduction.
- Metadata-only HEIC worked with the particular real sample and ExifTool 12.76. That does not
  establish every HEIF brand, auxiliary image, or vendor variant.
- `credit` relies on ExifTool's format-specific writer guarantees. It does not claim complete
  namespace parity across JPEG, PNG, WebP, TIFF, and HEIC.
- No fuzz campaign was run against the hand-written JPEG/PNG parsers. Targeted truncation, invalid
  length, invalid CRC, illegal order, and malformed marker cases are covered.
- The C2PA signing path is useful for development and tamper demonstrations, not a conforming
  public identity product.
