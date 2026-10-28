"""Read-only APT timeline/ActionScript audit, not a game runtime.

Usage: python tools/inspect_apt.py path/to/movie.apt [--scripts]
Requires adjacent .const and optional .geo. Original assets stay external.
Operand layouts: native _parseStream829f6bc8; function-body skipping
82a10280/82a100a8. Opcode names: sGlobalTable82ff7800. This is APT's
authored UI bytecode, not the game's PPC executable.
"""
import argparse
import struct
from pathlib import Path

OPCODES = {"0":"End","4":"NextFrame","5":"PrevFrame","6":"Play","7":"Stop","10":"Add","11":"Subtract","12":"Multiply","13":"Divide","14":"Equals","15":"LessThan","16":"And","17":"Or","18":"Not","19":"StringEquals","20":"StringLength","21":"SubString","23":"Pop","24":"ToInteger","28":"GetVariable","29":"SetVariable","32":"SetTarget2","33":"StringAdd","34":"GetProperty","35":"SetProperty","36":"CloneSprite","37":"RemoveSprite","38":"Trace","39":"StartDragMovie","40":"StopDragMovie","42":"Throw","43":"CastOp","44":"ImplementsOp","48":"Random","51":"AsciiToChar","52":"GetTimer","58":"Delete","59":"Delete2","60":"DefineLocal","61":"CallFunction","62":"End","63":"Modulo","64":"NewObject","65":"DefineLocal2","66":"InitArray","67":"InitObject","68":"TypeOf","69":"TargetPath","70":"Enumerate","71":"Add2","72":"LessThan2","73":"Equals2","74":"ToNumber","75":"ToString","76":"PushDuplicate","77":"StackSwap","78":"GetMember","79":"SetMember","80":"Increment","81":"Decrement","82":"CallMethod","83":"NewMethod","84":"InstanceOf","85":"Enumerate","86":"PushThis","88":"PushGlobal","89":"Push0","90":"Push1","91":"CallFuncAndPop","92":"CallFuncSetVar","93":"CallMethodPop","94":"CallMethodSetVar","96":"BitAnd","97":"BitOr","98":"BitXor","99":"BitLShift","100":"BitRShift","102":"StrictEquals","103":"Greater","105":"Extends","112":"PushThisVariable","113":"PushGlobalVariable","114":"PushZeroSetVar","115":"PushTrue","116":"PushFalse","117":"PushUndefined","118":"PushUndefined","119":"TraceStart","129":"GotoFrame","131":"GetUrl","135":"StoreRegister","136":"DefineDictionary","139":"SetTarget","140":"GotoLabel","142":"DefineFunction2","143":"Try","148":"With","150":"Push","153":"BranchAlways","154":"GetUrl2","155":"DefineFunction","157":"BranchIfTrue","158":"CallFrame","159":"GotoFrame2","161":"PushString","162":"PushStringDictByte","163":"PushStringDictWord","164":"PushStringGetVar","165":"PushStringGetMember","166":"PushStringSetVar","167":"PushStringSetMember","174":"StringDictByteGetVar","175":"StringDictByteGetMember","176":"DictCallFuncPop","177":"DictCallFuncSetVar","178":"DictCallMethodPop","179":"DictCallMethodSetVar","180":"PushFloat","181":"PushByte","182":"PushWord","183":"PushDWord","184":"BranchIfFalse","185":"PushRegister"}


class Audit:
    def __init__(self, path):
        self.b = path.read_bytes()
        self.c = path.with_suffix('.const').read_bytes()
        self.root = self.u(20, self.c)
        self.constants = []
        table = self.u(28, self.c)
        for i in range(self.u(24, self.c)):
            kind, value = struct.unpack_from('>II', self.c, table + i*8)
            if kind == 1: value = self.s(value, self.c)
            elif kind == 6: value = struct.unpack('>f', struct.pack('>I', value))[0]
            elif kind == 7: value = value if value < 0x80000000 else value - 0x100000000
            else: value = (kind, value)
            self.constants.append(value)
        self.pool = []

    def u(self, o, b=None):
        return struct.unpack_from('>I', self.b if b is None else b, o)[0]

    def s(self, o, b=None):
        b = self.b if b is None else b
        return b[o:b.index(0,o)].decode('utf-8') if o else ''

    def script(self, start, end=None, indent=''):
        p = start
        while end is None or p < end:
            at = p; op = self.b[p]; p += 1
            name = OPCODES.get(str(op))
            if name is None: raise ValueError(f'unknown opcode {op} at {at:#x}')
            arg = ''; body = None
            if op in (129,135,153,157,159,184,139,140,161,164,165,166,167,148):
                p = (p+3)&~3; value = self.u(p); p += 4
                if op in (139,140,161,164,165,166,167): arg = repr(self.s(value))
                elif op in (153,157,184): arg = hex(p + (value if value < 0x80000000 else value-0x100000000))
                else: arg = str(value)
            elif op in (136,150):
                p = (p+3)&~3; count=self.u(p); table=self.u(p+4);p+=8
                values=[self.constants[self.u(table+4*i)] for i in range(count)]
                if op == 136: self.pool = values
                arg = repr(values)
            elif op in (142,155):
                p=(p+3)&~3; fname=self.s(self.u(p));count=self.u(p+4)
                if op == 142:
                    flags=self.u(p+8);params=self.u(p+12);length=self.u(p+16)
                    parameters=[(self.u(params+8*i),self.s(self.u(params+8*i+4))) for i in range(count)]
                    p+=28
                else:
                    flags=0;params=self.u(p+8);length=self.u(p+12)
                    parameters=[self.s(self.u(params+4*i)) for i in range(count)];p+=24
                arg=f'{fname!r} params={parameters} flags={flags:#x} bytes={length}'
                body=(p,p+length);p+=length
            elif op in (162,174,175,176,177,178,179):
                value=self.b[p];p+=1;arg=repr(self.pool[value])
            elif op == 163:
                value=int.from_bytes(self.b[p:p+2],'big');p+=2;arg=repr(self.pool[value])
            elif op in (181,182,183):
                size={181:1,182:2,183:4}[op];arg=str(int.from_bytes(self.b[p:p+size],'big',signed=True));p+=size
            elif op == 185: arg=str(self.b[p]);p+=1
            elif op in (119,180):
                arg=str(struct.unpack_from('>f' if op==180 else '>I',self.b,p)[0]);p+=4
            elif op in (131,143): raise ValueError(f'unimplemented audit operand {name} at {at:#x}')
            print(f'{indent}{at:06x} {name} {arg}')
            if body:
                pool=self.pool.copy();self.script(*body,indent=indent+'  ');self.pool=pool
            if op == 0: break

    def dump(self, scripts):
        root=self.root;count=self.u(root+28);table=self.u(root+32)
        print('stage',self.u(root+36),self.u(root+40),'frame_ms',self.u(root+44),'characters',count)
        for i in range(count):
            o=self.u(table+4*i)
            if not o: print('character',i,'import');continue
            kind=self.u(o);print('character',i,'type',kind)
            if kind not in (5,9): continue
            frames=self.u(o+20)
            for j in range(self.u(o+16)):
                nc=self.u(frames+j*8);ct=self.u(frames+j*8+4)
                for k in range(nc):
                    c=self.u(ct+k*4);ck=self.u(c)
                    if ck==2: print(' frame',j,'label',self.s(self.u(c+4)))
                    elif ck in (3,9):
                        print(' frame',j,'place',hex(self.u(c+4)),'depth',self.u(c+8),'id',self.u(c+12),'name',repr(self.s(self.u(c+52))),'matrix',struct.unpack_from('>6f',self.b,c+16),'colour',hex(self.u(c+40)),hex(self.u(c+44)))
                        ev=self.u(c+60)
                        if scripts and ev:
                            for e in range(self.u(ev)):
                                eo=self.u(ev+4)+e*12;print('  event',hex(self.u(eo)),self.u(eo+4));self.script(self.u(eo+8),indent='    ')
                    elif ck in (1,8):
                        action=self.u(c+(4 if ck==1 else 8))
                        if ck==8:
                            print(' frame',j,'init_action character',self.u(c+4),'offset',hex(action))
                        else:
                            print(' frame',j,'action',hex(action))
                        if scripts:self.script(action,indent='  ')
                    elif ck==4:print(' frame',j,'remove',self.u(c+4))
                    else:print(' frame',j,'control',ck)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('movie',type=Path)
    parser.add_argument('--scripts',action='store_true')
    args=parser.parse_args()
    Audit(args.movie).dump(args.scripts)
