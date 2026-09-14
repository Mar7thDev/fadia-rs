# Checks fadia-rs's hand-written replication layouts against a fresh reflection
# dump from nte-dumper, matching each field/RPC to the client by name and
# reporting where the number has drifted.
#
#   python tools/diff_dump.py [<nte-dumper out dir>]
#
# Default out dir: ../nte-dumper/out relative to this repo.
import re,json,os,sys
_here=os.path.dirname(os.path.abspath(__file__))
LAYOUT=os.path.join(_here,"..","servers","game-server","src","logic","layout")
OUT=sys.argv[1] if len(sys.argv)>1 else os.path.join(_here,"..","..","nte-dumper","out")
# fadia struct name -> UE class name
MAP={
 'PlayerState':'AHTPlayerState',
 'HTPlayerCharacter':'AHTPlayerCharacter',
 'PlayerControllerBase':'AHTPlayerController',
 'HTAttributeSet':'UGameplayAttributeSet',
 'AbilitySystemComponent':'UHTAbilitySystemComponent',
 'WeaponBase':'AWeaponBase',
 'WorldDataLayers':'AWorldDataLayers',
 # Blueprint-defined: the replicated properties live in the .uasset, not in the
 # exe's reflection data, so they cannot be checked statically.
 'UltraDynamicWeather':None,
 # FGameplayAbilitySpec is a struct, not a class: check it with
 #   nte-dumper struct-layout GameplayAbilitySpec
 'GameplayAbilitySpec':None,
 'StateManagerComponent':'UHTCharacterStateManagerComponent',
}
def snake_to_camel(s):
    return ''.join(p[:1].upper()+p[1:] for p in s.split('_'))
def parse():
    out={}
    for fn in os.listdir(LAYOUT):
        txt=open(os.path.join(LAYOUT,fn),encoding='utf-8').read()
        for m in re.finditer(r'#\[derive\(Debug, RepLayout\)\]((?:.|\n)*?)pub struct (\w+)\s*\{(.*?)\}',txt,re.S):
            attrs,name,body=m.group(1),m.group(2),m.group(3)
            maxidx=re.search(r'max_rep_index\((\d+)\)',attrs)
            reps=[]
            for mm in re.finditer(r'#\[rep\(handle = (\d+)\)\]\s*\n\s*pub (\w+):',body):
                reps.append((int(mm.group(1)),mm.group(2)))
            out.setdefault(name,{}).update(dict(reps=reps,max_rep_index=int(maxidx.group(1)) if maxidx else None,file=fn))
        for m in re.finditer(r'#\[rpc_handlers\]\s*\nimpl (\w+)',txt):
            pass
    # rpcs: scan whole files
    for fn in os.listdir(LAYOUT):
        txt=open(os.path.join(LAYOUT,fn),encoding='utf-8').read()
        cur=None
        for line in txt.split('\n'):
            m=re.match(r'\s*impl (\w+)',line)
            if m: cur=m.group(1)
            m=re.match(r'\s*#\[rpc\((\d+),\s*(\w+)\)\]',line)
            if m and cur:
                out.setdefault(cur,{}).setdefault('rpcs',[]).append((int(m.group(1)),m.group(2)))
        # attach names on next fn line
    for fn in os.listdir(LAYOUT):
        txt=open(os.path.join(LAYOUT,fn),encoding='utf-8').read()
        cur=None
        acc={}
        lines=txt.split('\n')
        for i,line in enumerate(lines):
            m=re.match(r'\s*impl (\w+)',line)
            if m: cur=m.group(1)
            m=re.match(r'\s*#\[rpc\((\d+),\s*(\w+)\)\]',line)
            if m and cur:
                nm=None
                for j in range(i+1,min(i+4,len(lines))):
                    mm=re.search(r'fn (\w+)',lines[j])
                    if mm: nm=mm.group(1); break
                acc.setdefault(cur,[]).append((int(m.group(1)),m.group(2),nm))
        for k,v in acc.items(): out.setdefault(k,{})['rpcs_named']=v
    return out
fadia=parse()
layouts={l['class']:l for l in json.load(open(os.path.join(OUT,'rep_layouts.json'),encoding='utf-8'))}
caches={c['class']:c for c in json.load(open(os.path.join(OUT,'net_cache.json'),encoding='utf-8'))}
for name,info in sorted(fadia.items()):
    ue=MAP.get(name)
    print(f"\n=== fadia {name}  ->  {ue or '??? (unmapped)'}")
    if ue is None:
        print("   not checked here: Blueprint-defined, or a struct - use `nte-dumper struct-layout <Name>`")
        continue
    if ue not in layouts:
        print(f"   client class {ue} not found in dump")
        continue
    L=layouts[ue]; C=caches.get(ue)
    # fadia's #[max_rep_index(N)] is the *inclusive* top of the field index
    # space, and is passed to SerializeInt as N+1; UE uses the exclusive bound
    # FClassNetCache::GetMaxIndex(). So the attribute must be GetMaxIndex() - 1.
    if info.get('max_rep_index') is not None and C:
        want=C['max_index']-1
        print(f"   max_rep_index: fadia {info['max_rep_index']} | expected {want}"
              + ("  OK" if info['max_rep_index']==want else "  MISMATCH"))
    # Leaf names win: a struct property that expands covers several handles, and
    # the fadia field corresponds to the leaf, not to the struct.
    byh={}
    for e in L['entries']:
        for lf in e['leaves']:
            parts=lf['path'].split('.')
            leaf=parts[-1]
            parent=parts[-2] if len(parts)>1 else leaf
            # A struct like FGameplayAttributeData shows up as Parent.BaseValue /
            # Parent.CurrentValue; the meaningful name is the parent plus the
            # Base/Cur suffix, which is how fadia names its fields.
            alts={leaf,parent,parent+leaf}
            low=parent.lower()
            if low.endswith('base'):
                stem=parent[:-4]
                alts |= {stem+'Cur',stem+'Current',stem+'Base'}
            alts |= {parent+'Base',parent+'Cur',parent+'Current'}
            byh[lf['handle']]={'name':leaf,'alts':alts}
    for e in L['entries']:
        byh.setdefault(e['handle'],e)
    for h,field in info.get('reps',[]):
        e=byh.get(h)
        cn=snake_to_camel(field).replace('Id','ID')
        got=e['name'] if e else '-'
        target=cn.lower().replace('_','')
        names={got} | set(e.get('alts',())) if e else set()
        ok = any(n.lower().replace('_','').lstrip('m') == target
                 or target in n.lower().replace('_','')
                 or n.lower().replace('_','') in target
                 for n in names)
        print(f"   rep {h:>4} {field:<38} client={got:<38}{'' if ok else '   <-- MISMATCH'}")
    if C:
        byi={f['index']:f for f in C['fields']}
        for idx,d,nm in info.get('rpcs_named',[]):
            f=byi.get(idx)
            # fadia prefixes handler names with on_, and drops the client's
            # underscore separators.
            base=(nm or '')
            if base.startswith('on_'): base=base[3:]
            cn=snake_to_camel(base).lower()
            got=f['name'] if f else '-'
            ok = f and f.get('is_function') and got.lower().replace('_','')==cn.replace('_','')
            print(f"   rpc {idx:>4} {(nm or '?'):<38} client={got:<38}{'' if ok else '   <-- MISMATCH'}")
