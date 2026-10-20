"""Repack explicitly owned animation arrays inside existing bank storage.

Never grows the bank or interprets unknown outer header fields. Rejects shared,
interior, overlapping, or object-backed buffers and nonzero alignment gaps.
"""
import bisect
import struct


def repack(bank,replacements):
    entries=[]
    for obj in bank.by_id.values():
        p=obj['offset']
        if obj['type_hash']==3842202299 and obj['size']==104:
            arrays=[(64,2*struct.unpack_from('<I',bank.blob,p+72)[0]),(80,struct.unpack_from('<I',bank.blob,p+88)[0])]
        elif obj['type_hash']==1531114270 and obj['size']==184:
            arrays=[(136,4*struct.unpack_from('<Q',bank.blob,p+144)[0]),(152,2*struct.unpack_from('<Q',bank.blob,p+160)[0]),(168,struct.unpack_from('<Q',bank.blob,p+176)[0])]
        else:continue
        for field,size in arrays:
            site=p+field
            if not size and site not in replacements:continue
            at=bank.pointer(site) if size else bank.reloc.get(site,0)
            alignment=2 if field in (64,152) else (4 if field==136 else 16)
            entries.append(dict(site=site,start=at,size=size,alignment=alignment,data=replacements.get(site,bank.blob[at:at+size])))
    by_site={e['site']:e for e in entries}
    if not set(replacements)<=set(by_site):raise ValueError('Replacement has no existing allocation')
    targets=sorted((dst,src) for src,dst in bank.relocations)
    objects=sorted((o['offset'],o['offset']+o['size']) for o in bank.by_id.values())
    object_starts=[x[0] for x in objects]
    eligible=[];skipped=[]
    for e in entries:
        if not e['size']:
            eligible.append(e);continue
        a=e['start'];z=a+e['size'];i=bisect.bisect_left(targets,(a,-1));refs=[]
        while i<len(targets) and targets[i][0]<z:refs.append(targets[i]);i+=1
        j=max(0,bisect.bisect_right(object_starts,a)-1)
        overlaps=False
        while j<len(objects) and objects[j][0]<z:
            overlaps|=objects[j][1]>a;j+=1
        if refs!=[(a,e['site'])] or overlaps:
            if e['site'] in replacements:raise ValueError('Replacement buffer is not exclusive')
            skipped.append(e);continue
        eligible.append(e)
    eligible.sort(key=lambda e:e['start']);regions=[]
    protected_points=sorted(set(bank.names)|{v for pair in bank.relocations for v in pair})
    protected_spans=sorted(objects+[(e['start'],e['start']+e['size']) for e in skipped])
    protected_starts=[a for a,z in protected_spans]
    for e in eligible:
        if not e['size']:continue
        a=e['start'];z=a+e['size']
        if regions:
            end=regions[-1][1]
            if a<end:raise ValueError('Overlapping animation allocations')
            # Reclaim alignment bytes only when they contain no pointers, names,
            # registered objects, or data from an excluded allocation.
            gap_ok=0<=a-end<=15 and not any(bank.blob[end:a])
            if gap_ok and a>end:
                i=bisect.bisect_left(protected_points,end)
                gap_ok=i==len(protected_points) or protected_points[i]>=a
                j=max(0,bisect.bisect_right(protected_starts,end)-1)
                while j<len(protected_spans) and protected_spans[j][0]<a:
                    gap_ok &= protected_spans[j][1]<=end;j+=1
            if gap_ok:regions[-1]=(regions[-1][0],z);continue
        regions.append((a,z))
    free=list(regions);placements={}
    for e in sorted(eligible,key=lambda e:(-len(e['data']),e['site'])):
        choices=[]
        for i,(a,z) in enumerate(free):
            alignment=e['alignment'];aligned=(a+alignment-1)&~(alignment-1)
            if aligned+len(e['data'])<=z:choices.append((z-aligned-len(e['data']),i,aligned))
        if not choices:raise ValueError(f"Existing pool cannot fit {len(e['data'])} bytes at site {e['site']}")
        _,i,at=min(choices);a,z=free.pop(i);end=at+len(e['data'])
        if a<at:free.append((a,at))
        if end<z:free.append((end,z))
        placements[e['site']]=at
    output=bytearray(bank.data);base=bank.sections['blob'][0]
    for a,z in regions:output[base+a:base+z]=bytes(z-a)
    for e in eligible:
        at=placements[e['site']];output[base+at:base+at+len(e['data'])]=e['data']
    reloc_base=bank.sections['relocations'][0]
    for i,(site,old) in enumerate(bank.relocations):
        if site in placements:struct.pack_into('<Q',output,reloc_base+i*16+8,placements[site])
    # Check every relocated buffer, including all unchanged animation payloads.
    for e in eligible:
        at=placements[e['site']]
        if output[base+at:base+at+len(e['data'])]!=e['data']:raise ValueError('Pool verification failed')
    additions=[(site,at) for site,at in placements.items() if site not in bank.reloc]
    if additions:
        at=bank.sections['objects'][0]
        output[at:at]=b''.join(struct.pack('<QQ',*pair) for pair in additions)
        struct.pack_into('<Q',output,bank.header+40,len(bank.relocations)+len(additions))
    return output,dict(allocations=len(eligible),relocated=sum(placements[e['site']]!=e['start'] for e in eligible),
                       regions=len(regions),free_bytes=sum(z-a for a,z in free),added_relocations=len(additions),
                       unchanged_blob_size=True,unchanged_bank_size=len(output)==len(bank.data))
