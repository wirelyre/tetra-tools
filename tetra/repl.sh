#!/bin/sh

./.venv/bin/maturin develop
./.venv/bin/python -i -c "import tetra"
