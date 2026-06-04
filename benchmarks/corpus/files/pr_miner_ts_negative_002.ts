// pr-miner negative (TypeScript): every function pairs beginTx/commitTx,
// so no pairing-rule violation should be reported.

function txCompute1(x: number): number {
  beginTx();
  const r = x + 1;
  commitTx();
  return r;
}

function txCompute2(x: number): number {
  beginTx();
  const r = x + 2;
  commitTx();
  return r;
}

function txCompute3(x: number): number {
  beginTx();
  const r = x + 3;
  commitTx();
  return r;
}

function txCompute4(x: number): number {
  beginTx();
  const r = x + 4;
  commitTx();
  return r;
}
