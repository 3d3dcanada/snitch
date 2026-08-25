# Quality gates — 2026-08-25

## Added gates

- Ruff linting with an explicit Python 3.9 target and selected correctness rules.
- Mypy checking of every function body in `snitch/` against the declared Python 3.9 target.
- Pytest on the complete suite.
- Isolated sdist/wheel build, Twine metadata validation, and installed-wheel CLI smoke tests.
- GitHub Actions jobs for Ubuntu/Python 3.9, Ubuntu/current Python 3.14, macOS, and Windows.
  ExifTool is installed on each runner before the command tests.

The workflow actions are pinned to current immutable commits. The workflow was statically checked;
it has not run on GitHub because this audit did not authorize a push.

## Observed local results

```text
$ ruff check snitch tests
All checks passed!

$ mypy snitch
Success: no issues found in 6 source files

$ pytest -q
..............................................................           [100%]
62 passed in 4.09s

$ python -m build --outdir /tmp/.../dist-quality2
Successfully built snitch_tools-0.1.0.tar.gz and snitch_tools-0.1.0-py3-none-any.whl

$ python -m twine check /tmp/.../dist-quality2/*
snitch_tools-0.1.0-py3-none-any.whl: PASSED
snitch_tools-0.1.0.tar.gz: PASSED

$ /tmp/.../wheel-smoke/bin/snitch --version
snitch 0.1.0
$ /tmp/.../wheel-smoke/bin/python -m snitch --version
snitch 0.1.0

$ go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.12 .github/workflows/quality.yml
exit 0
```

The first build exposed deprecated setuptools licence metadata. `pyproject.toml` now uses an SPDX
licence expression; a second isolated build completed without warnings.
