"""Resolve native attributes by declaration source slot, never by name heuristics."""
def resolve_fields(profile, submesh):
    declaration=profile['formats'][submesh['format']]['declaration']
    result={}
    for token in declaration.split():
        usage,offset,slot,*rest=token.split(':')
        slot=int(slot,16);offset=int(offset,16)
        if slot>=len(submesh['streams']):raise ValueError('attribute source slot outside submesh streams')
        stream_index=submesh['streams'][slot]
        stream=profile['layout']['streams'][stream_index]
        encoding=rest[-1]
        size={'3f32':12,'4f16':8,'2f16':4,'4u8n':4}.get(encoding)
        if size is None or offset+size>stream['stride']:raise ValueError('attribute encoding/extent not qualified')
        if usage in result:raise ValueError('duplicate attribute usage')
        result[usage]={'source_slot':slot,'stream_index':stream_index,'offset':offset,
                       'data_off':stream['data'],'stride':stream['stride'],'encoding':encoding}
    for usage,encoding in [('p0','3f32'),('n0','4f16'),('g0','4f16'),('t0','2f16')]:
        if usage not in result or result[usage]['encoding']!=encoding:raise ValueError('required attribute format mismatch')
    return result
