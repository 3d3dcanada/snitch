# Platform table audit — 2026-08-25

## Result

The former table was not publishable as written. It labelled every row "verified 2026-08-25"
without retained upload/download evidence and made absolute claims about LinkedIn, Meta, X,
Reddit, Printables, and Google Images. Those claims have been replaced by a per-cell evidence model.

No account-based upload was performed in this audit. No cell is therefore classified as a live
round-trip. `D` means the platform currently documents the narrower behaviour in the note; `C`
would mean a retained, current, independent before/upload/download/after test; `?` is an explicitly
unverified expectation. Reading metadata during ingestion is not treated as preserving it in a
downloadable derivative.

## All 28 cells

| Platform | Visible stamp | C2PA | IPTC/XMP | EXIF |
|---|---:|---:|---:|---:|
| LinkedIn | ? keeps | D partial | ? unknown | ? unknown |
| Instagram | ? keeps | D partial | D partial | ? unknown |
| Facebook | ? keeps | D partial | D partial | ? unknown |
| X / Twitter | ? keeps | ? unknown | ? unknown | D strips |
| Reddit | ? keeps | ? unknown | ? unknown | D partial |
| Printables | ? keeps | ? unknown | ? unknown | ? unknown |
| Google Images | ? keeps | D partial | D reads | ? unknown |

The complete limitation and source for every cell is available in both:

```bash
snitch --platforms --notes
snitch --json --platforms --notes
```

`live_tested` is `false` for every cell in the machine-readable report. This prevents a platform's
own documentation from being mistaken for an observed derivative test.

## Current first-party sources

Accessed 2026-08-25:

- LinkedIn Help, “Content credentials”:
  https://www.linkedin.com/help/linkedin/answer/a6282984
- Meta, “Labeling AI-Generated Images on Facebook, Instagram and Threads”:
  https://about.fb.com/news/2024/02/labeling-ai-generated-images-on-facebook-instagram-and-threads/
- Meta, “How Meta Is Preparing for the 2026 US Midterm Elections”:
  https://about.fb.com/news/2026/02/meta-prepares-for-2026-us-midterms/
- X Help, “How to post photos or GIFs”:
  https://help.x.com/en/using-x/posting-gifs-and-pictures
- Reddit, “Privacy by design”:
  https://redditinc.com/privacy
- Google Search Central, “Image metadata in Google Images”:
  https://developers.google.com/search/docs/appearance/structured-data/image-license-metadata
- Google Search Help, “Find Google Image details”:
  https://support.google.com/websearch/answer/9789430

## Observed verification

```text
$ pytest -q tests/test_survival.py tests/test_snitch_cli.py
.............                                                            [100%]
13 passed in 0.61s

$ NO_COLOR=1 snitch --platforms --notes --check
Evidence on platform metadata handling   researched 2026-08-25
...
LinkedIn       ? keeps   D partial   ? unknown   ? unknown
...

$ NO_COLOR=1 snitch --json --platforms --notes --check | python -m json.tool >/dev/null
exit 0
```

The remaining verification gap is intentional and visible: authenticated platform round trips must
be run with the exact account, app, file type, and upload route before any `?` becomes `C`.
