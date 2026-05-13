# Source: https://github.com/carljm/django-secure/blob/2080b41f34e9c5fa19d4a8e9d566e13b8445b3cd/setup.py
# License: BSD-3-Clause
# Note: verbatim copy of carljm/django-secure@2080b41f setup.py. Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The module-level `long_description = (open(...).read() + ...)` chain at upstream lines 7-9 is NOT extracted by pr-miner's spec F2 because it sits outside any `function_definition` node; F2 walks only top-level `def` and `decorated_definition` children of the root. The Semgrep `open-never-closed` rule similarly does not fire on the module-level open chain because the rule's main pattern targets `$F = open(...)` followed by no close in a control-flow path, not the read-and-discard chain shape `open(...).read()`. The single top-level `def get_version()` opens `__init__.py` in a try/finally with `fh.close()` in finally, so pr-miner's F2 extracts item set `{open, join, readlines, startswith, split, strip, close}` for it — both `open` and `close` present, contributing one paired open+close transaction to the mining database. The file's net pr-miner contribution is +1 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the upstream file (verbatim copy).

from os.path import join, dirname

from setuptools import setup, find_packages

here = dirname(__file__)

long_description = (open(join(here, "README.rst")).read() + "\n\n" +
                    open(join(here, "CHANGES.rst")).read() + "\n\n" +
                    open(join(here, "TODO.rst")).read())

def get_version():
    fh = open(join(here, "djangosecure", "__init__.py"))
    try:
        for line in fh.readlines():
            if line.startswith("__version__ ="):
                return line.split("=")[1].strip().strip('"')
    finally:
        fh.close()

setup(
    name="django-secure",
    version=get_version(),
    description="Utilities and a 'linter' to help you make your Django site more secure.",
    long_description=long_description,
    author="Carl Meyer",
    author_email="carl@oddbird.net",
    url="https://github.com/carljm/django-secure/",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Environment :: Web Environment",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: BSD License",
        "Operating System :: OS Independent",
        "Programming Language :: Python",
        "Programming Language :: Python :: 2",
        "Programming Language :: Python :: 2.6",
        "Programming Language :: Python :: 2.7",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.2",
        "Programming Language :: Python :: 3.3",
        "Framework :: Django",
    ],
    zip_safe=False,
    install_requires=["Django>=1.4.2"],
    test_suite="runtests.runtests"
)
