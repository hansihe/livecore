#!/bin/bash
set -e

cargo run --bin gen_schema --features="jsonschema"
