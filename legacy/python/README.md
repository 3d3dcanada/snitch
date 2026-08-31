# The Python implementation, retired

This is the original `snitch-tools` Python package, kept verbatim. It is not deleted, and it is not
the thing that ships: the tool is Rust, at the root of this repository.

It is here for two reasons.

**It is the specification.** The Rust port is checked against it byte for byte. `snitch`,
`no-comment` and `credit` produce identical terminal output, identical JSON, identical exit codes
and identical output files across the fixture matrix. If you change behaviour in the Rust, this is
what tells you whether you meant to.

**It is the research.** `snitch/survival.py` is where the platform table was written and sourced.
The Rust reads `data/survival.json`, which was exported from it verbatim rather than retyped.

To run it:

```bash
cd legacy/python
python -m pip install -e '.[dev]'
pytest -q
```

Nothing new should be built here.
