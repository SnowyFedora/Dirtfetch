import re,urllib.request
s=urllib.request.urlopen('https://raw.githubusercontent.com/dylanaraps/neofetch/master/neofetch').read().decode('utf-8','ignore')
HEX=['#000000','#cd0000','#00cd00','#cdcd00','#0000cd','#cd00cd','#00cdcd','#e5e5e5','#7f7f7f','#ff0000','#00ff00','#ffff00','#5c5cff','#ff00ff','#00ffff','#ffffff']
names=[];cols=None;art=None;n=0
def flush():
    global n
    if art and names and cols:
        if art[0].startswith('"'): art[0]=art[0][1:]
        if art[-1].endswith('"'): art[-1]=art[-1][:-1]
        body='#colors='+cols[0]+','+cols[1]+'\n'+'\n'.join(art)+'\n'
        for nm in names:
            k=re.sub(r'[^a-z0-9_]+','_',nm.lower()).strip('_')
            if k:
                open('logos/'+k+'.txt','w').write(body);n+=1
for line in s.splitlines():
    st=line.strip()
    if art is not None:
        if st.startswith(';;'):
            flush();art=None;names=[];cols=None
        else:
            art.append(line.replace('\\`','`').replace('\\"','"'))
        continue
    if st.endswith(')') and '"' in st and not st.startswith('#') and 'ascii_data' not in st:
        names=re.findall(r'"([^"]+)"',st)
        continue
    m=re.match(r'set_colors\s+([0-9 ]+)',st)
    if m:
        cs=[HEX[int(x)] for x in m.group(1).split() if int(x)<16]
        if not cs: cs=['#e5e5e5']
        while len(cs)<2: cs.append(cs[0])
        cols=cs
        continue
    if "read -rd '' ascii_data" in line:
        art=[]
        continue
flush()
print('written',n,'marker logos into logos/')
