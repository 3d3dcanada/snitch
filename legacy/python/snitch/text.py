"""What is actually in a piece of text.

A port of the web tool's `app/snitch/lib/text.ts`, kept behaviourally identical on purpose so the
CLI, the MCP server and the website cannot disagree about what a piece of text contains. Change
one, change the other, or the tool starts telling two stories.

Everything reported here is CHECKABLE. Each finding names the exact code points it found, so a
sceptical reader can verify it in any editor. Nothing is inferred from writing style: no "this
looks like AI" guessing, because that is not measurable and would be the same class of false claim
the image side has already been audited for.

The five things worth knowing about a block of text:

 1. HIDDEN MARKS      zero-width characters carrying an identifier
 2. BIDI CONTROLS     right-to-left overrides, which can disguise what text actually says
 3. CONFUSABLES       Cyrillic or Greek letters inside Latin words, the lookalike-domain mechanism
 4. ODD WHITESPACE    non-breaking and narrow spaces that break search and survive copying
 5. ORIGIN TELLS      curly quotes and ellipses, which say the text came through a word processor

Item 5 is a weak signal and is labelled as one. It is a hint about provenance, never proof.
"""

ZWSP = "​"
ZWNJ = "‌"
ZWJ = "‍"
BOM = "﻿"
WORD_JOINER = "⁠"
INVISIBLE_SEPARATOR = "⁣"
FUNCTION_APPLICATION = "⁡"

# ZWJ (U+200D) and ZWNJ (U+200C) are deliberately absent, and must stay absent. They are
# load-bearing: ZWJ builds emoji sequences such as a family or a profession, and both are standard
# orthography in Arabic, Persian, Kurdish and Indic scripts. Stripping them corrupts real writing.
# Only characters with no legitimate role in running text belong in this set.
SUSPICIOUS_ZERO_WIDTH = (ZWSP, BOM, WORD_JOINER, INVISIBLE_SEPARATOR, FUNCTION_APPLICATION)

RLO = "‮"
LRO = "‭"
BIDI_CONTROLS = (
    "‪",  # LRE
    "‫",  # RLE
    "‬",  # PDF, pop directional formatting
    LRO,
    RLO,
    "⁦",  # LRI
    "⁧",  # RLI
    "⁨",  # FSI
    "⁩",  # PDI
)

ODD_SPACES = {
    " ": "non-breaking space",
    " ": "figure space",
    " ": "thin space",
    " ": "narrow no-break space",
    "　": "ideographic space",
    " ": "hair space",
}

ORIGIN_TELLS = {
    "‘": "curly opening quote",
    "’": "curly closing quote or apostrophe",
    "“": "curly opening double quote",
    "”": "curly closing double quote",
    "…": "ellipsis character",
    "–": "en dash",
    "—": "em dash",
}


def code_point(ch):
    return f"U+{ord(ch):04X}"


def script_of(ch):
    """Latin, Cyrillic or Greek, or None for anything else. Only these three confuse each other."""
    c = ord(ch)
    if 0x41 <= c <= 0x5A or 0x61 <= c <= 0x7A:
        return "Latin"
    if 0x0400 <= c <= 0x04FF:
        return "Cyrillic"
    if 0x0370 <= c <= 0x03FF:
        return "Greek"
    return None


def mixed_script_words(text):
    """Words containing more than one alphabet.

    This is the actual mechanism behind lookalike domains and impersonated names: a Cyrillic "а"
    is a different character from a Latin "a" and renders identically in most fonts.
    """
    out = []
    for word in text.split():
        if len(word) < 2:
            continue
        scripts = sorted({s for s in (script_of(ch) for ch in word) if s})
        if len(scripts) > 1:
            out.append({"word": word, "scripts": scripts})
    return out


def analyse(text):
    """Findings, in the same order and with the same keys as the web tool."""
    findings: list = []
    if not text:
        return findings

    hidden, bidi, spaces, tells = [], [], [], []
    for ch in text:
        if ch in SUSPICIOUS_ZERO_WIDTH:
            hidden.append(ch)
        elif ch in BIDI_CONTROLS:
            bidi.append(ch)
        elif ch in ODD_SPACES:
            spaces.append(ch)
        elif ch in ORIGIN_TELLS:
            tells.append(ch)

    def unique(seq):
        return list(dict.fromkeys(seq))

    if bidi:
        uniq = unique(bidi)
        has_override = any(c in (RLO, LRO) for c in uniq)
        findings.append({
            "key": "bidi",
            "label": "Direction-control characters",
            "value": f"{len(bidi)} found ({', '.join(code_point(c) for c in uniq)})",
            "severity": "alarming" if has_override else "notable",
            "note": (
                "This text contains a right-to-left override. That character can make text display "
                "in a different order than it is actually stored, which is the trick used to "
                "disguise what a filename or a link really says."
                if has_override else
                "Direction-control characters are legitimate in mixed right-to-left and "
                "left-to-right text, and are also used to disguise what text says. Worth knowing "
                "they are here."
            ),
            "chars": uniq,
        })

    mixed = mixed_script_words(text)
    if mixed:
        findings.append({
            "key": "confusable",
            "label": "Words mixing alphabets",
            "value": ", ".join(f"{m['word']} ({' + '.join(m['scripts'])})" for m in mixed[:4]),
            "severity": "alarming",
            "note": "These words contain letters from more than one alphabet. A Cyrillic a and a "
                    "Latin a look identical in almost every font, which is how lookalike domains "
                    "and impersonated names work. If you did not write this in more than one "
                    "language, treat it as deliberate.",
            "chars": [],
        })

    if hidden:
        uniq = unique(hidden)
        findings.append({
            "key": "hidden",
            "label": "Invisible characters",
            "value": f"{len(hidden)} found ({', '.join(code_point(c) for c in uniq)})",
            "severity": "notable",
            "note": "Invisible characters. They survive copying and pasting, they are not visible "
                    "in any editor, and they can be used to fingerprint a copy of a document. They "
                    "are also occasionally harmless leftovers from a web page.",
            "chars": uniq,
        })

    if spaces:
        uniq = unique(spaces)
        findings.append({
            "key": "space",
            "label": "Unusual spaces",
            "value": f"{len(spaces)} found ({', '.join(ODD_SPACES[c] for c in uniq)})",
            "severity": "neutral",
            "note": "Spaces that are not the ordinary space character. They survive copying, they "
                    "break search and find, and they usually arrive from a web page or a word "
                    "processor rather than being typed.",
            "chars": uniq,
        })

    if tells:
        uniq = unique(tells)
        plural = "" if len(tells) == 1 else "s"
        findings.append({
            "key": "origin",
            "label": "Came through a word processor",
            "value": f"{len(tells)} substituted character{plural} "
                     f"({', '.join(ORIGIN_TELLS[c] for c in uniq)})",
            "severity": "neutral",
            "note": "Curly quotes, ellipsis characters and long dashes are inserted automatically "
                    "by Word, Google Docs and most publishing tools. Plain typing in a code editor "
                    "does not produce them. This is a weak hint about where the text came from, "
                    "not proof of anything.",
            "chars": uniq,
        })

    if not findings:
        findings.append({
            "key": "clean",
            "label": "Nothing hidden",
            "value": "Plain text",
            "severity": "neutral",
            "note": "No invisible characters, no direction controls, no mixed alphabets and no "
                    "word-processor substitutions. What you see is what is stored.",
            "chars": [],
        })

    return findings


def strip_chars(text, chars):
    """Remove exactly the characters given, and nothing else."""
    if not chars:
        return text
    unwanted = set(chars)
    return "".join(c for c in text if c not in unwanted)


def clean(text):
    """Remove the tracking characters and leave the writing alone.

    Removes the invisible characters and the direction controls, because neither has a role in
    ordinary running text. Leaves odd spaces and word-processor substitutions in place: they are
    reported so a reader knows they are there, and removing them silently would edit someone's
    prose. Never touches ZWJ or ZWNJ, which real writing depends on.
    """
    findings = analyse(text)
    removable = [c for f in findings if f["key"] in ("hidden", "bidi") for c in f["chars"]]
    cleaned = strip_chars(text, removable)
    return {
        "text": cleaned,
        "removed": len(text) - len(cleaned),
        "removed_chars": [code_point(c) for c in removable],
        "findings": findings,
    }
