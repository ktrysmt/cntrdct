// pr-miner positive (Go): the beginTx/commitTx pairing rule is
// violated by the final function, which calls beginTx() but never commitTx().

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

func txCompute5(x int) int {
	beginTx()
	r := x + 5
	commitTx()
	return r
}

func txCompute6(x int) int {
	beginTx()
	r := x + 6
	commitTx()
	return r
}

func txCompute7(x int) int {
	beginTx()
	r := x + 7
	commitTx()
	return r
}

func txMissingCommit004() {
	beginTx()
	prMinerGo004Marker()
}
