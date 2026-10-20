"""Qualified cached PTM-active byte translation in the native post-scene update."""
CACHE_NORMALIZATION_SITE=0x521ab2
CACHE_NORMALIZATION_BEFORE=bytes.fromhex('0f95c0')
CACHE_NORMALIZATION_AFTER=bytes.fromhex('909090')
# Source 527d96..527da2 copies the raw Boolean into feature+68 -> flags+e6.
# Target 521aa8..521abd caches the same asset at flags+ce, but SETNE normalizes.
# Preserve the raw byte for the port's exact ==1 source gates; existing target
# consumers retain their original nonzero tests and therefore see no change.
def patches(image):
    if image.read(0x521aa8,0x15)!=bytes.fromhex('8a0084c0898560ffffff0f95c041888424ce000000'):
        raise ValueError('Native PTM cache producer changed')
    return [(CACHE_NORMALIZATION_SITE,CACHE_NORMALIZATION_BEFORE,CACHE_NORMALIZATION_AFTER)]
