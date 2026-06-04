function pipeline(x: number): number {
  const a = transform(x, normalise(x));
  obj.handler.process(a, b);
  return finalise(a);
}
