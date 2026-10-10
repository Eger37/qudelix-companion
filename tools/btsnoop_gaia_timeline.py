import struct,sys,datetime
d=open(sys.argv[1],'rb').read(); off=16
EPOCH=datetime.datetime(1,1,1)  # btsnoop ts: microseconds since 0000-01-01 (0x00E03AB44A676000 offset)
def wall(ts):
    us=ts-0x00dcddb30f2f8000  # to unix epoch
    return (datetime.datetime.utcfromtimestamp(us/1e6)+datetime.timedelta(hours=3)).strftime('%H:%M:%S.%f')[:-3]
psm={}; pend={}; frag={}
ctl={0x2f:'SABM',0x3f:'SABM',0x63:'UA',0x73:'UA',0x0f:'DM',0x1f:'DM',0x43:'DISC',0x53:'DISC',0xef:'UIH',0xff:'UIH'}
out=[]
def l2(h,dr,b,ts):
    ln,cid=struct.unpack('<HH',b[:4]); p=b[4:]
    if cid==1:
        i=0
        while i+4<=len(p):
            code,ident,sl=struct.unpack('<BBH',p[i:i+4]); body=p[i+4:i+4+sl]
            if code==2:
                ps,sc=struct.unpack('<HH',body[:4]); pend[(h,ident)]=(ps,sc)
                if ps in (1,3): out.append(f"{wall(ts)} h{h:x} {dr} L2CAP ConnReq psm={ps}")
            elif code==3:
                dc,sc,res,st=struct.unpack('<HHHH',body[:8])
                k=(h,ident)
                if k in pend and res==0:
                    ps,s=pend[k]; psm[(h,s)]=ps; psm[(h,dc)]=ps
            elif code==6:
                dc,sc=struct.unpack('<HH',body[:4])
                if psm.get((h,dc)) in (1,3) or psm.get((h,sc)) in (1,3): out.append(f"{wall(ts)} h{h:x} {dr} L2CAP DiscReq psm={psm.get((h,dc)) or psm.get((h,sc))}")
            i+=4+sl
        return
    kind=psm.get((h,cid))
    if kind==1:
        out.append(f"{wall(ts)} h{h:x} {dr} SDP {p.hex(' ')[:150]}")
    elif kind==3 and len(p)>=3:
        a=p[0]; c=p[1]; dlci=a>>2
        hl=3 if p[2]&1 else 4
        if (c&0xEF)==0xEF and (c&0x10): hl+=1
        L=(p[2]>>1) if p[2]&1 else ((p[2]>>1)|(p[3]<<7))
        info=p[hl:hl+L]; name=ctl.get(c,hex(c))
        if name!='UIH' or (info[:2]==b'\xff\x01'):
            if not (info[4:10]==bytes.fromhex('f001a0000004')):   # skip 5 s heartbeat
                out.append(f"{wall(ts)} h{h:x} {dr} RFCOMM dlci={dlci} {name} {info.hex(' ')[:100]}")
while off+24<=len(d):
    ol,il,fl,dr_,ts=struct.unpack('>IIIIq',d[off:off+24]); off+=24; pk=d[off:off+il]; off+=il
    if not pk: continue
    if pk[0]==4 and len(pk)>3 and pk[1] in (0x03,0x05):
        out.append(f"{wall(ts)} HCI {'ConnComplete' if pk[1]==3 else 'DisconnComplete'} {pk[3:12].hex(' ')}")
    if pk[0]!=2: continue
    hf,ln=struct.unpack('<HH',pk[1:5]); h=hf&0xfff; pb=(hf>>12)&3; data=pk[5:5+ln]; dr='RX' if fl&1 else 'TX'
    k=(h,dr)
    if pb in (0,2): frag[k]=bytearray(data)
    else: frag.setdefault(k,bytearray()).extend(data)
    b=frag[k]
    if len(b)>=4 and len(b)>=4+struct.unpack('<H',b[:2])[0]:
        try: l2(h,dr,bytes(b),ts)
        except Exception: pass
        frag[k]=bytearray()
print("\n".join(out))
