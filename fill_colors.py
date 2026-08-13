import os,hashlib
def hh(s):
    return '#'+hashlib.md5(s.encode()).hexdigest()[:6]
seen=set()
for l in open('src/distro_colors.txt'):
    seen.add(l.split(':')[0].strip().lower())
root=os.path.expanduser('~/.config/dirtfetch/logos')
add=[]
for dp,_,fs in os.walk(root):
    for f in fs:
        if f.endswith('.txt'):
            k=f[:-4].lower()
            if k not in seen:
                add.append(f'{k}: {hh(k)} {hh(k+"x")}\n')
open('src/distro_colors.txt','a').write(''.join(sorted(add)))
print('added for all remaining logos:',len(add))
