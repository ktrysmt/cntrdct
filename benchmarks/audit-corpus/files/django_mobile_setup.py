# Source: https://github.com/gregmuellegger/django-mobile/blob/fafc389057d9dfab5f3c69f7e054dbee8b546f44/setup.py
# License: BSD-3-Clause
# Note: verbatim copy of gregmuellegger/django-mobile@fafc389057d9dfab5f3c69f7e054dbee8b546f44 setup.py. Semgrep registry rule `python.lang.best-practice.open-never-closed.open-never-closed` fires on upstream lines 15 and 17 (audit-corpus lines 19 and 21 after 3-line header + 1 blank): inside the top-level `def readfile(filename):` at upstream line 13 (corpus line 17), both branches of `if sys.version_info[0] >= 3:` return `open(filename, ...).read()` without any matching `close()`, `with`, or try-finally block — the file handle is created, read once, and the only reference is dropped on return. The companion top-level `def get_author(package):` at upstream line 20 (corpus 24) and `def get_version(package):` at upstream line 29 (corpus 33) do not call `open` directly (they delegate to `readfile`), so the rule does not fire on them. The `UltraMagicString` class methods at upstream lines 37-55 are excluded from pr-miner's spec F2 extractor by design (only top-level `function_definition` / `decorated_definition` are walked; class bodies are out of scope). Mapping against cntrdct's pr-miner detector: spec F2 reaches `readfile`, `get_author`, and `get_version` (three top-level defs) and yields item sets `{open, read}`, `{readfile, join, search, group, UltraMagicString}`, and `{readfile, join, search, group}` respectively. With batch 8's `tugraph_det_ver.py` adding `replace_ver` (open+close) and `get_ver` (open-only), batch 9's `readfile` becomes the second open-only Python function in the audit corpus while `replace_ver` remains the only open+close transaction. Spec F3 Apriori mining at `MIN_SUPPORT = 0.05` / `MIN_CONFIDENCE = 0.85` therefore evaluates the `{open} -> {close}` rule against denominator 1 of N transactions (just `replace_ver`), confidence stays at 33% (1 of 3 functions containing `open` also contains `close`), and spec F4 violation detection never runs against `readfile` — FN by mining sparsity, NOT by extractor scope. Closing the 1.00 gap remains a corpus-density problem: this batch adds denominator weight to the same FN-by-sparsity class as batch 8 without changing the detector. SHA-256 is of the upstream file (verbatim copy).

#!/usr/bin/env python
# -*- coding: utf-8 -*-
import re
import os
import sys
from setuptools import setup


README_PATH = os.path.join(os.path.dirname(__file__), 'README.rst')
CHANGES_PATH = os.path.join(os.path.dirname(__file__), 'CHANGES.rst')


def readfile(filename):
    if sys.version_info[0] >= 3:
        return open(filename, 'r', encoding='utf-8').read()
    else:
        return open(filename, 'r').read()


def get_author(package):
    """
    Return package version as listed in `__version__` in `init.py`.
    """
    init_py = readfile(os.path.join(package, '__init__.py'))
    author = re.search("__author__ = u?['\"]([^'\"]+)['\"]", init_py).group(1)
    return UltraMagicString(author)


def get_version(package):
    """
    Return package version as listed in `__version__` in `init.py`.
    """
    init_py = readfile(os.path.join(package, '__init__.py'))
    return re.search("__version__ = ['\"]([^'\"]+)['\"]", init_py).group(1)


class UltraMagicString(object):
    '''
    Taken from
    http://stackoverflow.com/questions/1162338/whats-the-right-way-to-use-unicode-metadata-in-setup-py
    '''
    def __init__(self, value):
        self.value = value

    def __str__(self):
        return self.value

    def __unicode__(self):
        return self.value.decode('UTF-8')

    def __add__(self, other):
        return UltraMagicString(self.value + str(other))

    def split(self, *args, **kw):
        return self.value.split(*args, **kw)


if sys.version_info[0] >= 3:
    long_description = u'\n\n'.join((
        readfile(README_PATH),
        readfile(CHANGES_PATH),
    ))
else:
    long_description = u'\n\n'.join((
        readfile(README_PATH).decode('utf-8'),
        readfile(CHANGES_PATH).decode('utf-8'),
    ))
    long_description = long_description.encode('utf-8')
    long_description = UltraMagicString(long_description)


setup(
    name='django-mobile',
    version=get_version('django_mobile'),
    url='https://github.com/gregmuellegger/django-mobile',
    license='BSD',
    description=u'Detect mobile browsers and serve different template flavours to them.',
    long_description=long_description,
    author=get_author('django_mobile'),
    author_email='gregor@muellegger.de',
    keywords='django,mobile',
    classifiers=[
        'Development Status :: 4 - Beta',
        'Environment :: Web Environment',
        'Framework :: Django',
        'Intended Audience :: Developers',
        'License :: OSI Approved :: BSD License',
        'Natural Language :: English',
        'Operating System :: OS Independent',
        "Programming Language :: Python :: 2.6",
        "Programming Language :: Python :: 2.7",
        "Programming Language :: Python :: 3.3",
        "Topic :: Internet :: WWW/HTTP",
        "Topic :: Internet :: WWW/HTTP :: Dynamic Content",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    packages=[
        'django_mobile',
        'django_mobile.cache',
    ],
    tests_require=['Django', 'mock'],
    test_suite='django_mobile_tests.runtests.runtests',
)
