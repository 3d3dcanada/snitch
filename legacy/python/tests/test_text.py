"""The text scanner, and above all the characters it must never touch.

Stripping U+200D and U+200C indiscriminately is the obvious way to write this and it is wrong. ZWJ
builds emoji sequences and both are standard orthography in Arabic, Persian, Kurdish and Indic
scripts, so a naive sweep silently corrupts real writing in four languages and turns a family into
three strangers. The web tool's own source records catching that in test. This file is the reason
it cannot come back.
"""

from snitch import text

ZWSP = "​"
ZWNJ = "‌"
ZWJ = "‍"
BOM = "﻿"
RLO = "‮"


def keys(findings):
    return [f["key"] for f in findings]


# ----------------------------------------------------------------------------------------------
# what must survive
# ----------------------------------------------------------------------------------------------

def test_emoji_zwj_sequences_are_left_exactly_alone():
    samples = [
        "\U0001F469" + ZWJ + "\U0001F4BB",                                   # woman technologist
        "\U0001F468" + ZWJ + "\U0001F469" + ZWJ + "\U0001F467" + ZWJ + "\U0001F466",   # family
        "\U0001F3F3️" + ZWJ + "\U0001F308",                             # rainbow flag
    ]
    for sample in samples:
        result = text.clean(f"look {sample} here")
        assert result["text"] == f"look {sample} here"
        assert result["removed"] == 0


def test_arabic_persian_kurdish_and_indic_joiners_survive():
    samples = [
        "می" + ZWNJ + "خواهم",          # Persian, ZWNJ is required orthography
        "نمی" + ZWNJ + "دانم",          # Persian
        "دەب" + ZWNJ + "ێت",            # Kurdish
        "क" + ZWJ + "ष",                # Devanagari, ZWJ selects a conjunct form
        "क" + ZWNJ + "ष",               # Devanagari, ZWNJ blocks one
        "বা" + ZWNJ + "ংলা",             # Bengali
    ]
    for sample in samples:
        result = text.clean(sample)
        assert result["text"] == sample, f"corrupted {sample!r}"
        assert result["removed"] == 0


def test_ordinary_non_ascii_is_never_called_suspicious():
    for sample in ["café naïve", "中文です", "Ελληνικά", "Здравствуйте", "😀 emoji"]:
        assert keys(text.analyse(sample)) == ["clean"]


# ----------------------------------------------------------------------------------------------
# what must be caught
# ----------------------------------------------------------------------------------------------

def test_real_tracking_characters_are_found_and_removed():
    marked = f"in{ZWSP}voice{BOM} number⁠ one⁣ here⁡"

    result = text.clean(marked)

    assert result["text"] == "invoice number one here"
    assert result["removed"] == 5
    assert set(result["removed_chars"]) == {"U+200B", "U+FEFF", "U+2060", "U+2063", "U+2061"}
    hidden = next(f for f in result["findings"] if f["key"] == "hidden")
    assert hidden["severity"] == "notable"


def test_a_right_to_left_override_is_alarming_and_removed():
    result = text.clean(f"report{RLO}fdp.exe")

    bidi = next(f for f in result["findings"] if f["key"] == "bidi")
    assert bidi["severity"] == "alarming"
    assert "right-to-left override" in bidi["note"]
    assert RLO not in result["text"]


def test_a_plain_direction_mark_is_notable_rather_than_alarming():
    bidi = next(f for f in text.analyse("a‪bc‬") if f["key"] == "bidi")

    assert bidi["severity"] == "notable"


def test_a_cyrillic_letter_hiding_in_a_latin_word_is_reported():
    findings = text.analyse("pаypal is not paypal")          # first "a" is Cyrillic U+0430

    confusable = next(f for f in findings if f["key"] == "confusable")
    assert confusable["severity"] == "alarming"
    assert "Cyrillic" in confusable["value"] and "Latin" in confusable["value"]


def test_a_wholly_cyrillic_word_is_not_a_confusable():
    assert keys(text.analyse("Здравствуйте мир")) == ["clean"]


# ----------------------------------------------------------------------------------------------
# reported, but not silently edited
# ----------------------------------------------------------------------------------------------

def test_odd_spaces_and_word_processor_marks_are_reported_but_left_in_the_prose():
    original = "one two “quoted” and… so on—really"

    result = text.clean(original)

    assert result["text"] == original
    assert result["removed"] == 0
    assert set(keys(result["findings"])) == {"space", "origin"}
    for finding in result["findings"]:
        assert finding["severity"] == "neutral"


def test_clean_text_says_so_plainly():
    findings = text.analyse("Just some ordinary typed words.")

    assert len(findings) == 1
    assert findings[0]["key"] == "clean"
    assert findings[0]["value"] == "Plain text"


def test_empty_text_reports_nothing_at_all():
    assert text.analyse("") == []
    assert text.clean("")["text"] == ""


def test_strip_chars_removes_only_what_it_was_given():
    assert text.strip_chars("abc", []) == "abc"
    assert text.strip_chars("a-b-c", ["-"]) == "abc"
    assert text.strip_chars(f"a{ZWJ}b", [ZWSP]) == f"a{ZWJ}b"


def test_findings_name_the_exact_code_points_so_a_reader_can_check():
    hidden = next(f for f in text.analyse(f"a{ZWSP}b") if f["key"] == "hidden")

    assert "U+200B" in hidden["value"]
    assert hidden["chars"] == [ZWSP]
