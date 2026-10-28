"""Read native pedestrian declarations in IDA; write an evidence JSON only."""
import json
from pathlib import Path
import ida_bytes as b


def export(destination):
    def text(address):
        value = b.get_strlit_contents(b.get_dword(address), -1, 0)
        return value.decode('utf-8') if value else ''

    def section(address):
        return {'count': b.get_word(address), 'index': b.get_word(address + 2)}

    data = {'source': 'Skate2 debug HCDelegate_Pedestrian native declarations',
            'addresses': {'states': 0x821d25d0, 'transitions': 0x821d3690,
                          'behaviours': 0x821d4b00, 'conditions': 0x821d6078,
                          'attributes': 0x821d1798, 'indices': 0x821d1c08}}
    data['attributes'] = [dict(key=text(a), value=text(a+4))
                          for a in range(0x821d1798, 0x821d1798+142*8, 8)]
    data['indices'] = [b.get_word(a) for a in range(0x821d1c08, 0x821d1c08+59*2, 2)]
    data['states'] = [dict(name=text(a), interrupt_type=b.get_byte(a+4),
        active=b.get_byte(a+5) != 0, interrupt_target=b.get_word(a+6),
        parent=b.get_word(a+8), expression=b.get_word(a+10), children=section(a+12),
        behaviours=section(a+16), transitions=section(a+20),
        update_address=b.get_dword(a+24), trailing=b.get_dword(a+28))
        for a in range(0x821d25d0, 0x821d25d0+134*32, 32)]
    data['transitions'] = [dict(name=text(a), priority=b.get_byte(a+4),
        target=b.get_word(a+6), expression=b.get_word(a+8), hooks=section(a+10))
        for a in range(0x821d3690, 0x821d3690+59*16, 16)]
    data['behaviours'] = [dict(name=text(a), parent=b.get_word(a+4),
        attributes=section(a+6), parameters=section(a+10))
        for a in range(0x821d4b00, 0x821d4b00+201*16, 16)]
    data['conditions'] = [dict(name=text(a), attributes=section(a+4))
        for a in range(0x821d6078, 0x821d6078+89*8, 8)]
    Path(destination).write_text(json.dumps(data, indent=2), encoding='utf-8')
    return {key: len(data[key]) for key in ('states','transitions','behaviours','conditions')}
