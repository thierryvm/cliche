"""Lit le manifeste Windows embarque dans l'exe contenu par un installeur NSIS,
SANS installer ni lancer quoi que ce soit.

Pourquoi ce detour : l'artefact publie par la CI est un installeur NSIS, et le
binaire dont le manifeste doit etre controle est a l'interieur, compresse. Le
manifeste du STUB NSIS, lui, est en clair au debut du fichier - le chercher la
donnerait le mauvais manifeste et un faux OK. On ne lit donc que le flux
DECOMPRESSE.

Trois compressions possibles, essayees dans cet ordre :

  - LZMA1 BRUT. C'est le defaut de NSIS en mode solide, et c'est le piege : NSIS
    ecrit les 5 octets de proprietes LZMA puis les donnees, SANS les 8 octets de
    taille que reclame le format « alone ». Un decodeur ALONE echoue donc a coup
    sur - premiere version de ce script, 0 octet decompresse.
  - bzip2, dans la variante NSIS (l'entete « BZh » n'est pas ecrit).
  - deflate brut (zlib sans entete).
"""

import bz2
import lzma
import re
import sys
import zlib

LIMIT = 64 * 1024 * 1024


def _drain(dec, data: bytes, off: int) -> bytes:
    """Ce qu'un decodeur arrive a produire avant de caler."""
    out = bytearray()
    pos = off
    try:
        while pos < len(data) and len(out) < LIMIT:
            if getattr(dec, "eof", False):
                break
            out += dec.decompress(data[pos : pos + 65536])
            pos += 65536
    except Exception:
        pass  # le flux n'a pas de fin propre ici ; seul son contenu compte
    return bytes(out)


def try_lzma_raw(data: bytes, off: int) -> bytes:
    """LZMA1 brut : 5 octets de proprietes, puis les donnees."""
    if off + 5 > len(data):
        return b""
    props = data[off]
    if props >= 9 * 5 * 5:
        return b""
    dict_size = int.from_bytes(data[off + 1 : off + 5], "little")
    if not (1 << 12) <= dict_size <= (1 << 30):
        return b""
    lc = props % 9
    lp = (props // 9) % 5
    pb = (props // 9) // 5
    try:
        dec = lzma.LZMADecompressor(
            format=lzma.FORMAT_RAW,
            filters=[
                {
                    "id": lzma.FILTER_LZMA1,
                    "dict_size": dict_size,
                    "lc": lc,
                    "lp": lp,
                    "pb": pb,
                }
            ],
        )
    except Exception:
        return b""
    return _drain(dec, data, off + 5)


def try_deflate(data: bytes, off: int) -> bytes:
    return _drain(zlib.decompressobj(-15), data, off)


def try_bzip2(data: bytes, off: int) -> bytes:
    return _drain(bz2.BZ2Decompressor(), data, off)


def main(path: str) -> int:
    data = open(path, "rb").read()
    print(f"installeur : {len(data)} octets")

    sig = data.find(b"NullsoftInst")
    if sig < 0:
        print("ECHEC : signature NullsoftInst absente, ce n'est pas un NSIS")
        return 2
    print(f"signature NullsoftInst a l'offset {sig}")

    base = sig + 12 + 8  # magic(12) + header_size(4) + data_size(4)
    best, best_off, best_how = b"", -1, ""
    for how, fn in (("lzma1-brut", try_lzma_raw), ("deflate", try_deflate), ("bzip2", try_bzip2)):
        for off in range(max(0, base - 32), base + 256):
            out = fn(data, off)
            if len(out) > len(best):
                best, best_off, best_how = out, off, how
        if len(best) > 1_000_000:
            break

    if len(best) < 100_000:
        print(f"ECHEC : rien de consequent decompresse (max {len(best)} octets)")
        return 3
    print(f"flux {best_how} a l'offset {best_off} : {len(best)} octets decompresses")

    manifests = re.findall(rb"<assembly[\s\S]{0,20000}?</assembly>", best)
    if not manifests:
        print("ECHEC : aucun manifeste dans le flux decompresse")
        return 4

    print(f"{len(manifests)} manifeste(s) DANS LE FLUX (donc pas celui du stub)")
    verdict = 0
    for i, m in enumerate(manifests, 1):
        bad = [b for b in m if b > 0x7F]
        print(f"\n--- manifeste {i} : {len(m)} octets ---")
        print(f"  octets non-ASCII : {len(bad)}")
        for name in (b"dpiAware", b"dpiAwareness", b"longPathAware"):
            print(f"  {name.decode():14} : {'present' if name in m else 'ABSENT'}")
        if bad:
            print("  >>> ROUGE : un octet non-ASCII ici donne os error 14001 au lancement")
            verdict = 5
    return verdict


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
