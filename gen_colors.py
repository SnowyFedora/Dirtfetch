import re,urllib.request,string
NORM={'BLACK':'#000000','RED':'#cd0000','GREEN':'#00cd00','YELLOW':'#cdcd00','BLUE':'#0000cd','MAGENTA':'#cd00cd','CYAN':'#00cdcd','WHITE':'#e5e5e5'}
BRIGHT={'BLACK':'#7f7f7f','RED':'#ff0000','GREEN':'#00ff00','YELLOW':'#ffff00','BLUE':'#5c5cff','MAGENTA':'#ff00ff','CYAN':'#00ffff','WHITE':'#ffffff'}
def col(tok):
    tok=tok.strip().strip('"')
    if tok.isdigit():
        return (list(NORM.values())+list(BRIGHT.values()))[int(tok)&15]
    t=tok.replace('FF_COLOR_FG_','').upper()
    if t.startswith('BRIGHT_'): return BRIGHT.get(t[7:],'#ffffff')
    return NORM.get(t,'#e5e5e5')
out={}
base='https://raw.githubusercontent.com/fastfetch-cli/fastfetch/dev/src/logo/ascii/'
for ch in string.ascii_lowercase:
    try:
        s=urllib.request.urlopen(base+ch+'.inc').read().decode('utf-8','ignore')
    except Exception as e:
        print('skip',ch); continue
    for m in re.finditer(r'\.names\s*=\s*\{(.*?)\}.*?\.colors\s*=\s*\{(.*?)\}',s,re.S):
        names=re.findall(r'"([^"]+)"',m.group(1))
        cols=[col(t) for t in re.findall(r'[\w"]+',m.group(2))]
        if not cols: cols=['#e5e5e5']
        cols=(cols*2)[:2]
        for n in names:
            k=n.lower().strip()
            if k and k not in out: out[k]=cols
open('src/distro_colors.txt','w').write(''.join(f'{k}: {v[0]} {v[1]}\n' for k,v in sorted(out.items())))
print('fastfetch colors:',len(out))
