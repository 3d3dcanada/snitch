from snitch import cli, survival


def test_every_platform_cell_has_an_explicit_evidence_class():
    layer_keys = {key for key, _label, _description in survival.LAYERS}
    assert len(survival.PLATFORMS) == 7
    for platform, layers in survival.PLATFORMS.items():
        assert set(layers) == layer_keys, platform
        for layer, record in layers.items():
            assert record["verdict"] in survival.SYMBOL, (platform, layer)
            assert record["evidence"] in survival.EVIDENCE_CLASSES, (platform, layer)
            assert record["note"], (platform, layer)
            if record["evidence"] != survival.INFERENCE:
                assert record["sources"], (platform, layer)


def test_inferences_are_machine_readable_as_unverified():
    report = survival.as_dict(include_notes=True)

    linkedin_exif = report["platforms"]["LinkedIn"]["exif"]
    assert linkedin_exif["evidence"] == "inference"
    assert linkedin_exif["live_tested"] is False
    assert linkedin_exif["verdict"] == "unknown"
    assert report["platforms"]["LinkedIn"]["c2pa"]["live_tested"] is False


def test_table_visibly_marks_unverified_cells_and_prints_sources(capsys):
    cli.print_platforms(notes=True)

    output = capsys.readouterr().out
    assert "researched 2026-08-25" in output
    assert "? unverified" in output
    assert "? keeps" in output
    assert "D partial" in output
    assert "[inference]" in output
    assert "[documented]" in output
    assert "https://www.linkedin.com/help/linkedin/answer/a6282984" in output
    assert "verified 2026-08-25" not in output


def test_google_images_is_not_misrepresented_as_an_upload_round_trip():
    google = survival.PLATFORMS["Google Images"]

    assert google["iptc_xmp"]["verdict"] == survival.READS
    assert "crawls" in google["iptc_xmp"]["note"]
    assert "not upload survival" in google["c2pa"]["note"]


def test_linkedin_self_signed_display_is_not_claimed_as_verified():
    linkedin = survival.PLATFORMS["LinkedIn"]["c2pa"]

    assert linkedin["verdict"] == survival.PARTIAL
    assert "self-signed" in linkedin["note"]
    assert "unverified" in survival.one_line_advice()
