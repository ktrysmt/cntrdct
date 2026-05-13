# Source: https://github.com/dkruchinin/sanic-prometheus/blob/0cebb0badefd20603087036aa0b34f224e582269/scripts/release.py
# License: MIT
# Note: verbatim copy of dkruchinin/sanic-prometheus@0cebb0ba scripts/release.py. Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The Semgrep `open-never-closed` rule produces no findings on this file because both top-level defs that call `open` also explicitly close: `def get_version()` opens `VERSION` in a try/finally with `fh.close()` in finally, and `def update_changelog(version, msg)` opens `CHANGELOG.rst.tmp` in a try/finally with `wfh.close()` in finally. pr-miner's spec F2 extracts the item set `{open, read, strip, close}` for `get_version` and `{today, open, write, move, close}` for `update_changelog`, contributing two paired open+close transactions to the mining database. The third top-level def `main` calls neither `open` nor `close`. The file's net pr-miner contribution is +2 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the upstream file (verbatim copy).

#!/usr/bin/env python3

import datetime
import os
import shutil
import sys


def get_version():
    fh = open('VERSION', 'r')
    try:
        return fh.read().strip()
    finally:
        fh.close()


def update_changelog(version, msg):
    today = datetime.date.today()
    wfh = open('CHANGELOG.rst.tmp', 'w')
    try:
        lines_count = 0
        for line in open('CHANGELOG.rst', 'r'):
            lines_count += 1
            if lines_count == 4:
                wfh.write(f'Version {version} (on {today: %b %d %Y})\n')
                wfh.write('-------------------------------\n')
                wfh.write(f'* {msg}')
                wfh.write('\n\n')
            wfh.write(line)
    finally:
        wfh.close()
        shutil.move('CHANGELOG.rst.tmp', 'CHANGELOG.rst')


def main():
    version = get_version()
    print(f'New version is {version}')
    print('Creating archives ...')
    os.system('python3 setup.py sdist bdist_wheel')
    print('Updating the changelog ...')
    changelog_msg = input("Please enter a changelog message: ")
    if changelog_msg == "":
        print("ERROR: You didn't enter a changelog message!")
        sys.exit(-1)

    update_changelog(version, changelog_msg)
    print('Uploading the new release...')
    os.system('twine upload dist/*')


if __name__ == '__main__':
    main()
