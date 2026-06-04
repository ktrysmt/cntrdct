// Source: https://codeload.github.com/colinhacks/zod/tar.gz/refs/tags/v3.23.8
// License: MIT
// Note: verbatim extract from upstream GitHub release tarball (src/errors.ts)

import defaultErrorMap from "./locales/en";
import type { ZodErrorMap } from "./ZodError";

let overrideErrorMap = defaultErrorMap;
export { defaultErrorMap };

export function setErrorMap(map: ZodErrorMap) {
  overrideErrorMap = map;
}

export function getErrorMap() {
  return overrideErrorMap;
}
