// Source: https://codeload.github.com/colinhacks/zod/tar.gz/refs/tags/v3.23.8
// License: MIT
// Note: verbatim extract from upstream GitHub release tarball (src/helpers/errorUtil.ts)

export namespace errorUtil {
  export type ErrMessage = string | { message?: string };
  export const errToObj = (message?: ErrMessage) =>
    typeof message === "string" ? { message } : message || {};
  export const toString = (message?: ErrMessage): string | undefined =>
    typeof message === "string" ? message : message?.message;
}
