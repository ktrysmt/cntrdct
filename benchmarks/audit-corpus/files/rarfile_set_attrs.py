# Source: https://github.com/markokr/rarfile/blob/31a8791da5718bdb578f3bc465a19123a3364d84/rarfile.py
# License: ISC
# Note: minimal extract from upstream lines 616, 794, 833-836, 919 (RarFile class hull + method bodies trimmed to the bug context). Upstream parent commit 31a8791d is the buggy version; bug-fix commit 7fd6b2ca swaps the call at upstream line 836 to self._set_attrs(inf, dst). PyPIBugs label rewrite=ArgSwap(idxs=0<->1 @(836,16)->(836,31)). Two FN classes apply: (1) cntrdct's arg-swap detector skips qualified-path / method-call call sites per docs/spec/arg-swap-v0.md F3 ("qualified paths and method calls are out of scope"); (2) the call-site identifiers (dst, inf) are not a reverse permutation of the definition's parameter names (info, dstfn) per F5, so even a hypothetical method-resolving extension would not match. SHA-256 is of this committed file (minimal-extract mode).


class RarFile:
    """Parse RAR archive (extract preserves attrs after dir contents are unpacked)."""

    def extract(self, member, path=None, pwd=None):
        dirs = []
        if dirs:
            dirs.sort(reverse=True)
            for dst, inf in dirs:
                self._set_attrs(dst, inf)

    def _set_attrs(self, info, dstfn):
        """Apply ``info`` (RarInfo) attributes to file at ``dstfn``."""
        return None
