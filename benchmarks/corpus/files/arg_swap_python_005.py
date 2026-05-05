"""Synthetic fixture: async def fetch(port, host) — swapped call."""

import asyncio


async def fetch(host, port):
    return (host, port)


async def main():
    host = "h"
    port = 80
    _ = await fetch(port, host)


asyncio.run(main())
