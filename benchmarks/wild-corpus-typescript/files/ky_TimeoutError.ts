// Source: https://codeload.github.com/sindresorhus/ky/tar.gz/refs/tags/v1.7.2
// License: MIT
// Note: verbatim extract from upstream GitHub release tarball (source/errors/TimeoutError.ts)

import type {KyRequest} from '../types/request.js';

export class TimeoutError extends Error {
	public request: KyRequest;

	constructor(request: Request) {
		super(`Request timed out: ${request.method} ${request.url}`);
		this.name = 'TimeoutError';
		this.request = request;
	}
}
