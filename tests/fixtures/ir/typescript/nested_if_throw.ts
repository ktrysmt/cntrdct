function guard(x: number): number {
  if (x > 0) {
    return x;
  } else {
    throw new Error("non-positive");
  }
  unreachable();
}
