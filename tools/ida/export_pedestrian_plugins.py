"""Read original generated pedestrian plugin tables; no game execution."""
import json
from pathlib import Path
import ida_bytes as b
import idautils

FAMILIES=('actortracker_spectate','conversation','lookat','sit','spectate','useatm','usenewspaperbox','usetrashbin','usevendingmachine','usewaterfountain')
def export(destination):
    names=list(idautils.Names())
    def text(a):
        value=b.get_strlit_contents(b.get_dword(a),-1,0)
        return value.decode('utf-8') if value else ''
    def sec(a):return dict(count=b.get_word(a),index=b.get_word(a+2))
    result={}
    for family in FAMILIES:
        addresses={n.split('@')[0][2:]:a for a,n in names if n.startswith('?k') and '@HCDelegate_'+family+'@' in n}
        def rows(key,stride):
            a=addresses[key]
            return range(a,a+b.get_item_size(a),stride)
        data=dict(addresses=addresses)
        data['attributes']=[dict(key=text(a),value=text(a+4)) for a in rows('Attribs',8)]
        data['indices']=[b.get_word(a) for a in rows('IndexArray',2)]
        data['states']=[dict(name=text(a),interrupt_type=b.get_byte(a+4),active=b.get_byte(a+5)!=0,interrupt_target=b.get_word(a+6),parent=b.get_word(a+8),expression=b.get_word(a+10),children=sec(a+12),behaviours=sec(a+16),transitions=sec(a+20),update_address=b.get_dword(a+24),trailing=b.get_dword(a+28)) for a in rows('States',32)]
        data['transitions']=[dict(name=text(a),priority=b.get_byte(a+4),target=b.get_word(a+6),expression=b.get_word(a+8),hooks=sec(a+10)) for a in rows('Transitions',16)]
        data['behaviours']=[dict(name=text(a),parent=b.get_word(a+4),attributes=sec(a+6),parameters=sec(a+10)) for a in rows('Behaviours',16)]
        data['conditions']=[dict(name=text(a),attributes=sec(a+4)) for a in rows('Conditions',8)]
        data['hooks']=[dict(name=text(a),attributes=sec(a+4)) for a in rows('Hooks',8)]
        data['parameters']=[b.get_dword(a) for a in rows('Params',4)]
        result[family]=data
    Path(destination).write_text(json.dumps(result,indent=2),encoding='utf-8')
    return {name:{k:len(d[k]) for k in ('states','behaviours','conditions')} for name,d in result.items()}
