#!/bin/sh

# ./.venv/bin/maturin develop
./.venv/bin/maturin develop --release
./.venv/bin/python -i -c "import tetra"
