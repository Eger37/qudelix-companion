import struct,sys
d=open(sys.argv[1],'rb').read()
assert d[:8]==b'btsnoop\0'
off=16; recs=[]
while off+24<=len(d):
    ol,il,fl,dr,ts=struct.unpack('>IIIIq',d[off:off+24]); off+=24
    recs.append((fl,ts,d[off:off+il])); off+=il
t0=recs[0][1]
psm={}   # (handle,cid)->psm
pend={}  # scid -> psm (outgoing req, local scid)
frag={}
ctl={0x2f:'SABM',0x3f:'SABM',0x63:'UA',0x73:'UA',0x0f:'DM',0x1f:'DM',0x43:'DISC',0x53:'DISC',0xef:'UIH',0xff:'UIH'}
def handle_l2(h,direction,l2,ts):
    if len(l2)<4: return
    ln,cid=struct.unpack('<HH',l2[:4]); p=l2[4:]
    if cid==1:
        i=0
        while i+4<=len(p):
            code,ident,sl=struct.unpack('<BBH',p[i:i+4]); body=p[i+4:i+4+sl]
            if code==2:  # conn req psm,scid
                ps,sc=struct.unpack('<HH',body[:4]); pend[(h,direction,ident)]=(ps,sc)
                print(f"{(ts-t0)/1e6:9.3f} {direction} L2CAP ConnReq psm={ps:#x} scid={sc:#x}")
            elif code==3:
                dc,sc,res,st=struct.unpack('<HHHH',body[:8])
                print(f"{(ts-t0)/1e6:9.3f} {direction} L2CAP ConnRsp dcid={dc:#x} scid={sc:#x} res={res}")
                for k,(ps,s) in list(pend.items()):
                    if k[0]==h and k[2]==ident and ps==3:
                        psm[(h,s)]=3; psm[(h,dc)]=3
            elif code in (6,7):
                dc,sc=struct.unpack('<HH',body[:4])
                print(f"{(ts-t0)/1e6:9.3f} {direction} L2CAP Disc{'Req' if code==6 else 'Rsp'} dcid={dc:#x} scid={sc:#x}")
            i+=4+sl
        return
    if psm.get((h,cid))!=3 or len(p)<3: return
    addr=p[0]; c=p[1]; dlci=addr>>2
    if p[2]&1: L=p[2]>>1; hl=3
    else: L=(p[2]>>1)|(p[3]<<7); hl=4
    if (c&0xEF)==0xEF and (c&0x10): hl+=1  # credit byte
    info=p[hl:hl+L]
    name=ctl.get(c,hex(c))
    if dlci==3 or name!='UIH' or dlci==0:
        print(f"{(ts-t0)/1e6:9.3f} {direction} RFCOMM dlci={dlci} {name} {info.hex(' ')[:90]}")
for fl,ts,pk in recs:
    if not pk or pk[0]!=2: 
        if pk and pk[0]==4 and pk[1] in (0x03,0x05):  # conn complete / disconn complete
            print(f"{(ts-t0)/1e6:9.3f} HCI evt {pk[1]:#x} {pk[3:10].hex(' ')}")
        continue
    hf,ln=struct.unpack('<HH',pk[1:5]); h=hf&0xfff; pb=(hf>>12)&3; data=pk[5:5+ln]
    direction='RX' if fl&1 else 'TX'
    key=(h,direction)
    if pb in (0,2):
        frag[key]=bytearray(data)
    else:
        frag.setdefault(key,bytearray()).extend(data)
    buf=frag[key]
    if len(buf)>=4 and len(buf)>=4+struct.unpack('<H',buf[:2])[0]:
        handle_l2(h,direction,bytes(buf),ts); frag[key]=bytearray()
