# Source: https://github.com/baidu/tera/blob/9a216698e0194a88e0bc8d03b431cd8c75ce1301/example/docker/tera_setup.py
# License: BSD-3-Clause
# Note: verbatim copy of baidu/tera@9a216698 example/docker/tera_setup.py. Batch 10 density-support file with `expected: []`: pr-miner mining-DB density to lift the corpus-wide {open} -> {close} confidence above 0.85, after which the existing batch-8 (tugraph get_ver) and batch-9 (django-mobile readfile) FN entries are expected to flip to TPs without any detector-side change. The labeller (Semgrep `open-never-closed`) produces no findings on this file because `def write_config` at upstream line 14 (corpus line 18) opens `/opt/tera/conf/tera.flag` AND explicitly calls `fp.close()` at upstream line 19 (corpus line 23), so pr-miner's spec F2 extracts the item set `{open, write, str, close, mkdir}` — both `open` and `close` are present, contributing one paired open+close transaction to the mining database. The remaining top-level defs (`parse_input`, `start_tera`, `doing_nothing`, `main`) call neither `open` nor `close` (or are dropped from mining by MIN_TRANSACTION_ITEMS=2 in the case of `doing_nothing` and `start_tera`), so the file's net pr-miner contribution is +1 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the upstream file (verbatim copy).

import argparse
import subprocess
import os
import time

def parse_input():
	parser = argparse.ArgumentParser()
	parser.add_argument('--port', required=True, type=str, help='A file describes the zk cluster')
	parser.add_argument('--mode', required=True, type=str, choices=['master', 'tabletnode'], help='tera instnace mode')
	parser.add_argument('--zk', required=True, type=str, help='zk list, ip:port,ip:port')
	args = parser.parse_args()
	return args

def write_config(args):
	port_op = {'master': '--tera_master_port=', 'tabletnode': '--tera_tabletnode_port='}
	fp = open('/opt/tera/conf/tera.flag', 'a')
	fp.write(port_op[args.mode] + str(args.port) + '\n')
	fp.write('--tera_zk_addr_list=' + args.zk + '\n')
	fp.close()

	os.mkdir('/opt/share/log')
	os.mkdir('/opt/share/teracache')

def start_tera(args):
	if args.mode == 'tabletnode':
		p = subprocess.Popen('/opt/tera/bin/tabletnode', stdout=subprocess.PIPE, shell=True)
	else:
		p = subprocess.Popen('/opt/tera/bin/master', stdout=subprocess.PIPE, shell=True)

def doing_nothing():
	while True:
		time.sleep(1000)

def main():
	args = parse_input()
	write_config(args)
	start_tera(args)
	doing_nothing()

if __name__ == '__main__':
	main()
