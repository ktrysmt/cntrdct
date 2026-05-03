#!/usr/bin/env python3
"""
Read a cntrdct JSON findings array from stdin and print the count to stdout.
"""

import json
import sys

print(len(json.load(sys.stdin)))
