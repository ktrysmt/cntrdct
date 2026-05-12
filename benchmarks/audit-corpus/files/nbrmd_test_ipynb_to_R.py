# Source: https://github.com/mwouts/nbrmd/blob/4ecf608f0ed2ec2bd1dfaa2c4cdc3b08f2072a46/tests/test_ipynb_to_R.py
# License: MIT
# Note: verbatim copy at parent commit 4ecf608f (buggy version); bug-fix commit dfa96996 swaps the call at upstream line 22 to compare_notebooks(nb2, nb1). PyPIBugs label rewrite=ArgSwap(idxs=0<->1 @(22,4)->(22,21)). Audit-corpus line 26 = upstream line 22 + 4 (3-line header + 1 blank). FN by cross-file resolution: compare_notebooks is imported from jupytext.compare (upstream line 5); cntrdct's arg-swap detector resolves only same-file definitions per docs/spec/arg-swap-v0.md F4.

import nbformat
import itertools
import pytest
import jupytext
from jupytext.compare import compare_notebooks
from .utils import list_notebooks


@pytest.mark.parametrize('nb_file,ext', itertools.product(list_notebooks('ipynb_R'), ['.r', '.R']))
def test_identity_source_write_read(nb_file, ext):
    """
    Test that writing the notebook with R, and read again,
    is the same as removing outputs
    """

    with open(nb_file) as fp:
        nb1 = nbformat.read(fp, as_version=4)

    R = jupytext.writes(nb1, ext)
    nb2 = jupytext.reads(R, ext)

    compare_notebooks(nb1, nb2)
