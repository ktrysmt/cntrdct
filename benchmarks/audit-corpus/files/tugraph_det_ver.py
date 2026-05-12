# Source: https://github.com/TuGraph-family/tugraph-db/blob/672e4b1998b78e5dbd45ae44950e86b48c841437/release/det_ver.py
# License: Apache-2.0
# Note: verbatim copy of TuGraph-family/tugraph-db@672e4b1998b78e5dbd45ae44950e86b48c841437 release/det_ver.py. Semgrep registry rule `python.lang.best-practice.open-never-closed.open-never-closed` fires on upstream line 6 (audit-corpus line 10 after 3-line header + 1 blank + leading `#!/usr/bin/env python` at upstream line 1 → corpus line 5): inside the top-level `def get_ver():` at upstream line 5 (corpus line 9), `f = open('Options.cmake','r')` is followed by `f.readlines()` and a `return` without any matching `f.close()`, `with`, or try-finally block. The companion top-level `def replace_ver(...)` at upstream line 17 (corpus line 21) opens at upstream line 18 (corpus line 22) but DOES close at upstream line 29 (corpus line 33), so it does not trigger the rule. Mapping against cntrdct's pr-miner detector: spec F2 `function_definition` extractor reaches `get_ver` (top-level def) and yields the item set `{open, readlines, find, split}`; spec F3 Apriori mining at `MIN_SUPPORT = 0.05` / `MIN_CONFIDENCE = 0.85` cannot synthesise the `{open} → {close}` rule from the single corpus-wide transaction (replace_ver) that contains both items, so spec F4 violation detection never runs against `get_ver` and the bug stays unflagged — FN by mining sparsity (single supporting transaction), not by extractor scope. SHA-256 is of the upstream file (verbatim copy).

#!/usr/bin/env python
# This script should be processed under root
import re

def get_ver():
    f = open('Options.cmake','r')
    lines = f.readlines()
    for line in lines:
        if line.find('LGRAPH_VERSION_MAJOR') > 0:
            ver_major = line.split(" ")[-1].split(")")[0]
        if line.find('LGRAPH_VERSION_MINOR') > 0:
            ver_minor = line.split(" ")[-1].split(")")[0]
        if line.find('LGRAPH_VERSION_PATCH') > 0:
            ver_patch = line.split(" ")[-1].split(")")[0]
    return ver_major + '.' + ver_minor + '.' + ver_patch

def replace_ver(file_name, pattern, curr_ver):
    f = open(file_name, 'r+')
    lines = f.readlines()
    f.seek(0,0)
    for line in lines:
        if pattern in line:
            print("updating %s" % file_name)
            new_line = re.sub(r'[0-9]+\.[0-9]+\.[0-9]+', curr_ver, line)
            print(new_line)
            f.write(new_line)
        else:
            f.write(line)
    f.close()

curr_ver = get_ver()
print("current version: %s" % curr_ver)
replace_ver('docs/autogen/TuGraph-Python-Procedure-API/index.rst', 'Version: ', curr_ver)
replace_ver('docs/autogen/TuGraph-CPP-Procedure-API/Doxyfile', 'PROJECT_NUMBER         = ', curr_ver)
replace_ver('docs/en-US/source/5.developer-manual/6.interface/3.procedure/4.Python-procedure.rst', 'Version: ', curr_ver)
replace_ver('docs/en-US/source/5.developer-manual/6.interface/3.procedure/Doxyfile', 'PROJECT_NUMBER         = ', curr_ver)
replace_ver('docs/zh-CN/source/5.developer-manual/6.interface/3.procedure/4.Python-procedure.rst', 'Version: ', curr_ver)
replace_ver('docs/zh-CN/source/5.developer-manual/6.interface/3.procedure/Doxyfile', 'PROJECT_NUMBER         = ', curr_ver)
replace_ver('docs/zh-CN/source/1.guide.md', '安装', curr_ver)
replace_ver('docs/en-US/source/1.guide.md', 'runtime', curr_ver)

dockerfiles = [
    "tugraph-mini-runtime-centos7-Dockerfile",
    "tugraph-mini-runtime-centos8-Dockerfile",
    "tugraph-mini-runtime-ubuntu18.04-Dockerfile",
    "tugraph-runtime-centos7-Dockerfile",
    "tugraph-runtime-centos8-Dockerfile",
    "tugraph-runtime-ubuntu18.04-Dockerfile"
]
for file in dockerfiles:
    replace_ver('ci/images/' + file, 'COPY', curr_ver)
    replace_ver('ci/images/' + file, 'RUN dpkg', curr_ver)
    replace_ver('ci/images/' + file, 'RUN rpm', curr_ver)
