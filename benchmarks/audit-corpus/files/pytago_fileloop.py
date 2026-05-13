# Source: https://github.com/nottheswimmer/pytago/blob/3f1bb95b27d92f80677d5c0c208a3c26664fb216/examples/fileloop.py
# License: MIT
# Note: verbatim copy of nottheswimmer/pytago@3f1bb95b examples/fileloop.py. Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The Semgrep `open-never-closed` rule produces no findings on this file because the sole top-level `def main():` opens `file.txt` AND explicitly calls `fh.close()`; the subsequent `with open("file2.txt") as fh2:` and `with open("file3.txt", "rb") as fh3:` blocks are correctly recognized by Semgrep as resource-managed (context-managed file handles). pr-miner's spec F2 extracts the item set `{open, print, close}` for `main`, contributing one paired open+close transaction to the mining database. The file's net pr-miner contribution is +1 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the upstream file (verbatim copy).

def main():
    fh = open("file.txt")
    for line in fh:
        print(line)
    fh.close()

    with open("file2.txt") as fh2:
        for line in fh2:
            print(line)

    with open("file3.txt", "rb") as fh3:
        for l in fh3:
            print(l)


if __name__ == '__main__':
    main()
