// pr-miner positive (TypeScript): the beginTx/commitTx pairing rule is
// violated by the final function, which calls beginTx() but never commitTx().

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

function txCompute5(x: number): number {
  beginTx();
  const r = x + 5;
  commitTx();
  return r;
}

function txCompute6(x: number): number {
  beginTx();
  const r = x + 6;
  commitTx();
  return r;
}

function txCompute7(x: number): number {
  beginTx();
  const r = x + 7;
  commitTx();
  return r;
}

function txMissingCommit008(): void {
  beginTx();
  prMinerTs008Marker();
}
