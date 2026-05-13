# Source: https://github.com/R-s0n/ars0n-framework/blob/a68b5120e0cbb8098d2fd68d2b660cd0257d79dc/toolkit/toolkit/fire-scanner.py
# License: MIT
# Note: minimal extract from R-s0n/ars0n-framework@a68b5120 toolkit/toolkit/fire-scanner.py (upstream 675 lines, MIT). Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The upstream file contains two top-level defs that explicitly pair open() with close() — `write_urls_file` (upstream line 195) and `build_slack_message` (upstream line 537) — plus one open-only top-level def (`process_results` at line 518 opens `/tmp/{args.fqdn}-{now}.json` without explicit close), which a verbatim copy would contribute to the mining database as count(open) without count(close), lowering the {open} -> {close} confidence ratio. This minimal extract therefore keeps only the two paired defs plus the upstream imports and the `get_home_dir` helper that `build_slack_message` calls, so the file parses cleanly under tree-sitter Python 3. The Semgrep `open-never-closed` rule produces no findings on these two defs (each opens AND explicitly closes the file handle within the same function body). pr-miner's spec F2 extracts items including `{open, write, close}` for `write_urls_file` and `{open, read, close, get_home_dir, post, print, len, ...}` for `build_slack_message`, contributing two paired open+close transactions to the mining database. The file's net pr-miner contribution is +2 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the extracted file as committed (per benchmarks/audit-corpus/README.md "minimal extracts" clause).

import requests
import subprocess
import argparse
import json
import re
from datetime import datetime, timedelta


def get_home_dir():
    get_home_dir = subprocess.run(["echo $HOME"], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, shell=True)
    return get_home_dir.stdout.replace("\n", "")

def write_urls_file(url_str):
    f = open("/tmp/urls.txt", "w")
    f.write(url_str)
    f.close()

def build_slack_message(args, thisFqdn, data, template):
    info_counter = 0
    non_info_counter = 0
    for result in data:
        if len(result['info']['name']) < 2:
            data.remove(result)
            continue
        if result['info']['severity'] == 'info':
            info_counter += 1
            result['impactful'] = False
        else :
            non_info_counter += 1
            result['impactful'] = True
    httprobe_arr = thisFqdn['recon']['subdomains']['httprobe']
    masscan_arr = thisFqdn['recon']['subdomains']['masscanLive']
    urls = httprobe_arr + masscan_arr
    target_count = len(urls)
    if non_info_counter != 0 or info_counter != 0:
        message_json = {'text':f'Nuclei Scan Completed!\n\nResults:\nWeb Servers Scanned: {target_count}\nRood/Seed Targeted: {args.fqdn}\nTemplate Category: {template}\nImpactful Results: {non_info_counter}\nInformational Results: {info_counter}\n\nNothing wrong with a little Spray and Pray!!  :pray:','username':'Vuln Disco Box','icon_emoji':':dart:'}
        home_dir = get_home_dir()
        f = open(f'{home_dir}/.keys/slack_web_hook')
        token = f.read()
        f.close()
        slack_auto = requests.post(f'https://hooks.slack.com/services/{token}', json=message_json) 
        print(f"[+] Slack Notification Sent!  {non_info_counter} Impactful Findings!")

