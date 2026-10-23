#!/usr/bin/env python3
"""Real Linux/Unix PTY acceptance for both release examples; stdlib only."""
import fcntl,os,pty,re,select,signal,struct,subprocess,termios,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
class Game:
    def __init__(self,name):
        self.master,self.slave=pty.openpty()
        fcntl.ioctl(self.slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0))
        self.original=termios.tcgetattr(self.slave)
        def connect():
            os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
        self.process=subprocess.Popen([str(ROOT/'target/release/examples'/name)],stdin=self.slave,stdout=self.slave,stderr=self.slave,preexec_fn=connect,env={**os.environ,'TERM':'xterm-256color'})
        self.output=bytearray()
    def read(self,duration=.15):
        result=bytearray();until=time.monotonic()+duration
        while time.monotonic()<until:
            if select.select([self.master],[],[],max(0,until-time.monotonic()))[0]:
                result.extend(os.read(self.master,65536))
        self.output.extend(result);return bytes(result)
    def send(self,keys):
        os.write(self.master,keys);return self.read()
    def resize(self,w,h):
        fcntl.ioctl(self.slave,termios.TIOCSWINSZ,struct.pack('HHHH',h,w,0,0));os.kill(self.process.pid,signal.SIGWINCH);return self.read()
    def idle(self):
        stat=Path(f'/proc/{self.process.pid}/stat')
        def ticks():
            f=stat.read_text().split();return int(f[13])+int(f[14])
        before=ticks() if stat.exists() else None
        assert not self.read(.6),'unsolicited paused output'
        if before is not None:
            delta=ticks()-before;assert delta<=1,f'busy pause: {delta} CPU ticks'
            print(f'  paused CPU: {delta} ticks/600ms',flush=True)
    def finish(self,key):
        self.send(key);assert self.process.wait(timeout=3)==0
        assert termios.tcgetattr(self.slave)==self.original,'termios not restored'
        assert b'\x1b[?1049l' in self.output,'alternate screen not restored'
        assert b'\x1b[?25h' in self.output,'cursor not restored'
    def close(self):
        if self.process.poll() is None:self.process.kill();self.process.wait()
        os.close(self.master);os.close(self.slave)
def mix(n):
    mask=(1<<64)-1;n=((n^(n>>30))*0xbf58476d1ce4e5b9)&mask;n=((n^(n>>27))*0x94d049bb133111eb)&mask;return n^(n>>31)
def snake(key):
    g=Game('terrasnake')
    try:
        assert b'TerraSnake' in g.read(.3)
        assert b'Score 1' in g.read(.65),'food not consumed by normal ticks'
        assert b'PAUSED' in g.send(b'p');g.idle()
        assert b'Need 40x21' in g.resize(20,6)
        assert b'PAUSED' in g.resize(100,30);g.idle()
        assert b'RUNNING' in g.send(b'n')
        assert b'GAME OVER' in g.read(6.5),'wall collision did not stop game'
        g.idle();assert b'Score 0' in g.send(b'n')
        g.send(b'w');g.finish(key)
        print(f'PASS: TerraSnake scoring, input, collision, pause, restart, resize, exit {key!r}, restoration',flush=True)
    finally:g.close()
def sandbox(key):
    g=Game('sandbox')
    try:
        assert b'TerraGame Sandbox' in g.read(.3)
        assert b'Tick' in g.read(.5),'simulation not advancing'
        # Deterministic resource in origin region: ordinary controls still perform
        # movement and gathering; no test hook changes the running simulation.
        h=mix(75^mix(0)^mix(0x9e3779b97f4a7c15));x=h%16;y=(h>>8)%16
        g.send(b'd'*x+b's'*y)
        result=g.send(b'e');assert re.search(rb'Gathered [1-9]',result),'resource interaction failed'
        g.send(b'a'*x+b'w'*y)
        result=g.send(b'd'*17);assert b'Active 9' in result and b'Dormant 3' in result,'region transition absent'
        assert b'PAUSED' in g.send(b'p');g.idle()
        g.resize(10,4);assert b'PAUSED' in g.resize(100,30);g.idle()
        assert b'RUNNING' in g.send(b'p');g.read(1.1)
        g.finish(key)
        print(f'PASS: sandbox movement, multirate stats, gathering, regions, pause, resize, exit {key!r}, restoration',flush=True)
    finally:g.close()
for key in [b'q',b'\x1b',b'\x03']:
    snake(key);sandbox(key)
