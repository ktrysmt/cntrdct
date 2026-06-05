// pr-miner negative (Go): every function pairs beginTx/commitTx,
// so no pairing-rule violation should be reported.

package corpus

func txCompute1(x int) int {
	beginTx()
	r := x + 1
	commitTx()
	return r
}

func txCompute2(x int) int {
	beginTx()
	r := x + 2
	commitTx()
	return r
}

func txCompute3(x int) int {
	beginTx()
	r := x + 3
	commitTx()
	return r
}

func txCompute4(x int) int {
	beginTx()
	r := x + 4
	commitTx()
	return r
}

