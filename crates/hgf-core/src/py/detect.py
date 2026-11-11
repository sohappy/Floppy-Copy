# Floppy-Copy format detector, run with the Python of the Greaseweazle host tools.
# Decodes cylinder 0 of a flux image with every format gw knows and prints one JSON
# object per format on lines starting with "FCDETECT ". Formats ending in .scan and
# the raw.* formats are skipped. For 40-track formats, cylinder 1 is also decoded from
# physical cylinders 1 and 2, to spot a 40-track disk read in an 80-track drive.
import sys, json, io, contextlib
from greaseweazle.codec import codec
from greaseweazle.tools import util
from greaseweazle import track as gtrack

path = sys.argv[1]
only = sys.argv[2].split(',') if len(sys.argv) > 2 and sys.argv[2] else None
plls = gtrack.plls

def out(obj):
    sys.stdout.write('FCDETECT ' + json.dumps(obj) + '\n')
    sys.stdout.flush()

image = util.get_image_class(path).from_file(path, None, {})
cache = {}
def flux(c, h):
    if (c, h) not in cache:
        try:
            cache[c, h] = image.get_track(c, h)
        except Exception:
            cache[c, h] = None
    return cache[c, h]

def decode(dd, cyl, head, phys):
    t = flux(phys, head)
    trk = dd.mk_track(cyl, head)
    if trk is None:
        return None
    if t is None:
        return (0, trk.nsec, trk)
    with contextlib.redirect_stdout(io.StringIO()):
        trk.decode_flux(t)
        for pll in plls[1:]:
            if trk.nr_missing() == 0:
                break
            trk.decode_flux(t, pll)
    return (trk.nsec - trk.nr_missing(), trk.nsec, trk)

formats = only or sorted(codec.get_all_formats('', codec.DiskDef_File(name=None)))
out({'formats': len(formats)})
for i, name in enumerate(formats):
    if name.endswith('.scan') or name.startswith('raw.'):
        continue
    try:
        with contextlib.redirect_stdout(io.StringIO()):
            dd = codec.get_diskdef(name)
        found = total = 0
        boot = ''
        for head in range(dd.heads):
            r = decode(dd, 0, head, 0)
            if r is None:
                continue
            found += r[0]
            total += r[1]
            if head == 0 and r[0] > 0:
                try:
                    boot = bytes(r[2].get_img_track()[:64]).hex()
                except Exception:
                    boot = ''
        res = {'format': name, 'found': found, 'total': total, 'cyls': dd.cyls,
               'heads': dd.heads, 'boot': boot, 'index': i}
        if total and found == total and dd.cyls <= 42:
            one = decode(dd, 1, 0, 1)
            two = decode(dd, 1, 0, 2)
            if one and two:
                res['cyl1_step1'] = one[0]
                res['cyl1_step2'] = two[0]
                res['cyl1_total'] = one[1]
        out(res)
    except Exception as e:
        out({'format': name, 'error': str(e), 'index': i})
out({'done': True})
